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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

fn empty_json_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// Execution mode for the private runner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum RunnerRunMode {
    #[default]
    OneOff,
    Learning,
}

/// Target surface the private runner should attack.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum RunnerAttackSurface {
    #[default]
    Chat,
    DocumentWorkflow,
}

/// Uploaded PDF form template for document workflow attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerDocumentTemplate {
    pub file_name: String,
    pub media_type: String,
    pub data_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, String>>,
    #[serde(default)]
    pub flatten: bool,
}

/// One tailored attack vector handed to the runner as a seed. The runner feeds
/// these into HackAgent's case strengthening so attacks are specific to the
/// target agent — gray-box, not generic templates. Carries only what the runner
/// needs to seed an attack; the product-side `AttackVector` keeps richer
/// provenance (e.g. the workflow path) that the runner does not.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerAttackVector {
    /// What a successful attack makes the agent do (the runner's scoring goal).
    pub goal: String,
    /// Technique class, e.g. `data_exfiltration`, `scope_violation`.
    pub technique: String,
    /// Sink category the vector targets (`http`, `email_send`, …) or `chat_reply`.
    pub target_operation: String,
    /// Concrete seed payload; the runner strengthens it.
    pub injection_payload: String,
}

/// Body of `POST /redteam/jobs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerDispatch {
    pub target_url: String,
    pub profile: String,
    #[serde(default, skip_serializing_if = "runner_mode_is_default")]
    pub mode: RunnerRunMode,
    #[serde(default, skip_serializing_if = "runner_attack_surface_is_default")]
    pub attack_surface: RunnerAttackSurface,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_template: Option<RunnerDocumentTemplate>,
    /// Tailored seeds from the agent's `redteam/plan`. Absent ⇒ the runner uses
    /// its generic attack pack (back-compatible).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_vectors: Option<Vec<RunnerAttackVector>>,
}

fn runner_mode_is_default(mode: &RunnerRunMode) -> bool {
    *mode == RunnerRunMode::OneOff
}

fn runner_attack_surface_is_default(surface: &RunnerAttackSurface) -> bool {
    *surface == RunnerAttackSurface::Chat
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

/// One ordered event emitted while executing an attack session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerSessionEvent {
    pub event_id: String,
    pub seq: i32,
    pub kind: String,
    pub actor: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub content_text: Option<String>,
    #[serde(default = "empty_json_object")]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// One independent attack session from the runner. The runner owns scoring;
/// Rust copies the verdict verbatim and never re-scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerAttackSession {
    pub session_id: String,
    #[serde(default)]
    pub runner_session_id: Option<String>,
    pub seq: i32,
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
    pub status: RunnerStatus,
    pub outcome: String,
    pub landed: bool,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub events: Vec<RunnerSessionEvent>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Response from `GET /redteam/jobs/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerReport {
    pub status: RunnerStatus,
    #[serde(default)]
    pub sessions: Vec<RunnerAttackSession>,
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

#[cfg(test)]
mod tests {
    use super::RunnerSessionEvent;

    #[test]
    fn runner_session_event_defaults_missing_payload_to_object() {
        let event: RunnerSessionEvent = serde_json::from_value(serde_json::json!({
            "eventId": "evt-1",
            "seq": 1,
            "kind": "target_reply",
            "actor": "target"
        }))
        .expect("event deserializes");

        assert_eq!(event.payload, serde_json::json!({}));
    }
}
