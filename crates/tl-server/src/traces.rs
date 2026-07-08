//! Dashboard trace read endpoints.

use std::collections::BTreeMap;
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
use tl_core::{
    ApiError, ApiErrorCode, EventKind, GuardEvent, Origin, TraceGraphEdge, TraceGraphEdgeKind,
    TraceGraphNode, TraceGraphNodeKind, TraceGraphResponse, TraceListResponse, TraceSummary,
};

use crate::environments::EnvironmentStore;

#[derive(Debug, thiserror::Error)]
pub enum TraceStoreError {
    #[error("internal: {0}")]
    Internal(String),
}

/// Feature-independent trace write. Every decision path records through this
/// one seam: the postgres adapter converts it to
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

    /// Record a decision trace. Best-effort on the postgres path (batched
    /// channel, same as before); synchronous on the memory path.
    async fn record(&self, write: TraceWriteRequest) -> Result<(), TraceStoreError>;

    /// Fetch a single trace by id (not window-bounded, unlike `list_recent`).
    /// Default `None` for stores without point lookup (the channel test double).
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
/// listing semantics of `tl_storage::TraceRepo`.
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

/// `GET /v1/traces/graph` - build a source/action graph from persisted trace
/// events. Use `trace_id` for one trace, otherwise recent traces are selected
/// by `session_id` + `limit`.
#[utoipa::path(
    get,
    path = "/v1/traces/graph",
    tag = "traces",
    params(
        ("trace_id" = Option<String>, Query, description = "Build a graph for one trace id"),
        ("session_id" = Option<String>, Query, description = "Build a graph for recent traces in this monitoring session"),
        ("limit" = Option<usize>, Query, description = "Maximum traces to include when trace_id is absent, capped at 100"),
    ),
    responses(
        (status = 200, description = "Trace graph", body = TraceGraphResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Trace not found", body = ApiError),
    ),
)]
pub async fn trace_graph(
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

    if let Some(trace_id) = read_query_param(uri.query(), "trace_id") {
        return match state
            .store
            .get(&workspace_id, &environment_id, &trace_id)
            .await
        {
            Ok(Some(trace)) => Json(build_trace_graph(&[trace])).into_response(),
            Ok(None) => api_error_response(
                StatusCode::NOT_FOUND,
                ApiErrorCode::NotFound,
                "trace not found".into(),
            ),
            Err(e) => api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            ),
        };
    }

    let limit = read_query_param(uri.query(), "limit")
        .and_then(|value| value.parse().ok())
        .unwrap_or(50)
        .clamp(1, 100);
    let session_id = read_query_param(uri.query(), "session_id");
    match state
        .store
        .list_recent(&workspace_id, &environment_id, session_id.as_deref(), limit)
        .await
    {
        Ok(traces) => Json(build_trace_graph(&traces)).into_response(),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

fn build_trace_graph(traces: &[TraceSummary]) -> TraceGraphResponse {
    let mut graph = GraphBuilder::default();
    for trace in traces {
        graph.add_trace(trace);
    }
    graph.finish(traces.len() as u32)
}

#[derive(Default)]
struct GraphBuilder {
    nodes: BTreeMap<String, TraceGraphNode>,
    edges: BTreeMap<String, TraceGraphEdge>,
    event_count: u32,
    missing_event_count: u32,
}

impl GraphBuilder {
    fn add_trace(&mut self, trace: &TraceSummary) {
        let trace_id = trace.trace_id.clone();
        let trace_node = format!("trace:{trace_id}");
        self.upsert_node(
            trace_node.clone(),
            TraceGraphNodeKind::Trace,
            trace.trace_id.clone(),
            &trace_id,
            json!({
                "decision": trace.decision,
                "domain": trace.domain,
                "created_at": trace.created_at,
                "run_id": trace.run_id,
                "run_event_id": trace.run_event_id,
                "session_id": trace.session_id,
            }),
        );
        let decision_node = format!("decision:{trace_id}");
        self.upsert_node(
            decision_node.clone(),
            TraceGraphNodeKind::Decision,
            trace.decision.clone(),
            &trace_id,
            json!({
                "verdict": trace.decision,
                "elapsed_ms": trace.elapsed_ms,
                "payload_reason": trace.payload.get("reason").cloned().unwrap_or(serde_json::Value::Null),
                "violated_rule": trace.payload.get("violated_rule").cloned().unwrap_or(serde_json::Value::Null),
            }),
        );
        self.insert_edge(
            &trace_id,
            trace_node.clone(),
            decision_node,
            TraceGraphEdgeKind::DecidedAs,
            Some(trace.decision.clone()),
            serde_json::Value::Null,
        );

        let Some(event) = event_from_trace(trace) else {
            self.missing_event_count += 1;
            return;
        };
        self.event_count += 1;
        self.add_event(trace, &trace_node, &event);
    }

    fn add_event(&mut self, trace: &TraceSummary, trace_node: &str, event: &GuardEvent) {
        let trace_id = &trace.trace_id;
        let event_label = event_kind_label(&event.kind);
        let event_node = format!("event:{trace_id}");
        self.upsert_node(
            event_node.clone(),
            TraceGraphNodeKind::Event,
            event_label.clone(),
            trace_id,
            json!({
                "kind": event_label,
                "operation": event.action.operation,
                "side_effect": event.action.side_effect,
                "agent_id": event.principal.agent_id,
                "user_id": event.principal.user_id,
                "task_id": event.principal.task_id,
            }),
        );
        self.insert_edge(
            trace_id,
            trace_node.to_string(),
            event_node.clone(),
            TraceGraphEdgeKind::Contains,
            Some("event".into()),
            serde_json::Value::Null,
        );

        let tool_node = (!event.action.operation.trim().is_empty()).then(|| {
            let tool_node = format!("tool:{}", event.action.operation);
            self.upsert_node(
                tool_node.clone(),
                TraceGraphNodeKind::Tool,
                event.action.operation.clone(),
                trace_id,
                json!({
                    "operation": event.action.operation,
                    "side_effect": event.action.side_effect,
                    "resolution": event.resolution,
                }),
            );
            self.insert_edge(
                trace_id,
                event_node.clone(),
                tool_node.clone(),
                TraceGraphEdgeKind::Invokes,
                Some(event.action.operation.clone()),
                serde_json::Value::Null,
            );
            tool_node
        });

        match event.kind {
            EventKind::OutputProposed => {
                let output_node = format!("output:{trace_id}");
                self.upsert_node(
                    output_node.clone(),
                    TraceGraphNodeKind::Output,
                    "output.proposed".into(),
                    trace_id,
                    json!({ "parameters": event.action.parameters }),
                );
                self.insert_edge(
                    trace_id,
                    event_node.clone(),
                    output_node,
                    TraceGraphEdgeKind::ProposesOutput,
                    None,
                    serde_json::Value::Null,
                );
            }
            EventKind::MemoryWriteProposed => {
                let memory_node = format!("memory:{trace_id}:write");
                self.upsert_node(
                    memory_node.clone(),
                    TraceGraphNodeKind::Memory,
                    event.action.operation.clone(),
                    trace_id,
                    json!({ "operation": event.action.operation }),
                );
                self.insert_edge(
                    trace_id,
                    event_node.clone(),
                    memory_node,
                    TraceGraphEdgeKind::WritesMemory,
                    Some("memory.write".into()),
                    serde_json::Value::Null,
                );
            }
            EventKind::MemoryRetrievalUsedForAction => {
                let memory_node = format!("memory:{trace_id}:retrieval");
                self.upsert_node(
                    memory_node.clone(),
                    TraceGraphNodeKind::Memory,
                    event.action.operation.clone(),
                    trace_id,
                    json!({ "operation": event.action.operation }),
                );
                self.insert_edge(
                    trace_id,
                    memory_node,
                    event_node.clone(),
                    TraceGraphEdgeKind::ReadsMemory,
                    Some("memory.retrieval".into()),
                    serde_json::Value::Null,
                );
            }
            _ => {}
        }

        for source in &event.sources {
            let source_node = format!("source:{}", source.id);
            self.upsert_node(
                source_node.clone(),
                TraceGraphNodeKind::Source,
                source.id.clone(),
                trace_id,
                json!({
                    "origin": source.origin,
                    "labels": source.labels,
                    "kind": source.kind,
                }),
            );
            self.insert_edge(
                trace_id,
                source_node.clone(),
                event_node.clone(),
                TraceGraphEdgeKind::Influences,
                Some("event".into()),
                serde_json::Value::Null,
            );
            if source.origin == Origin::Memory {
                let memory_node = format!("memory:{}", source.id);
                self.upsert_node(
                    memory_node.clone(),
                    TraceGraphNodeKind::Memory,
                    source.id.clone(),
                    trace_id,
                    json!({ "source_id": source.id, "kind": source.kind }),
                );
                self.insert_edge(
                    trace_id,
                    memory_node,
                    source_node,
                    TraceGraphEdgeKind::Contains,
                    Some("memory_source".into()),
                    serde_json::Value::Null,
                );
            }
        }

        for (path, source_ids) in &event.provenance.0 {
            let path_kind = match event.kind {
                EventKind::OutputProposed => TraceGraphNodeKind::Output,
                _ => TraceGraphNodeKind::Parameter,
            };
            let path_node = match path_kind {
                TraceGraphNodeKind::Output => format!("output:{trace_id}:{path}"),
                _ => format!("param:{trace_id}:{path}"),
            };
            self.upsert_node(
                path_node.clone(),
                path_kind,
                path.clone(),
                trace_id,
                json!({ "path": path, "source_ids": source_ids }),
            );
            if let Some(tool_node) = &tool_node {
                self.insert_edge(
                    trace_id,
                    path_node.clone(),
                    tool_node.clone(),
                    TraceGraphEdgeKind::UsedByAction,
                    Some(path.clone()),
                    json!({ "path": path }),
                );
            }
            for source_id in source_ids {
                let source_node = format!("source:{source_id}");
                self.insert_edge(
                    trace_id,
                    source_node,
                    path_node.clone(),
                    TraceGraphEdgeKind::Derives,
                    Some(path.clone()),
                    json!({ "path": path }),
                );
            }
        }
    }

    fn upsert_node(
        &mut self,
        id: String,
        kind: TraceGraphNodeKind,
        label: String,
        trace_id: &str,
        data: serde_json::Value,
    ) {
        if let Some(existing) = self.nodes.get_mut(&id) {
            if !existing.trace_ids.iter().any(|id| id == trace_id) {
                existing.trace_ids.push(trace_id.to_string());
            }
            return;
        }
        self.nodes.insert(
            id.clone(),
            TraceGraphNode {
                id,
                kind,
                label,
                trace_ids: vec![trace_id.to_string()],
                data,
            },
        );
    }

    fn insert_edge(
        &mut self,
        trace_id: &str,
        from: String,
        to: String,
        kind: TraceGraphEdgeKind,
        label: Option<String>,
        data: serde_json::Value,
    ) {
        let id = format!(
            "{trace_id}:{kind:?}:{from}:{to}:{}",
            label.as_deref().unwrap_or("")
        );
        self.edges.entry(id.clone()).or_insert(TraceGraphEdge {
            id,
            from,
            to,
            kind,
            trace_id: Some(trace_id.to_string()),
            label,
            data,
        });
    }

    fn finish(self, trace_count: u32) -> TraceGraphResponse {
        TraceGraphResponse {
            nodes: self.nodes.into_values().collect(),
            edges: self.edges.into_values().collect(),
            trace_count,
            event_count: self.event_count,
            missing_event_count: self.missing_event_count,
        }
    }
}

fn event_from_trace(trace: &TraceSummary) -> Option<GuardEvent> {
    serde_json::from_value(trace.payload.get("event")?.clone()).ok()
}

fn event_kind_label(kind: &EventKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{kind:?}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{
        Action, Decision, Labels, Principal, ProvenanceMap, SideEffectClass, Source, Verdict,
    };

    fn graph_trace(event: GuardEvent) -> TraceSummary {
        let mut decision = Decision::allow("trace-1");
        decision.verdict = Verdict::Block;
        decision.reason = "parameter_auth: parameter_source.recipient: wrong source".into();
        let mut payload = serde_json::to_value(&decision).expect("decision serializes");
        payload.as_object_mut().expect("decision object").insert(
            "event".into(),
            serde_json::to_value(event).expect("event serializes"),
        );

        TraceSummary {
            trace_id: "trace-1".into(),
            run_id: None,
            run_event_id: None,
            session_id: Some("session-1".into()),
            environment_id: "production".into(),
            environment: "production".into(),
            domain: "customer_support".into(),
            decision: "block".into(),
            elapsed_ms: 7,
            latest_review_outcome: None,
            latest_reviewed_at: None,
            payload,
            created_at: "2026-07-08T00:00:00Z".into(),
        }
    }

    fn tool_event() -> GuardEvent {
        let mut provenance = ProvenanceMap::default();
        provenance.insert("recipient", vec!["src.email".into()]);

        GuardEvent {
            kind: EventKind::ToolCallProposed,
            principal: Principal {
                workspace_id: "ws_1".into(),
                environment_id: "production".into(),
                agent_id: "agent-1".into(),
                user_id: None,
                session_id: Some("session-1".into()),
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: Action {
                operation: "send_email".into(),
                parameters: json!({ "recipient": "attacker@example.com" }),
                side_effect: Some(SideEffectClass::ExternalCommunication),
            },
            sources: vec![Source {
                id: "src.email".into(),
                origin: Origin::Email,
                labels: Labels::default(),
                kind: Some("inbound_message".into()),
            }],
            provenance,
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::Value::Null,
        }
    }

    #[test]
    fn graph_builds_source_to_parameter_to_tool_path() {
        let graph = build_trace_graph(&[graph_trace(tool_event())]);

        assert_eq!(graph.trace_count, 1);
        assert_eq!(graph.event_count, 1);
        assert_eq!(graph.missing_event_count, 0);
        assert!(graph.nodes.iter().any(|node| {
            node.id == "source:src.email" && node.kind == TraceGraphNodeKind::Source
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id == "param:trace-1:recipient" && node.kind == TraceGraphNodeKind::Parameter
        }));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.id == "tool:send_email" && node.kind == TraceGraphNodeKind::Tool));
        assert!(graph.edges.iter().any(|edge| {
            edge.from == "source:src.email"
                && edge.to == "param:trace-1:recipient"
                && edge.kind == TraceGraphEdgeKind::Derives
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from == "param:trace-1:recipient"
                && edge.to == "tool:send_email"
                && edge.kind == TraceGraphEdgeKind::UsedByAction
        }));
    }
}
