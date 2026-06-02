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

/// Generate a fresh trace id. UUIDv7 is time-ordered so callers (and
/// the storage layer's daily-partitioned tables) get cheap chronological
/// scans without a separate sequence.
pub fn new_trace_id() -> String {
    Uuid::now_v7().to_string()
}
