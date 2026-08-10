use serde::{Deserialize, Serialize};

use crate::{
    EvaluationJobSummary, EvaluationResultSummary, RunFinalizationSummary, RunParticipantSummary,
    TraceSummary,
};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RunKind {
    ChatSession,
    LiveCall,
    Workflow,
    Job,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RunStatus {
    Warming,
    Running,
    Completed,
    Failed,
    Canceled,
    TimedOut,
}

impl RunStatus {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Canceled | Self::TimedOut
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RunEvaluationEligibility {
    #[default]
    Eligible,
    LegacyIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RunEventKind {
    UserTurn,
    AssistantTurn,
    ToolCall,
    WorkflowStep,
    Interruption,
    Retry,
    SystemEvent,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateRunRequest {
    pub agent_id: String,
    pub kind: RunKind,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub status: Option<RunStatus>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub external_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateRunRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub status: Option<RunStatus>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub metadata: Option<serde_json::Value>,
    /// RFC 3339 timestamp. Defaults to now when completing/failing/canceling
    /// a run without an explicit timestamp.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateRunEventRequest {
    /// Agent responsible for this event. Defaults to the run's primary agent.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_id: Option<String>,
    pub kind: RunEventKind,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub sequence: Option<i32>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub label: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub input_summary: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub output_summary: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub metadata: serde_json::Value,
    /// RFC 3339 timestamp. Defaults to now when omitted.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunSummary {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub environment: String,
    pub agent_id: String,
    pub kind: RunKind,
    pub status: RunStatus,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub evaluation_eligibility: Option<RunEvaluationEligibility>,
    pub external_id: Option<String>,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub metadata: serde_json::Value,
    /// RFC 3339 timestamp.
    pub started_at: String,
    /// RFC 3339 timestamp.
    pub ended_at: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub trace_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub blocked_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub rewritten_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub escalated_count: i64,
    pub p95_latency_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunListResponse {
    pub runs: Vec<RunSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunEventSummary {
    pub id: String,
    pub workspace_id: String,
    pub run_id: String,
    pub agent_id: String,
    pub sequence: i32,
    pub kind: RunEventKind,
    pub label: Option<String>,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub metadata: serde_json::Value,
    /// RFC 3339 timestamp.
    pub occurred_at: String,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunEventListResponse {
    pub events: Vec<RunEventSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunProviderUsage {
    pub gateway_request_id: String,
    pub route_id: String,
    pub provider: String,
    pub model: String,
    pub provider_response_id: Option<String>,
    pub status: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub latency_ms: u64,
    pub estimated_cost_usd_nanos: Option<String>,
    pub input_rate_usd_per_million_nanos: Option<String>,
    pub output_rate_usd_per_million_nanos: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunGuardrailUsage {
    pub phase: String,
    pub judge: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub estimated_cost_usd_nanos: Option<String>,
    pub fallback_used: bool,
    pub latency_ms: u64,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunBudgetWindowSnapshot {
    pub window: String,
    pub cap_usd_nanos: String,
    pub committed_before_usd_nanos: String,
    pub reserved_before_usd_nanos: String,
    pub requested_usd_nanos: String,
    pub remaining_after_usd_nanos: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunLlmBudgetDecision {
    pub principal_id: String,
    pub status: String,
    pub currency: String,
    pub governing_window: Option<String>,
    pub requested_usd_nanos: Option<String>,
    pub actual_usd_nanos: Option<String>,
    pub windows: Vec<RunBudgetWindowSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunSpanSummary {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub agent_id: String,
    pub run_event_id: Option<String>,
    pub name: String,
    pub span_kind: i32,
    pub operation_name: Option<String>,
    pub conversation_id: Option<String>,
    pub external_agent_id: Option<String>,
    /// RFC 3339 timestamp.
    pub started_at: String,
    /// RFC 3339 timestamp.
    pub ended_at: String,
    pub status_code: i32,
    pub status_message: Option<String>,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub resource: serde_json::Map<String, serde_json::Value>,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub attributes: serde_json::Map<String, serde_json::Value>,
    #[cfg_attr(feature = "ts-export", ts(type = "Array<Record<string, unknown>>"))]
    pub events: Vec<serde_json::Value>,
    #[cfg_attr(feature = "ts-export", ts(type = "Array<Record<string, unknown>>"))]
    pub links: Vec<serde_json::Value>,
    pub content_capture_status: String,
    pub dropped_attribute_count: i32,
    pub late_evidence: bool,
    /// RFC 3339 timestamp.
    pub ingested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunDetail {
    pub run: RunSummary,
    pub events: Vec<RunEventSummary>,
    pub traces: Vec<TraceSummary>,
    #[serde(default)]
    pub spans: Vec<RunSpanSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub provider_usage: Option<RunProviderUsage>,
    #[serde(default)]
    pub guardrail_usage: Vec<RunGuardrailUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub budget_decision: Option<RunLlmBudgetDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub finalization: Option<RunFinalizationSummary>,
    #[serde(default)]
    pub participants: Vec<RunParticipantSummary>,
    #[serde(default)]
    pub evaluation_jobs: Vec<EvaluationJobSummary>,
    #[serde(default)]
    pub evaluations: Vec<EvaluationResultSummary>,
}

#[cfg(test)]
mod tests {
    use super::RunStatus;

    #[test]
    fn only_closed_run_states_are_terminal() {
        assert!(!RunStatus::Warming.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
        assert!(RunStatus::Completed.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(RunStatus::Canceled.is_terminal());
        assert!(RunStatus::TimedOut.is_terminal());
    }

    #[test]
    fn timed_out_uses_the_canonical_wire_value() {
        assert_eq!(
            serde_json::to_string(&RunStatus::TimedOut).unwrap(),
            "\"timed_out\""
        );
    }
}
