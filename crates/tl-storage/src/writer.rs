//! Background trace writer.
//!
//! The hot path (`POST /v1/events`) returns the decision before
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

use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{Decision, GuardEvent};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use crate::models::NewTrace;
use crate::postgres::DbPool;
use crate::schema::{runs, traces};
use crate::StorageError;

/// One queued write. The hot path constructs this with the agent's
/// inferred domain and the engine's `Decision`.
#[derive(Clone, Debug)]
pub struct TraceWrite {
    pub decision: Decision,
    /// Normalized event evidence for this decision. `None` for writers
    /// that predate event collection; the payload stays a bare `Decision`.
    pub event: Option<GuardEvent>,
    pub workspace_id: String,
    pub environment_id: String,
    /// Direct registered agent attribution; never inferred from payload JSON.
    pub agent_id: String,
    pub run_id: Option<String>,
    pub run_event_id: Option<String>,
    /// Monitoring session id, passed through verbatim (opaque string,
    /// never parsed). Promoted to the `session_id` trace column so the
    /// trace API can filter per session.
    pub session_id: Option<String>,
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
            received = rx.recv(), if buf.len() < config.batch_size => match received {
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
                        if rx.is_closed() {
                            tracing::error!(
                                buffered_traces = buf.len(),
                                "trace writer stopped with unflushed evidence after channel closure"
                            );
                            return;
                        }
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

    // Keep the original writes buffered until the transaction commits. A
    // transient database failure must not silently discard evaluation
    // evidence that already passed the bounded channel.
    let rows = buf.clone();
    let mut traces_to_insert: Vec<NewTrace> = rows.into_iter().map(trace_write_to_new).collect();

    let mut conn = pool
        .get()
        .await
        .map_err(|e| StorageError::Internal(format!("db pool: {e}")))?;

    let mut run_groups = std::collections::BTreeMap::<
        (String, String),
        std::collections::BTreeSet<uuid::Uuid>,
    >::new();
    for trace in &traces_to_insert {
        if let Some(run_id) = trace.run_id {
            run_groups
                .entry((trace.workspace_id.clone(), trace.environment_id.clone()))
                .or_default()
                .insert(run_id);
        }
    }

    let result = conn
        .transaction::<(), StorageError, _>(async |conn| {
            let evidence_at = chrono::Utc::now();
            for ((workspace_id, environment_id), run_ids) in &run_groups {
                let run_id_values = run_ids.iter().copied().collect::<Vec<_>>();
                let states = runs::table
                    .filter(runs::workspace_id.eq(workspace_id))
                    .filter(runs::environment_id.eq(environment_id))
                    .filter(runs::id.eq_any(&run_id_values))
                    .select((runs::id, runs::capture_status))
                    .order(runs::id.asc())
                    .for_update()
                    .load::<(uuid::Uuid, String)>(conn)
                    .await?;
                let closed_runs = states
                    .into_iter()
                    .filter_map(|(run_id, status)| {
                        matches!(status.as_str(), "complete" | "incomplete").then_some(run_id)
                    })
                    .collect::<std::collections::HashSet<_>>();
                for trace in &mut traces_to_insert {
                    if trace.workspace_id == *workspace_id
                        && trace.environment_id == *environment_id
                    {
                        trace.late_evidence = trace
                            .run_id
                            .is_some_and(|run_id| closed_runs.contains(&run_id));
                    }
                }
                diesel::update(
                    runs::table
                        .filter(runs::workspace_id.eq(workspace_id))
                        .filter(runs::environment_id.eq(environment_id))
                        .filter(runs::id.eq_any(&run_id_values)),
                )
                .set((
                    runs::last_evidence_at.eq(evidence_at),
                    runs::updated_at.eq(evidence_at),
                ))
                .execute(conn)
                .await?;
            }

            diesel::insert_into(traces::table)
                .values(&traces_to_insert)
                .on_conflict((traces::trace_id, traces::created_at))
                .do_nothing()
                .execute(conn)
                .await
                .map_err(|error| StorageError::Internal(format!("trace flush: {error}")))?;
            Ok(())
        })
        .await;
    if result.is_ok() {
        buf.clear();
    }
    result
}

/// Serialize the trace payload: the full `Decision`, plus an additive
/// `"event"` key when event evidence was collected. Existing consumers
/// parse the payload as a `Decision` and ignore unknown keys, so the
/// enrichment is backward compatible. Evidence serialization failure
/// degrades to the bare decision payload — enrichment must never cost
/// a trace row.
pub(crate) fn trace_write_to_new(w: TraceWrite) -> NewTrace {
    let trace_uuid =
        uuid::Uuid::parse_str(&w.decision.trace_id).unwrap_or_else(|_| uuid::Uuid::nil());
    let payload = build_trace_payload(&w.decision, w.event.as_ref());
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
        session_id: w.session_id,
        agent_id: w.agent_id,
        environment_id: w.environment_id,
        domain: w.domain,
        decision: effect_text(&w.decision.effect).to_string(),
        elapsed_ms: w.decision.latency_ms as i32,
        payload,
        late_evidence: false,
    }
}

fn build_trace_payload(decision: &Decision, event: Option<&GuardEvent>) -> serde_json::Value {
    let mut payload = serde_json::to_value(decision).unwrap_or(serde_json::Value::Null);
    if let (Some(event), Some(object)) = (event, payload.as_object_mut()) {
        match serde_json::to_value(event) {
            Ok(evidence) => {
                object.insert("event".into(), evidence);
            }
            Err(e) => {
                tracing::warn!(error = %e, "event evidence serialization failed; persisting bare decision payload");
            }
        }
    }
    payload
}

fn effect_text(v: &tl_core::AuthorizationEffect) -> &'static str {
    match v {
        tl_core::AuthorizationEffect::Permit => "permit",
        tl_core::AuthorizationEffect::Deny => "deny",
        tl_core::AuthorizationEffect::Transform => "transform",
        tl_core::AuthorizationEffect::RequireApproval => "require_approval",
        tl_core::AuthorizationEffect::Defer => "defer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{
        Action, AuthorizationEffect, EventKind, GuardEvent, Principal, ProvenanceMap,
        SideEffectClass,
    };

    fn event() -> GuardEvent {
        let mut provenance = ProvenanceMap::default();
        provenance.insert("text", vec!["legacy.input".into()]);

        GuardEvent {
            kind: EventKind::OutputProposed,
            principal: Principal {
                workspace_id: "ws_1".into(),
                environment_id: "production".into(),
                agent_id: "agent-1".into(),
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: Action {
                operation: "output".into(),
                parameters: serde_json::json!({ "text": "safe reply" }),
                side_effect: Some(SideEffectClass::None),
                invocation_id: None,
                tool_identity: None,
                authorization: None,
            },
            sources: vec![],
            provenance,
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::Value::Null,
        }
    }

    #[test]
    fn trace_payload_without_event_is_bare_decision() {
        let decision = Decision::allow("trace-1");

        let payload = build_trace_payload(&decision, None);

        assert!(payload.get("event").is_none());
        assert_eq!(payload, serde_json::to_value(&decision).unwrap());
    }

    #[test]
    fn trace_payload_with_event_carries_evidence() {
        let decision = Decision::allow("trace-1");

        let payload = build_trace_payload(&decision, Some(&event()));

        let evidence = payload.get("event").expect("event evidence attached");
        assert_eq!(evidence["kind"], "output.proposed");
        assert_eq!(evidence["principal"]["workspace_id"], "ws_1");
        assert_eq!(evidence["action"]["operation"], "output");
        assert_eq!(evidence["provenance"]["text"][0], "legacy.input");

        // The enriched payload must still parse as a Decision for every
        // existing payload consumer.
        let parsed: Decision = serde_json::from_value(payload).unwrap();
        assert_eq!(parsed.effect, AuthorizationEffect::Permit);
        assert_eq!(parsed.trace_id, "trace-1");
    }
}
