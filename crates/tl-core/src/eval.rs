use serde::{Deserialize, Serialize};

use crate::redteam::RedteamJobSummary;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Source that created or last refreshed a regression case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RegressionCaseSource {
    Harden,
    Manual,
}

/// Expected guarded outcome for a regression case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RegressionExpectedOutcome {
    /// Content-policy regressions should block the unsafe output.
    Block,
    /// Approval regressions should route the action to human review.
    Escalate,
    /// Event checker regressions may block or escalate, but must not allow.
    Stop,
}

/// Durable case in the evolving eval/regression ledger.
///
/// Cases promoted by harden point back to the source red-team job and evidence
/// sequence numbers. The red-team session store remains the source of truth for
/// the full captured trace; this summary is the stable suite/index row a
/// regression runner can select later.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionCaseSummary {
    pub id: String,
    pub case_key: String,
    pub environment_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_id: Option<String>,
    pub source: RegressionCaseSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub source_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_session_seqs: Vec<i32>,
    pub substrate: String,
    pub artifact_id: String,
    pub expected_outcome: RegressionExpectedOutcome,
    pub attack: String,
    pub goal: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Response from `GET /v1/redteam/regressions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionCaseListResponse {
    pub cases: Vec<RegressionCaseSummary>,
}

/// Body of `POST /v1/redteam/regressions/run`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionRunRequest {
    /// Source red-team job whose promoted cases should be re-run. The new job
    /// inherits this job's target URL, profile, agent id, and environment.
    pub source_job_id: String,
    /// Optional stable case keys to run. Empty means all promoted cases for the
    /// source job, capped by `limit`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub case_keys: Vec<String>,
    /// Maximum cases to include when `case_keys` is empty. Defaults to 20 and
    /// is capped at 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub limit: Option<usize>,
}

/// Response from `POST /v1/redteam/regressions/run`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionRunResponse {
    pub job: RedteamJobSummary,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub case_count: u32,
    pub case_keys: Vec<String>,
}

/// Per-case result status for a promoted regression case re-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RegressionResultStatus {
    Passed,
    Failed,
    Missing,
    Inconclusive,
}

/// Result for one promoted regression case within a completed red-team job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionCaseResult {
    pub case_key: String,
    pub expected_outcome: RegressionExpectedOutcome,
    pub status: RegressionResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub actual_outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub landed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub reason: Option<String>,
}

/// Computed metrics for a regression job over selected promoted cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionResultSummaryResponse {
    pub job: RedteamJobSummary,
    pub source_job_id: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub total: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub passed: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub failed: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub missing: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub inconclusive: u32,
    pub results: Vec<RegressionCaseResult>,
}

/// Durable historical summary for one regression result computation.
///
/// Result snapshots are idempotent by `(job_id, source_job_id, case_keys)` so a
/// CI job or dashboard refresh can ask for the same summary multiple times
/// without creating duplicate trend rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionResultSnapshotSummary {
    pub id: String,
    pub job_id: String,
    pub source_job_id: String,
    pub environment_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub case_keys: Vec<String>,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub total: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub passed: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub failed: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub missing: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub inconclusive: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// Response from `GET /v1/redteam/regressions/results`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RegressionResultTrendResponse {
    pub snapshots: Vec<RegressionResultSnapshotSummary>,
}
