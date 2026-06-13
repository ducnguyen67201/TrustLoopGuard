//! Wire types for the red-team dispatch orchestrator (`/v1/redteam/*`).
//!
//! A *dispatch* creates a durable *job* that the server runs in the background by
//! driving the standalone attack runner (which executes hackagent). The server
//! owns the job + per-attack results; the runner owns nothing.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Lifecycle of a dispatched red-team job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum JobStatus {
    Queued,
    Running,
    Complete,
    Error,
    Cancelled,
}

/// Which attack generator the runner should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RedteamGenerator {
    /// Deterministic built-in attack catalogue (no external engine, no LLM).
    Deterministic,
    /// hackagent-generated adversarial cases (UNVALIDATED; falls back to deterministic).
    Hackagent,
}

/// Body of `POST /v1/redteam/dispatch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamDispatchRequest {
    /// Loopback agent endpoint to attack (arena adapter contract).
    pub target_url: String,
    /// `fast` | `full` | `max`.
    pub profile: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub generator: Option<RedteamGenerator>,
    /// Optional registered agent this job is associated with (for history).
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_id: Option<String>,
}

/// Summary row for a dispatched job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobSummary {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub status: JobStatus,
    pub target: String,
    pub profile: String,
    pub generator: RedteamGenerator,
    // Serialized as `null` when absent (serde sends `None` as null), so the wire
    // type is `string | null`, not an omitted key. No `ts(optional)`.
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Non-control attacks attempted.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub attacks: i64,
    /// Attacks that got through.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub landed: i64,
    /// Attacks the guard blocked.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub blocked: i64,
    #[serde(default)]
    pub error: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

/// One scored attack within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobResult {
    pub seq: i32,
    pub attack: String,
    pub goal: String,
    /// `landed` | `blocked` | `clean` | `error`.
    pub outcome: String,
    pub landed: bool,
    #[serde(default)]
    pub prompt: Option<String>,
    pub reply: String,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Response from `GET /v1/redteam/jobs/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobDetail {
    pub job: RedteamJobSummary,
    pub results: Vec<RedteamJobResult>,
}

/// Response from `GET /v1/redteam/jobs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobListResponse {
    pub jobs: Vec<RedteamJobSummary>,
}

/// Response from `GET /v1/redteam/jobs/{id}/results`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobResultListResponse {
    pub results: Vec<RedteamJobResult>,
}

/// One attack result flattened with its parent job's context. Powers the
/// workspace-wide records browser (`GET /v1/redteam/attacks`), which lists
/// attacks across all jobs rather than within a single job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamAttackRecord {
    /// The parent job this attack belongs to.
    pub job_id: String,
    pub target: String,
    pub profile: String,
    /// RFC 3339 timestamp of the parent job's creation.
    pub created_at: String,
    pub seq: i32,
    pub attack: String,
    pub goal: String,
    /// `landed` | `blocked` | `clean` | `error`.
    pub outcome: String,
    pub landed: bool,
    #[serde(default)]
    pub prompt: Option<String>,
    pub reply: String,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Response from `GET /v1/redteam/attacks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamAttackRecordListResponse {
    pub records: Vec<RedteamAttackRecord>,
}
