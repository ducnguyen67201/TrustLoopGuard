//! Background trace writer.
//!
//! The hot path (`POST /v1/check`) returns the decision before
//! persistence completes. It pushes a `TraceWrite` into an `mpsc`
//! channel; this module drains that channel into Postgres via
//! batched multi-row INSERTs.
//!
//! Two flush triggers:
//! - **Size**: when the in-memory buffer reaches `batch_size`.
//! - **Time**: every `flush_interval`, regardless of buffer fill.
//!
//! Backpressure: the channel is bounded. When full, the caller's
//! `try_send` returns `Full` and the trace is dropped with a tracing
//! warning. Better to lose a trace than block the request path.

use std::time::Duration;

use sqlx::postgres::PgPool;
use sqlx::types::Json;
use sqlx::QueryBuilder;
use tl_core::Decision;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use crate::StorageError;

/// One queued write. The hot path constructs this with the agent's
/// inferred domain and the engine's `Decision`.
#[derive(Clone, Debug)]
pub struct TraceWrite {
    pub decision: Decision,
    pub domain: String,
}

#[derive(Clone, Copy, Debug)]
pub struct WriterConfig {
    /// Channel capacity. Drops once full; sender uses `try_send` so
    /// the request path never blocks on a slow writer.
    pub buffer_size: usize,
    /// Flush when the in-memory buffer hits this size.
    pub batch_size: usize,
    /// Flush this often regardless of fill, so low-traffic deployments
    /// don't lose recent traces on shutdown.
    pub flush_interval: Duration,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            buffer_size: 4_096,
            batch_size: 50,
            flush_interval: Duration::from_millis(100),
        }
    }
}

/// Spawn the background writer. Returns the sender to plug into
/// `AppState` plus the join handle so the server can flush on
/// shutdown by dropping the sender and awaiting completion.
pub fn spawn_writer(
    pool: PgPool,
    config: WriterConfig,
) -> (mpsc::Sender<TraceWrite>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(config.buffer_size);
    let handle = tokio::spawn(writer_loop(pool, rx, config));
    (tx, handle)
}

async fn writer_loop(pool: PgPool, mut rx: mpsc::Receiver<TraceWrite>, config: WriterConfig) {
    let mut buf: Vec<TraceWrite> = Vec::with_capacity(config.batch_size);
    let mut tick = interval(config.flush_interval);
    // Default `Burst` would queue up missed ticks and fire them in
    // a row after a backlog drain — we want at-most-one tick per period.
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            // Drain the channel as quickly as possible, batching as we go.
            received = rx.recv() => match received {
                Some(t) => {
                    buf.push(t);
                    if buf.len() >= config.batch_size {
                        if let Err(e) = flush(&pool, &mut buf).await {
                            tracing::error!(error = %e, "trace writer flush failed");
                        }
                    }
                }
                None => {
                    // Channel closed — graceful shutdown. Flush whatever's
                    // left and exit.
                    if !buf.is_empty() {
                        if let Err(e) = flush(&pool, &mut buf).await {
                            tracing::error!(error = %e, "trace writer final flush failed");
                        }
                    }
                    return;
                }
            },
            _ = tick.tick() => {
                if !buf.is_empty() {
                    if let Err(e) = flush(&pool, &mut buf).await {
                        tracing::error!(error = %e, "trace writer interval flush failed");
                    }
                }
            }
        }
    }
}

async fn flush(pool: &PgPool, buf: &mut Vec<TraceWrite>) -> Result<(), StorageError> {
    if buf.is_empty() {
        return Ok(());
    }

    let rows = std::mem::take(buf);

    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        r#"INSERT INTO "Traces" (trace_id, domain, decision, elapsed_ms, payload) "#,
    );
    qb.push_values(rows.iter(), |mut row, w| {
        let trace_uuid = match uuid::Uuid::parse_str(&w.decision.trace_id) {
            Ok(u) => u,
            Err(_) => uuid::Uuid::nil(),
        };
        let payload = serde_json::to_value(&w.decision).unwrap_or(serde_json::Value::Null);
        row.push_bind(trace_uuid)
            .push_bind(&w.domain)
            .push_bind(verdict_text(&w.decision.verdict))
            .push_bind(w.decision.latency_ms as i32)
            .push_bind(Json(payload));
    });
    qb.push(" ON CONFLICT (trace_id, created_at) DO NOTHING");

    qb.build()
        .execute(pool)
        .await
        .map_err(|e| StorageError::Internal(format!("trace flush: {e}")))?;

    Ok(())
}

fn verdict_text(v: &tl_core::Verdict) -> &'static str {
    match v {
        tl_core::Verdict::Allow => "allow",
        tl_core::Verdict::Block => "block",
        tl_core::Verdict::Rewrite => "rewrite",
        tl_core::Verdict::Escalate => "escalate",
    }
}
