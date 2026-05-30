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

use diesel_async::RunQueryDsl;
use tl_core::Decision;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use crate::models::NewTrace;
use crate::postgres::DbPool;
use crate::schema::traces;
use crate::StorageError;

/// One queued write. The hot path constructs this with the agent's
/// inferred domain and the engine's `Decision`.
#[derive(Clone, Debug)]
pub struct TraceWrite {
    pub decision: Decision,
    pub workspace_id: String,
    pub environment_id: String,
    pub run_id: Option<String>,
    pub run_event_id: Option<String>,
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
    pool: DbPool,
    config: WriterConfig,
) -> (mpsc::Sender<TraceWrite>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(config.buffer_size);
    let handle = tokio::spawn(writer_loop(pool, rx, config));
    (tx, handle)
}

async fn writer_loop(pool: DbPool, mut rx: mpsc::Receiver<TraceWrite>, config: WriterConfig) {
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

async fn flush(pool: &DbPool, buf: &mut Vec<TraceWrite>) -> Result<(), StorageError> {
    if buf.is_empty() {
        return Ok(());
    }

    let rows = std::mem::take(buf);
    let traces: Vec<NewTrace> = rows
        .into_iter()
        .map(|w| {
            let trace_uuid =
                uuid::Uuid::parse_str(&w.decision.trace_id).unwrap_or_else(|_| uuid::Uuid::nil());
            let payload = serde_json::to_value(&w.decision).unwrap_or(serde_json::Value::Null);
            NewTrace {
                workspace_id: w.workspace_id,
                trace_id: trace_uuid,
                run_id: w
                    .run_id
                    .as_deref()
                    .and_then(|id| uuid::Uuid::parse_str(id).ok()),
                run_event_id: w
                    .run_event_id
                    .as_deref()
                    .and_then(|id| uuid::Uuid::parse_str(id).ok()),
                environment_id: w.environment_id,
                domain: w.domain,
                decision: verdict_text(&w.decision.verdict).to_string(),
                elapsed_ms: w.decision.latency_ms as i32,
                payload,
            }
        })
        .collect();

    let mut conn = pool
        .get()
        .await
        .map_err(|e| StorageError::Internal(format!("db pool: {e}")))?;

    diesel::insert_into(traces::table)
        .values(&traces)
        .on_conflict((traces::trace_id, traces::created_at))
        .do_nothing()
        .execute(&mut conn)
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
