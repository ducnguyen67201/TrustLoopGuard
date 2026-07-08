use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::HumanReviewOutcome;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TraceSummary {
    pub trace_id: String,
    pub run_id: Option<String>,
    pub run_event_id: Option<String>,
    /// Monitoring session id the trace was tagged with, if any.
    // serde(default) without skip_serializing_if: the key is always
    // present in responses, matching run_id / run_event_id on this
    // struct — consumers may rely on its presence.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub session_id: Option<String>,
    pub environment_id: String,
    pub environment: String,
    pub domain: String,
    pub decision: String,
    pub elapsed_ms: i32,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub latest_review_outcome: Option<HumanReviewOutcome>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub latest_reviewed_at: Option<String>,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub payload: serde_json::Value,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TraceListResponse {
    pub traces: Vec<TraceSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum TraceGraphNodeKind {
    Trace,
    Event,
    Decision,
    Source,
    Parameter,
    Output,
    Tool,
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum TraceGraphEdgeKind {
    Contains,
    DecidedAs,
    Invokes,
    ProposesOutput,
    WritesMemory,
    ReadsMemory,
    Influences,
    Derives,
    UsedByAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TraceGraphNode {
    pub id: String,
    pub kind: TraceGraphNodeKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<Vec<String>>", optional))]
    pub trace_ids: Vec<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TraceGraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: TraceGraphEdgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub label: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TraceGraphResponse {
    pub nodes: Vec<TraceGraphNode>,
    pub edges: Vec<TraceGraphEdge>,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub trace_count: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub event_count: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub missing_event_count: u32,
}

/// Generate a fresh trace id. UUIDv7 is time-ordered so callers (and
/// the storage layer's daily-partitioned tables) get cheap chronological
/// scans without a separate sequence.
pub fn new_trace_id() -> String {
    Uuid::now_v7().to_string()
}
