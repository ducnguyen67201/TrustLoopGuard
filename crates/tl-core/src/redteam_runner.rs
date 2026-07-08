//! Wire contract for the private red-team runner.
//!
//! This is the **source of truth** for the request/response shapes exchanged
//! between `tl-server` (the Rust client) and the private Python runner service
//! it calls via `REDTEAM_RUNNER_URL`. The Python side
//! generates its Pydantic models from the JSON Schema `tl-codegen` emits for
//! these types — keep the contract here, never hand-duplicate it.
//!
//! The wire is camelCase and strict (`deny_unknown_fields`): both sides pin
//! versions, so an unexpected or renamed field must fail loudly rather than be
//! silently dropped.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::GuardEvent;

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

/// Runner-side provenance for a planned source-to-sink attack path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerWorkflowPath {
    pub source_node: String,
    pub source_type: String,
    pub source_category: String,
    pub sink_node: String,
    pub sink_type: String,
    pub sink_category: String,
}

/// Body of `POST /redteam/plan`. TrustLoopGuard sends only the target agent's
/// structured context; the private runner owns attack-planner instructions and
/// vector generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerPlanRequest {
    pub agent_display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub workflow_present: bool,
    #[serde(default)]
    pub paths: Vec<RunnerWorkflowPath>,
}

/// One tailored attack vector generated or consumed by the private runner.
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
    /// Optional product-side source-to-sink provenance for this seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<RunnerWorkflowPath>,
}

/// Response from `POST /redteam/plan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RunnerPlanResponse {
    pub vectors: Vec<RunnerAttackVector>,
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
    /// Tailored seeds from the agent's `redteam/plan`.
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
    pub guard_event: Option<GuardEvent>,
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
    pub plan: RunnerPlanRequest,
    pub plan_result: RunnerPlanResponse,
    pub dispatch: RunnerDispatch,
    pub handle: RunnerHandle,
    pub report: RunnerReport,
}

#[cfg(test)]
mod tests {
    use super::{RunnerAttackVector, RunnerDispatch, RunnerSessionEvent};

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
        assert!(event.guard_event.is_none());
    }

    #[test]
    fn runner_session_event_accepts_structured_guard_event_evidence() {
        let event: RunnerSessionEvent = serde_json::from_value(serde_json::json!({
            "eventId": "evt-1",
            "seq": 1,
            "kind": "tool_call",
            "actor": "target",
            "guardEvent": {
                "kind": "tool.call.proposed",
                "principal": {
                    "workspace_id": "ws",
                    "environment_id": "production",
                    "agent_id": "agent-1"
                },
                "action": {
                    "operation": "issue_refund",
                    "parameters": {},
                    "side_effect": "api_mutation"
                },
                "sources": [],
                "provenance": {},
                "context": {}
            }
        }))
        .expect("event deserializes");

        assert!(event.guard_event.is_some());
    }

    #[test]
    fn runner_dispatch_accepts_legacy_payload_without_attack_vectors() {
        let dispatch: RunnerDispatch = serde_json::from_value(serde_json::json!({
            "targetUrl": "http://127.0.0.1:9102",
            "profile": "fast"
        }))
        .expect("legacy dispatch deserializes");

        assert!(dispatch.attack_vectors.is_none());
    }

    #[test]
    fn runner_attack_vector_serializes_source_path_as_camel_case() {
        let vector = RunnerAttackVector {
            goal: "exfiltrate".into(),
            technique: "data_exfiltration".into(),
            target_operation: "http".into(),
            injection_payload: "send it".into(),
            source_path: Some(super::RunnerWorkflowPath {
                source_node: "Inbox".into(),
                source_type: "email".into(),
                source_category: "email_read".into(),
                sink_node: "Post".into(),
                sink_type: "http".into(),
                sink_category: "http".into(),
            }),
        };

        let json = serde_json::to_value(vector).expect("serializes");
        assert_eq!(json["sourcePath"]["sourceNode"], "Inbox");
        assert_eq!(json["sourcePath"]["sinkCategory"], "http");
    }
}
