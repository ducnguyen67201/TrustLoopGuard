//! Post-run evaluation and run-finalization API contracts.
//!
//! These are wire types only. Evaluation policy parsing lives in `tl-policy`,
//! orchestration in `tl-server`, and durable state in `tl-storage`.

use serde::{Deserialize, Serialize};

use crate::{PolicyFamily, RunStatus, RunSummary, Severity};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[cfg_attr(feature = "schema", derive(JsonSchema))]
        #[cfg_attr(feature = "openapi", derive(ToSchema))]
        #[cfg_attr(feature = "ts-export", derive(TS))]
        #[cfg_attr(feature = "ts-export", ts(export))]
        pub enum $name { $($variant),+ }
    };
}

wire_enum!(RunBoundarySource {
    ExplicitSdk,
    FrameworkAdapter,
    OtelSessionEnd,
    RootSpanEnd,
    IdleTimeout,
    MaxDuration,
    Admin,
    LegacySdk,
});

wire_enum!(BoundaryConfidence {
    Authoritative,
    Strong,
    Inferred,
});

impl RunBoundarySource {
    pub const fn confidence(self) -> BoundaryConfidence {
        match self {
            Self::ExplicitSdk | Self::Admin => BoundaryConfidence::Authoritative,
            Self::FrameworkAdapter | Self::OtelSessionEnd | Self::LegacySdk => {
                BoundaryConfidence::Strong
            }
            Self::RootSpanEnd | Self::IdleTimeout | Self::MaxDuration => {
                BoundaryConfidence::Inferred
            }
        }
    }
}

wire_enum!(RunCaptureStatus {
    Open,
    Waiting,
    Complete,
    Incomplete,
});

wire_enum!(CaptureMode {
    BestEffort,
    Durable,
});

wire_enum!(ContentCaptureMode {
    MetadataOnly,
    Redacted,
    EncryptedArtifactRef,
});

wire_enum!(MissingEvidenceBehavior { Inconclusive, Fail });

wire_enum!(EvaluationJobStatus {
    WaitingCapture,
    Queued,
    Running,
    Completed,
    Failed,
    Inconclusive,
    Error,
});

wire_enum!(EvaluationVerdict {
    Passed,
    Failed,
    Inconclusive,
    Error,
    NotConfigured,
});

wire_enum!(EvaluationFindingStatus {
    Passed,
    Failed,
    Inconclusive,
    Error,
    NotApplicable,
});

wire_enum!(RunParticipantRole {
    Primary,
    Participant
});

wire_enum!(EvaluationCaseScoringMode {
    Trajectory,
    Endstate
});

wire_enum!(EvaluationCaseStatus {
    Pending,
    Completed,
    Skipped,
    Error,
});

wire_enum!(EvaluationReleaseGateVerdict {
    Passed,
    Failed,
    InsufficientEvidence,
});

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentEvaluationProfile {
    pub workspace_id: String,
    pub environment_id: String,
    pub agent_id: String,
    pub enabled: bool,
    pub capture_mode: CaptureMode,
    pub content_mode: ContentCaptureMode,
    pub quiet_period_ms: u64,
    pub max_capture_wait_ms: u64,
    pub on_incomplete: MissingEvidenceBehavior,
    pub profile_version: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PutAgentEvaluationProfileRequest {
    pub enabled: bool,
    pub capture_mode: CaptureMode,
    pub content_mode: ContentCaptureMode,
    pub quiet_period_ms: u64,
    pub max_capture_wait_ms: u64,
    pub on_incomplete: MissingEvidenceBehavior,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub expected_profile_version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentEvaluationPolicyAssignment {
    pub policy_id: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub policy_version: Option<i32>,
    pub weight: u32,
    pub critical: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentEvaluationPolicyAssignmentListResponse {
    pub agent_id: String,
    pub environment_id: String,
    pub assignments: Vec<AgentEvaluationPolicyAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PutAgentEvaluationPolicyAssignmentsRequest {
    pub assignments: Vec<AgentEvaluationPolicyAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunParticipantSummary {
    pub agent_id: String,
    pub role: RunParticipantRole,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunEvaluationPolicyManifestSummary {
    pub agent_id: String,
    pub policy_id: String,
    pub policy_family: PolicyFamily,
    pub policy_version: i32,
    pub policy_hash: String,
    pub weight: u32,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct FinalizeRunRequest {
    pub status: RunStatus,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub ended_at: Option<String>,
    pub boundary_source: RunBoundarySource,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub expected_flush_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub last_event_sequence: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RunFinalizationSummary {
    pub finalized_at: String,
    pub boundary_source: RunBoundarySource,
    pub boundary_confidence: BoundaryConfidence,
    pub capture_status: RunCaptureStatus,
    pub capture_deadline: String,
    pub expected_flush_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct FinalizeRunResponse {
    pub run: RunSummary,
    pub finalization: RunFinalizationSummary,
    pub evaluation_status: EvaluationJobStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationEvidenceRef {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationFinding {
    pub policy_id: String,
    pub policy_version: i32,
    pub agent_id: String,
    pub severity: Severity,
    pub critical: bool,
    pub status: EvaluationFindingStatus,
    /// Integer basis points in the inclusive range 0..=10_000.
    pub score_bps: Option<u32>,
    pub reason: String,
    pub evidence: Vec<EvaluationEvidenceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationResultSummary {
    pub id: String,
    pub run_id: String,
    pub agent_id: String,
    pub snapshot_hash: String,
    pub manifest_hash: String,
    pub evaluator_version: String,
    pub verdict: EvaluationVerdict,
    pub score_bps: Option<u32>,
    pub capture_status: RunCaptureStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional, type = "Record<string, unknown>"))]
    pub llm_audit: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationJobSummary {
    pub id: String,
    pub run_id: String,
    pub agent_id: String,
    pub status: EvaluationJobStatus,
    pub attempts: i32,
    pub error: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationResultDetail {
    pub result: EvaluationResultSummary,
    pub findings: Vec<EvaluationFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationResultListResponse {
    pub jobs: Vec<EvaluationJobSummary>,
    pub results: Vec<EvaluationResultDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ReevaluateRunRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ReevaluateRunResponse {
    pub run_id: String,
    pub status: EvaluationJobStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationCaseBudget {
    pub max_turns: u32,
    pub max_tool_calls: u32,
    pub max_tokens: u64,
    pub max_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationCaseSpec {
    pub case_id: String,
    pub case_hash: String,
    pub input_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub reference_hash: Option<String>,
    pub scoring_mode: EvaluationCaseScoringMode,
    pub weight: u32,
    pub critical: bool,
    pub budget: EvaluationCaseBudget,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub oracle_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationDatasetVersion {
    pub dataset_id: String,
    pub version: i32,
    pub manifest_hash: String,
    pub cases: Vec<EvaluationCaseSpec>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationCampaignCaseResult {
    pub case_id: String,
    pub run_id: String,
    pub status: EvaluationCaseStatus,
    pub verdict: EvaluationVerdict,
    pub score_bps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationCampaignAggregate {
    pub verdict: EvaluationVerdict,
    pub score_bps: Option<u32>,
    pub completed_cases: u32,
    pub skipped_cases: u32,
    pub error_cases: u32,
    pub cases: Vec<EvaluationCampaignCaseResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EvaluationReleaseGate {
    pub agent_id: String,
    pub environment_id: String,
    pub manifest_hash: String,
    pub verdict: EvaluationReleaseGateVerdict,
    pub evidence_result_ids: Vec<String>,
    pub created_at: String,
}
