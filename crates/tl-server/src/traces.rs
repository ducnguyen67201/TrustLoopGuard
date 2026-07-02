//! Dashboard trace read endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode, TraceListResponse, TraceSummary};

use crate::environments::EnvironmentStore;

#[derive(Debug, thiserror::Error)]
pub enum TraceStoreError {
    #[error("internal: {0}")]
    Internal(String),
}

/// Feature-independent trace write. Every decision path (events, gateway,
/// pay) records through this one seam: the postgres adapter converts it to
/// the batched writer's `TraceWrite`; the memory store keeps it directly so
/// dev mode and tests see the same trace history the SQL paths do.
#[derive(Debug, Clone)]
pub struct TraceWriteRequest {
    pub workspace_id: String,
    pub environment_id: String,
    pub decision: tl_core::Decision,
    pub event: Option<tl_core::GuardEvent>,
    pub run_id: Option<String>,
    pub run_event_id: Option<String>,
    pub session_id: Option<String>,
    pub domain: String,
}

#[async_trait]
pub trait TraceStore: Send + Sync {
    async fn list_recent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TraceSummary>, TraceStoreError>;

    /// Sum prior counted spend for an owner since `since`: allowed payment
    /// events whose operation is in `operations`. Backs the daily/monthly
    /// payment caps without a dedicated spend table.
    async fn sum_payment_minor_since(
        &self,
        workspace_id: &str,
        owner: &str,
        operations: &[String],
        since: DateTime<Utc>,
    ) -> Result<i64, TraceStoreError>;

    /// Record a decision trace. Best-effort on the postgres path (batched
    /// channel, same as before); synchronous on the memory path.
    async fn record(&self, write: TraceWriteRequest) -> Result<(), TraceStoreError>;

    /// Fetch a single trace by id (not window-bounded, unlike `list_recent`).
    /// Resolves a held payment for execution even after it has aged out of the
    /// recent window. Default `None` for stores without point lookup (the
    /// channel test double).
    async fn get(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _trace_id: &str,
    ) -> Result<Option<TraceSummary>, TraceStoreError> {
        Ok(None)
    }
}

fn verdict_text(v: tl_core::Verdict) -> &'static str {
    match v {
        tl_core::Verdict::Allow => "allow",
        tl_core::Verdict::Block => "block",
        tl_core::Verdict::Rewrite => "rewrite",
        tl_core::Verdict::Escalate => "escalate",
    }
}

#[derive(Debug, Clone)]
struct StoredTrace {
    workspace_id: String,
    summary: TraceSummary,
    created_at: DateTime<Utc>,
}

/// Upper bound on retained traces in the in-memory (dev/test) store, so it
/// can't grow without limit. Generous enough to keep a day of dev traffic and
/// resolve recent holds; production uses the Postgres path.
const MEMORY_TRACE_CAP: usize = 50_000;

/// In-memory trace store with real accumulation, mirroring the SQL
/// semantics of `tl_storage::TraceRepo` (list + spend sum) so windowed
/// payment caps and hold execution behave identically without Postgres.
#[derive(Debug, Default)]
pub struct MemoryTraceStore {
    traces: std::sync::Mutex<Vec<StoredTrace>>,
}

#[async_trait]
impl TraceStore for MemoryTraceStore {
    async fn list_recent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TraceSummary>, TraceStoreError> {
        let traces = self.traces.lock().expect("trace store lock");
        let mut rows: Vec<&StoredTrace> = traces
            .iter()
            .filter(|t| {
                t.workspace_id == workspace_id
                    && t.summary.environment_id == environment_id
                    && session_id.map_or(true, |sid| t.summary.session_id.as_deref() == Some(sid))
            })
            .collect();
        rows.sort_by_key(|t| std::cmp::Reverse(t.created_at));
        Ok(rows
            .into_iter()
            .take(limit)
            .map(|t| t.summary.clone())
            .collect())
    }

    async fn sum_payment_minor_since(
        &self,
        workspace_id: &str,
        owner: &str,
        operations: &[String],
        since: DateTime<Utc>,
    ) -> Result<i64, TraceStoreError> {
        let traces = self.traces.lock().expect("trace store lock");
        let mut total: i64 = 0;
        for t in traces.iter() {
            if t.workspace_id != workspace_id
                || t.summary.decision != "allow"
                || t.created_at < since
            {
                continue;
            }
            let event = t.summary.payload.get("event");
            let agent = event
                .and_then(|e| e.get("principal"))
                .and_then(|p| p.get("agent_id"))
                .and_then(|v| v.as_str());
            let operation = event
                .and_then(|e| e.get("action"))
                .and_then(|a| a.get("operation"))
                .and_then(|v| v.as_str());
            if agent != Some(owner)
                || !operation.is_some_and(|op| operations.iter().any(|o| o == op))
            {
                continue;
            }
            let amount = event
                .and_then(|e| e.get("action"))
                .and_then(|a| a.get("parameters"))
                .and_then(|p| p.get("amount"))
                .and_then(serde_json::Value::as_i64);
            if let Some(amount) = amount {
                total = total.saturating_add(amount);
            }
        }
        Ok(total)
    }

    async fn record(&self, write: TraceWriteRequest) -> Result<(), TraceStoreError> {
        // Payload format mirrors the postgres writer: full Decision + an
        // additive `event` key, so readers parse `payload.event.…` the same
        // against both stores.
        let mut payload = serde_json::to_value(&write.decision).unwrap_or(serde_json::Value::Null);
        if let (Some(event), Some(object)) = (write.event.as_ref(), payload.as_object_mut()) {
            match serde_json::to_value(event) {
                Ok(evidence) => {
                    object.insert("event".into(), evidence);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "event evidence serialization failed; bare decision payload");
                }
            }
        }
        let now = Utc::now();
        let summary = TraceSummary {
            trace_id: write.decision.trace_id.clone(),
            run_id: write.run_id,
            run_event_id: write.run_event_id,
            session_id: write.session_id,
            environment_id: write.environment_id.clone(),
            environment: write.environment_id,
            domain: write.domain,
            decision: verdict_text(write.decision.verdict).to_string(),
            elapsed_ms: write.decision.latency_ms as i32,
            latest_review_outcome: None,
            latest_reviewed_at: None,
            payload,
            created_at: now.to_rfc3339(),
        };
        let mut traces = self.traces.lock().expect("trace store lock");
        traces.push(StoredTrace {
            workspace_id: write.workspace_id,
            summary,
            created_at: now,
        });
        // Bound memory: this store is dev/test only (Postgres is the
        // production trace path). Without a cap, one push per event grows
        // unbounded and every read is an O(n) scan — a DoS on non-Postgres
        // deployments. Drop the oldest beyond the cap; windowed spend caps on
        // the memory path are best-effort within this horizon.
        let overflow = traces.len().saturating_sub(MEMORY_TRACE_CAP);
        if overflow > 0 {
            traces.drain(0..overflow);
        }
        Ok(())
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        trace_id: &str,
    ) -> Result<Option<TraceSummary>, TraceStoreError> {
        let traces = self.traces.lock().expect("trace store lock");
        Ok(traces
            .iter()
            .find(|t| {
                t.workspace_id == workspace_id
                    && t.summary.environment_id == environment_id
                    && t.summary.trace_id == trace_id
            })
            .map(|t| t.summary.clone()))
    }
}

/// Observation double: forwards every `record` into an mpsc receiver and
/// serves empty reads. Lets integration tests assert on enqueued traces
/// through the same seam the postgres writer uses, without Postgres.
pub struct ChannelTraceStore {
    tx: tokio::sync::mpsc::Sender<TraceWriteRequest>,
}

impl ChannelTraceStore {
    pub fn channel(buffer: usize) -> (Arc<Self>, tokio::sync::mpsc::Receiver<TraceWriteRequest>) {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer);
        (Arc::new(Self { tx }), rx)
    }
}

#[async_trait]
impl TraceStore for ChannelTraceStore {
    async fn list_recent(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _session_id: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<TraceSummary>, TraceStoreError> {
        Ok(vec![])
    }

    async fn sum_payment_minor_since(
        &self,
        _workspace_id: &str,
        _owner: &str,
        _operations: &[String],
        _since: DateTime<Utc>,
    ) -> Result<i64, TraceStoreError> {
        Ok(0)
    }

    async fn record(&self, write: TraceWriteRequest) -> Result<(), TraceStoreError> {
        let _ = self.tx.try_send(write);
        Ok(())
    }
}

#[derive(Clone)]
pub struct TraceState {
    pub store: Arc<dyn TraceStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
}

/// `GET /v1/traces` - list recent persisted decision traces for a workspace.
#[utoipa::path(
    get,
    path = "/v1/traces",
    tag = "traces",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum traces to return, capped at 100"),
        ("session_id" = Option<String>, Query, description = "Return only traces tagged with this monitoring session id"),
    ),
    responses(
        (status = 200, description = "Recent traces", body = TraceListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_traces(
    State(state): State<TraceState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(environment_id) => environment_id,
        Err(error) => return crate::environments::environment_error_response(error),
    };
    let limit = read_query_param(uri.query(), "limit")
        .and_then(|value| value.parse().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let session_id = read_query_param(uri.query(), "session_id");
    match state
        .store
        .list_recent(&workspace_id, &environment_id, session_id.as_deref(), limit)
        .await
    {
        Ok(traces) => Json(TraceListResponse { traces }).into_response(),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

/// Read a single percent-decoded query parameter, mirroring the other
/// query parsers in this crate. An empty value is treated as absent.
fn read_query_param(query: Option<&str>, name: &str) -> Option<String> {
    url::form_urlencoded::parse(query?.as_bytes())
        .find(|(key, value)| key == name && !value.is_empty())
        .map(|(_, value)| value.into_owned())
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    crate::log_api_error(status, code, &message);
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
