use serde::{Deserialize, Serialize};

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
pub enum HumanReviewOutcome {
    Accepted,
    Corrected,
    Rejected,
    FalsePositive,
    MissedIssue,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateHumanReviewEventRequest {
    pub outcome: HumanReviewOutcome,
    #[serde(default)]
    pub reason_codes: Vec<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub note: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewEvent {
    pub id: String,
    pub workspace_id: String,
    pub trace_id: String,
    pub run_id: Option<String>,
    pub run_event_id: Option<String>,
    pub outcome: HumanReviewOutcome,
    #[serde(default)]
    pub reason_codes: Vec<String>,
    pub note: Option<String>,
    pub reviewer_id: Option<String>,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub metadata: serde_json::Value,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewEventListResponse {
    pub review_events: Vec<HumanReviewEvent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewAnalyticsSummary {
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub trace_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub automated_intervention_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub human_review_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub human_intervention_count: i64,
    pub human_intervention_rate: f64,
    pub false_positive_rate: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewOutcomeCounts {
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub accepted_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub corrected_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub rejected_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub false_positive_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub missed_issue_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub ignored_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewWorkflowStepRow {
    pub workflow_step: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub human_review_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub corrected_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub rejected_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub false_positive_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewPolicyRow {
    pub policy_id: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub escalation_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub corrected_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub false_positive_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewGroupRow {
    pub group: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub human_review_count: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub human_intervention_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewReasonRow {
    pub reason_code: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HumanReviewAnalyticsResponse {
    pub summary: HumanReviewAnalyticsSummary,
    pub outcomes: HumanReviewOutcomeCounts,
    pub by_workflow_step: Vec<HumanReviewWorkflowStepRow>,
    pub by_policy: Vec<HumanReviewPolicyRow>,
    pub by_agent: Vec<HumanReviewGroupRow>,
    pub by_run_kind: Vec<HumanReviewGroupRow>,
    pub top_reasons: Vec<HumanReviewReasonRow>,
}
