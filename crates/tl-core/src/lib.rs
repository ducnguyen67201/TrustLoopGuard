//! Core types for TrustLoopGuard. Stable across all other crates.
//!
//! # Versioning
//!
//! These types are the **wire format**. Compatibility is enforced at the
//! HTTP layer via the URL path (`/v1/...`, `/v2/...`), not via a body
//! discriminator. When the wire shape needs to break, copy this module
//! into `crates/tl-core/src/v2.rs` and let both compile in parallel.
//!
//! # Codegen
//!
//! `tl-codegen` reads these types and emits:
//! - `docs/openapi.yaml` (via `utoipa`)
//! - `policies/schema.json` (via `schemars`)
//! - `sdks/typescript/src/types.ts` (via `ts-rs`)
//!
//! CI fails if the committed artifacts diverge from what the derives produce.
//! Do not hand-edit those files.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

pub mod agent;
pub mod auth;
pub mod policy;
pub mod team;
pub mod tier;

pub use agent::{
    AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone, KnowledgeSource,
    KnowledgeSourceKind,
};
pub use auth::{AuthRequest, AuthResponse, ChangePasswordRequest};
pub use policy::{
    AiEditRequest, AiEditResponse, EntityVersionDetail, EntityVersionListResponse,
    EntityVersionSummary, GuardrailGenerateResponse, GuardrailListResponse, PolicyAction,
    PolicyBatchSetEnabledRequest, PolicyBatchSetEnabledResponse, PolicyDocument, PolicyDraft,
    PolicyDraftRequest, PolicyDraftResponse, PolicyListResponse, PolicyMatchType,
    PolicySetEnabledRequest, PolicySummary, PolicyValidateResponse, PolicyValidationIssue,
};
pub use team::{
    CreateInviteRequest, CreateInviteResponse, CreateWorkspaceRequest, InviteListResponse,
    InviteStatus, MemberListResponse, MyWorkspace, MyWorkspacesResponse, WorkspaceInvite,
    WorkspaceMember, WorkspaceRole,
};
pub use tier::{Tier, TierResult, TierStatus};

/// Backwards-compatible workspace used when older clients do not send
/// workspace context. New clients should send `workspace_id` on `/v1/check`
/// or `X-TLG-Workspace-Id` on authoring endpoints.
pub const DEFAULT_WORKSPACE_ID: &str = "default";

/// Channel an agent is operating on. Drives latency budget and matcher selection.
///
/// Flat enum on the wire so SDK type generation stays clean across languages.
/// New channels are added as variants here; we don't carry a free-form
/// `Other(String)` because it pollutes the Pydantic / TS surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Channel {
    Voice,
    Chat,
    Email,
}

/// What TrustLoopGuard tells the caller to do with the proposed output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Verdict {
    Allow,
    Block,
    Rewrite,
    Escalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CheckRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub workspace_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub run_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub run_event_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub run_event: Option<CreateRunEventRequest>,
    pub agent_id: String,
    pub channel: Channel,
    pub input: String,
    pub proposed_output: String,
    /// Optional domain selector for the dispatcher. Defaults to
    /// `customer_support` server-side when absent. Reserved for future
    /// `voice_agent` / `coding_agent` handlers.
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub policies: Vec<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub context: serde_json::Value,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub redaction: Option<RedactionInfo>,
}

impl Default for CheckRequest {
    fn default() -> Self {
        Self {
            workspace_id: None,
            run_id: None,
            run_event_id: None,
            run_event: None,
            agent_id: String::new(),
            channel: Channel::Chat,
            input: String::new(),
            proposed_output: String::new(),
            domain: None,
            policies: Vec::new(),
            context: serde_json::Value::Null,
            trace_id: None,
            redaction: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RedactionMode {
    SdkLocal,
    CustomerService,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RedactionStatus {
    NotRequested,
    Applied,
    Failed,
    RejectedRawSensitiveData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedactedEntity {
    pub entity_type: String,
    pub token: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedactionInfo {
    pub mode: RedactionMode,
    pub status: RedactionStatus,
    pub entities: Vec<RedactedEntity>,
    pub input_redacted: bool,
    pub proposed_output_redacted: bool,
    pub context_redacted: bool,
}

impl RedactionInfo {
    /// Reject states that the wire shape allows but the contract forbids.
    /// Only `Applied` may carry redacted entities or claim that a field was
    /// touched; any other status describing entity output is a misreport.
    pub fn validate(&self) -> Result<(), &'static str> {
        let claims_effect = !self.entities.is_empty()
            || self.input_redacted
            || self.proposed_output_redacted
            || self.context_redacted;
        match self.status {
            RedactionStatus::Applied => Ok(()),
            RedactionStatus::NotRequested
            | RedactionStatus::Failed
            | RedactionStatus::RejectedRawSensitiveData
                if claims_effect =>
            {
                Err("redaction status does not permit entities or redacted fields")
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TriggeredPolicy {
    pub id: String,
    pub severity: Severity,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct Decision {
    pub trace_id: String,
    pub verdict: Verdict,
    pub reason: String,
    pub triggered_policies: Vec<TriggeredPolicy>,
    pub safe_output: Option<String>,
    pub latency_ms: u64,
    /// Per-tier breakdown produced by the parallel-cancel orchestrator.
    /// Empty for callers that only ran the synchronous `Engine::check`
    /// path; populated when `Engine::check_async` is used.
    #[serde(default)]
    pub tier_results: Vec<TierResult>,
    #[serde(default)]
    pub redaction: Option<RedactionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TraceSummary {
    pub trace_id: String,
    pub run_id: Option<String>,
    pub run_event_id: Option<String>,
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
#[serde(rename_all = "lowercase")]
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
    pub agent_id: String,
    pub kind: RunKind,
    pub status: RunStatus,
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
pub struct RunDetail {
    pub run: RunSummary,
    pub events: Vec<RunEventSummary>,
    pub traces: Vec<TraceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct DashboardApiKey {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub status: String,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub last_used_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiKeyListResponse {
    pub api_keys: Vec<DashboardApiKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiKeyBatchRevokeRequest {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiKeyBatchRevokeResponse {
    pub api_keys: Vec<DashboardApiKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateApiKeyResponse {
    pub api_key: DashboardApiKey,
    /// Full bearer key. Returned only once at creation time.
    pub plaintext_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct WorkspaceSettings {
    pub default_action: String,
    pub escalation_webhook_url: Option<String>,
    pub telemetry_enabled: bool,
    pub retention_days: String,
    #[serde(default)]
    pub data_handling_mode: DataHandlingMode,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub config: serde_json::Value,
    /// RFC 3339 timestamp.
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum DataHandlingMode {
    #[default]
    RawAllowed,
    RedactedOnly,
    NoBodyRetention,
    PrivateDeployment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum DashboardKnowledgeSourceKind {
    Url,
    File,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum KnowledgeSourceStatus {
    Draft,
    Indexing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct KnowledgeFileInput {
    pub file_name: String,
    pub media_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct KnowledgeFileMetadata {
    pub file_name: String,
    pub media_type: String,
    pub byte_size: i32,
    pub checksum_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct KnowledgeSourceDocument {
    pub id: String,
    pub title: String,
    pub kind: DashboardKnowledgeSourceKind,
    pub location: Option<String>,
    pub status: KnowledgeSourceStatus,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub metadata: serde_json::Value,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
    /// RFC 3339 timestamp.
    pub last_indexed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct KnowledgeSourceListResponse {
    pub knowledge_sources: Vec<KnowledgeSourceDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateKnowledgeSourceRequest {
    pub title: String,
    pub kind: DashboardKnowledgeSourceKind,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub file: Option<KnowledgeFileInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct KnowledgeSourceFileResponse {
    pub file_name: String,
    pub media_type: String,
    pub byte_size: i32,
    pub data_base64: String,
}

impl Decision {
    pub fn allow(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            verdict: Verdict::Allow,
            reason: "no policies triggered".into(),
            triggered_policies: vec![],
            safe_output: None,
            latency_ms: 0,
            tier_results: vec![],
            redaction: None,
        }
    }
}

/// Generate a fresh trace id. UUIDv7 is time-ordered so callers (and
/// the storage layer's daily-partitioned tables) get cheap chronological
/// scans without a separate sequence.
pub fn new_trace_id() -> String {
    Uuid::now_v7().to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum TlError {
    #[error("policy compile error: {0}")]
    PolicyCompile(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Canonical error envelope returned on non-2xx responses from every
/// TrustLoopGuard endpoint. SDKs deserialize this body to produce typed
/// errors; integrators don't have to inspect status codes by hand.
///
/// The shape is intentionally minimal — `code` drives SDK-side fan-out,
/// `message` is for logs, `retriable` tells callers whether the same
/// request may be retried, and `details` is opaque so the server can
/// add validation field paths without breaking SDKs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    /// Whether the caller may retry the same request without modification.
    /// SDKs honor `Retry-After` when present in addition to this flag.
    pub retriable: bool,
    /// Opaque structured details (e.g. validation field path).
    /// Defaults to `null`; servers may add fields without breaking SDKs.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub details: serde_json::Value,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

/// Stable error code dictionary. Add variants here when introducing new
/// failure modes; never repurpose an existing variant — SDK callers may
/// be branching on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ApiErrorCode {
    /// 400 — request malformed or failed validation.
    Invalid,
    /// 401 — missing or invalid credentials.
    Unauthorized,
    /// 403 — credentials valid but caller lacks permission.
    Forbidden,
    /// 404 — referenced resource not found.
    NotFound,
    /// 410 — API version retired; caller must upgrade.
    Gone,
    /// 422 — well-formed but semantically rejected.
    Unprocessable,
    /// 429 — rate limited. Honor `Retry-After` header when present.
    RateLimited,
    /// 500 — server-side bug; not retriable without server fix.
    Internal,
    /// 502 / 503 / 504 — transient infra issue; retriable with backoff.
    Unavailable,
}

impl ApiErrorCode {
    /// Map an HTTP status code to the canonical error code. Used by SDKs
    /// when the server returned a body that doesn't match `ApiError` —
    /// gives us a fallback that's still useful to integrators.
    pub fn from_http_status(status: u16) -> Self {
        match status {
            400 => Self::Invalid,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            410 => Self::Gone,
            422 => Self::Unprocessable,
            429 => Self::RateLimited,
            500..=501 => Self::Internal,
            502..=504 => Self::Unavailable,
            _ if (500..600).contains(&status) => Self::Internal,
            _ => Self::Invalid,
        }
    }

    /// Default retriable flag for this code. The server may override via
    /// `ApiError.retriable`; this is only used when synthesizing an
    /// `ApiError` from a raw HTTP status.
    pub fn default_retriable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_helper_sets_verdict() {
        let d = Decision::allow("t-1");
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.trace_id, "t-1");
        assert!(d.tier_results.is_empty());
    }

    #[test]
    fn pre_v0_check_request_still_deserializes() {
        // Pre-PR-1 wire shape: no `domain` field. Must still parse so
        // existing SDKs and replay fixtures don't break.
        let json = r#"{
            "agent_id": "a",
            "channel": "chat",
            "input": "hi",
            "proposed_output": "hello"
        }"#;
        let req: CheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.agent_id, "a");
        assert!(req.workspace_id.is_none());
        assert!(req.domain.is_none());
        assert!(req.policies.is_empty());
    }

    #[test]
    fn check_request_supports_struct_update_defaults() {
        let req = CheckRequest {
            agent_id: "a".into(),
            channel: Channel::Chat,
            input: "hi".into(),
            proposed_output: "hello".into(),
            ..CheckRequest::default()
        };

        assert!(req.workspace_id.is_none());
        assert!(req.run_id.is_none());
        assert!(req.run_event_id.is_none());
        assert!(req.run_event.is_none());
        assert!(req.policies.is_empty());
        assert!(req.context.is_null());
    }

    #[test]
    fn check_request_and_decision_carry_redaction_metadata_without_raw_values() {
        let metadata = RedactionInfo {
            mode: RedactionMode::SdkLocal,
            status: RedactionStatus::Applied,
            entities: vec![RedactedEntity {
                entity_type: "EMAIL".into(),
                token: "[EMAIL_1]".into(),
                count: 1,
            }],
            input_redacted: true,
            proposed_output_redacted: true,
            context_redacted: false,
        };

        let req = CheckRequest {
            agent_id: "a".into(),
            channel: Channel::Chat,
            input: "email [EMAIL_1]".into(),
            proposed_output: "reply to [EMAIL_1]".into(),
            redaction: Some(metadata.clone()),
            ..CheckRequest::default()
        };
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("\"redaction\""));
        assert!(serialized.contains("\"mode\":\"sdk_local\""));
        assert!(!serialized.contains("alice@example.com"));

        let mut decision = Decision::allow("t-1");
        decision.redaction = req.redaction.clone();
        assert_eq!(
            decision.redaction.as_ref().unwrap().entities[0].token,
            "[EMAIL_1]"
        );
    }

    #[test]
    fn api_error_round_trip() {
        let body = r#"{"code":"rate_limited","message":"too many requests","retriable":true}"#;
        let parsed: ApiError = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.code, ApiErrorCode::RateLimited);
        assert!(parsed.retriable);
        assert!(parsed.details.is_null());
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert!(serialized.contains("\"code\":\"rate_limited\""));
    }

    #[test]
    fn api_error_code_status_fallback() {
        assert_eq!(
            ApiErrorCode::from_http_status(429),
            ApiErrorCode::RateLimited
        );
        assert_eq!(
            ApiErrorCode::from_http_status(503),
            ApiErrorCode::Unavailable
        );
        assert_eq!(
            ApiErrorCode::from_http_status(401),
            ApiErrorCode::Unauthorized
        );
        assert_eq!(ApiErrorCode::from_http_status(599), ApiErrorCode::Internal);
        assert!(ApiErrorCode::RateLimited.default_retriable());
        assert!(ApiErrorCode::Unavailable.default_retriable());
        assert!(!ApiErrorCode::Invalid.default_retriable());
        assert!(!ApiErrorCode::Internal.default_retriable());
    }

    #[test]
    fn pre_v0_decision_still_deserializes() {
        let json = r#"{
            "trace_id": "t-1",
            "verdict": "allow",
            "reason": "ok",
            "triggered_policies": [],
            "safe_output": null,
            "latency_ms": 1
        }"#;
        let d: Decision = serde_json::from_str(json).unwrap();
        assert_eq!(d.verdict, Verdict::Allow);
        assert!(d.tier_results.is_empty());
    }

    #[test]
    fn redaction_info_validate_accepts_applied_with_or_without_effect() {
        // `Applied` covers both empty (nothing matched) and populated
        // outcomes; that ambiguity is by design — the redactor ran.
        let empty = RedactionInfo {
            mode: RedactionMode::Server,
            status: RedactionStatus::Applied,
            entities: vec![],
            input_redacted: false,
            proposed_output_redacted: false,
            context_redacted: false,
        };
        assert!(empty.validate().is_ok());

        let populated = RedactionInfo {
            mode: RedactionMode::SdkLocal,
            status: RedactionStatus::Applied,
            entities: vec![RedactedEntity {
                entity_type: "EMAIL".into(),
                token: "[EMAIL_1]".into(),
                count: 1,
            }],
            input_redacted: true,
            proposed_output_redacted: false,
            context_redacted: false,
        };
        assert!(populated.validate().is_ok());
    }

    #[test]
    fn redaction_info_validate_rejects_non_applied_claiming_effect() {
        let cases = [
            RedactionStatus::NotRequested,
            RedactionStatus::Failed,
            RedactionStatus::RejectedRawSensitiveData,
        ];
        for status in cases {
            let with_entities = RedactionInfo {
                mode: RedactionMode::SdkLocal,
                status,
                entities: vec![RedactedEntity {
                    entity_type: "EMAIL".into(),
                    token: "[EMAIL_1]".into(),
                    count: 1,
                }],
                input_redacted: false,
                proposed_output_redacted: false,
                context_redacted: false,
            };
            assert!(
                with_entities.validate().is_err(),
                "{status:?} with entities must fail"
            );

            let with_redacted_flag = RedactionInfo {
                mode: RedactionMode::SdkLocal,
                status,
                entities: vec![],
                input_redacted: true,
                proposed_output_redacted: false,
                context_redacted: false,
            };
            assert!(
                with_redacted_flag.validate().is_err(),
                "{status:?} with input_redacted must fail"
            );
        }
    }

    #[test]
    fn policy_validate_response_is_core_wire_contract() {
        let response = PolicyValidateResponse {
            valid: false,
            policy_id: Some("refund-guarantee".into()),
            errors: vec![PolicyValidationIssue {
                path: "match.regex".into(),
                message: "regex failed to compile".into(),
            }],
        };

        let body = serde_json::to_value(&response).unwrap();
        assert_eq!(body["valid"], false);
        assert_eq!(body["policy_id"], "refund-guarantee");
        assert_eq!(body["errors"][0]["path"], "match.regex");
    }
}
