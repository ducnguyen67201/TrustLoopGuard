//! Wire contract for the private red-team runner.
//!
//! This is the **source of truth** for the request/response shapes exchanged
//! between `tl-server` (the Rust client) and the HackAgentOrchestration runner
//! (the Python service it calls via `REDTEAM_RUNNER_URL`). The Python side
//! generates its Pydantic models from the JSON Schema `tl-codegen` emits for
//! these types — keep the contract here, never hand-duplicate it.
//!
//! The wire is camelCase and strict (`deny_unknown_fields`): both sides pin
//! versions, so an unexpected or renamed field must fail loudly rather than be
//! silently dropped.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Body of `POST /redteam/jobs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerDispatch {
    pub target_url: String,
    pub profile: String,
}

/// Response from `POST /redteam/jobs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerHandle {
    pub job_id: String,
}

/// Lifecycle the runner reports for one of its jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum RunnerStatus {
    Running,
    Complete,
    Error,
}

/// One scored attack from the runner. The runner owns scoring; Rust copies the
/// verdict verbatim and never re-scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerAttack {
    #[serde(default)]
    pub case_id: Option<String>,
    #[serde(default)]
    pub track: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub trial_index: Option<i32>,
    pub attack: String,
    pub goal: String,
    pub outcome: String,
    pub landed: bool,
    #[serde(default)]
    pub prompt: Option<String>,
    pub reply: String,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Response from `GET /redteam/jobs/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerReport {
    pub status: RunnerStatus,
    #[serde(default)]
    pub attacks: Vec<RunnerAttack>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Codegen-only aggregator so a single JSON Schema document carries every runner
/// type in its `definitions`. Never constructed at runtime.
#[cfg(feature = "schema")]
#[derive(JsonSchema)]
#[allow(dead_code)]
pub struct RedteamRunnerContract {
    pub dispatch: RunnerDispatch,
    pub handle: RunnerHandle,
    pub report: RunnerReport,
}
