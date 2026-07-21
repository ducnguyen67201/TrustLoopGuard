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

pub mod agent;
pub mod analytics;
pub mod auth;
pub mod authorization;
pub mod budget_alert;
pub mod dashboard;
pub mod enforcement;
pub mod error;
pub mod event;
pub mod financial;
pub mod gateway;
pub mod github_integration;
pub mod guard;
pub mod human_review;
pub mod knowledge;
pub mod label;
pub mod label_policy;
pub mod llm_pricing;
pub mod llm_usage;
pub mod mcp_gateway;
pub mod policy;
pub mod provenance;
pub mod redteam;
pub mod redteam_runner;
pub mod run;
pub mod team;
pub mod tier;
pub mod tool;
pub mod trace;

pub use agent::{
    AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone, KnowledgeSource,
    KnowledgeSourceKind, WorkflowDefinition, WorkflowRequirement,
};
pub use analytics::{
    AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView,
    AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDashboardWidget,
    AnalyticsDimension, AnalyticsFacet, AnalyticsFacetCatalogResponse, AnalyticsFilter,
    AnalyticsMetric, AnalyticsQueryPoint, AnalyticsQueryRequest, AnalyticsQueryResponse,
    AnalyticsWidgetLayout, CreateAnalyticsDashboardViewRequest,
    UpdateAnalyticsDashboardViewRequest,
};
pub use auth::{AuthRequest, AuthResponse, ChangePasswordRequest, OAuthIdentityRequest};
pub use authorization::{
    ActionGrantScope, ApprovalDecision, ApprovalEnvelope, ApprovalStatus, AuthorityRequirement,
    AuthorizationApproval, AuthorizationApprovalListResponse, AuthorizationApprovalSummary,
    AuthorizationCapabilityId, AuthorizationClaim, AuthorizationDecision, AuthorizationDomain,
    AuthorizationDomainEvidence, AuthorizationEffect, AuthorizationFinding, AuthorizationGrant,
    AuthorizationGrantListResponse, AuthorizationGrantRef, AuthorizationGrantScope,
    AuthorizationGrantSource, AuthorizationIntentStatus, AuthorizationLease, AuthorizationReceipt,
    AuthorizationReceiptListResponse, AuthorizationSubject, CompleteAuthorizationLeaseRequest,
    CreateAuthorizationGrantRequest, DecideAuthorizationApprovalRequest,
    DecideAuthorizationApprovalResponse, FinancialExecutionStatus, FinancialGrantScope, GrantMode,
    GrantStatus, LeaseStatus,
};
pub use budget_alert::{
    BudgetAlertConfig, BudgetAlertConfigListResponse, BudgetAlertFiring,
    BudgetAlertFiringListResponse, BudgetAlertThresholdType, BudgetAlertWindow,
    CreateBudgetAlertConfigRequest, UpdateBudgetAlertConfigRequest,
};
pub use dashboard::{
    ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse, CreateApiKeyRequest,
    CreateApiKeyResponse, CreateWorkspaceEnvironmentRequest, DashboardApiKey, DataHandlingMode,
    EnvironmentCheckerModes, UpdateEnvironmentCheckerModesRequest,
    UpdateWorkspaceEnvironmentRequest, UpdateWorkspaceSettingsRequest, WorkspaceEnvironment,
    WorkspaceEnvironmentListResponse, WorkspaceSettings,
};
pub use enforcement::{CheckerFindingEvidence, CheckerRun, EnforcementMode, SignalEvidence};
pub use error::{ApiError, ApiErrorCode, TlError};
pub use event::{
    Action, EventKind, GuardEvent, Principal, ShellActionParameters, ShellLanguage, SideEffectClass,
};
pub use financial::{
    AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentReservation,
    AgenticPaymentReservationStatus, AgenticPaymentRollbackRequest, CounterpartyRef,
    CreateFinancialActionRequest, CreateFinancialPolicyRequest, EvidenceRef,
    ExecuteFinancialActionRequest, FinancialAction, FinancialActionKind,
    FinancialActionListResponse, FinancialActionOutcome, FinancialActionOutcomeStatus,
    FinancialActionPrecondition, FinancialActionRecord, FinancialActionState,
    FinancialEligibilityCheck, FinancialEligibilityResult, FinancialEligibilityStatus,
    FinancialOutcomeListResponse, FinancialPolicyListResponse, FinancialPolicyRecord,
    FinancialPolicySelector, FinancialRail, FinancialReceipt, MoneyAmount, RecoveryStatus,
    ReversalCapability, SpendMeter, X402NormalizedPaymentRequirement, X402PaymentRequirement,
    X402SettlementProof, USD,
};
pub use gateway::{
    CreateGatewayProviderConnectionRequest, CreateGatewayRouteRequest, GatewayCredentialStatus,
    GatewayProviderConnection, GatewayProviderConnectionListResponse, GatewayProviderKind,
    GatewayRoute, GatewayRouteListResponse, UpdateGatewayProviderConnectionRequest,
    UpdateGatewayRouteRequest,
};
pub use github_integration::{
    GitHubCallbackRequest, GitHubCallbackResponse, GitHubConnectionCreateRequest,
    GitHubConnectionListResponse, GitHubConnectionStatus, GitHubConnectionSummary,
    GitHubInstallUrlRequest, GitHubInstallUrlResponse, GitHubInstallationStatus,
    GitHubInstallationSummary, GitHubIntegrationAnalysisSummary, GitHubIntegrationApproveResponse,
    GitHubIntegrationCancelResponse, GitHubIntegrationJobCreateRequest,
    GitHubIntegrationJobListResponse, GitHubIntegrationJobStatus, GitHubIntegrationJobSummary,
    GitHubIntegrationManualStep, GitHubProposedFileChange, GitHubProposedFileOperation,
    GitHubRepositoryListResponse, GitHubRepositorySelection, GitHubRepositorySummary,
    GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1,
};
pub use guard::{
    Channel, CheckRequest, Decision, RedactedEntity, RedactionInfo, RedactionMode, RedactionStatus,
    Severity, TriggeredPolicy,
};
pub use human_review::{
    CreateHumanReviewEventRequest, HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary,
    HumanReviewEvent, HumanReviewEventListResponse, HumanReviewGroupRow, HumanReviewOutcome,
    HumanReviewOutcomeCounts, HumanReviewPolicyRow, HumanReviewReasonRow,
    HumanReviewWorkflowStepRow,
};
pub use knowledge::{
    CreateKnowledgeSourceRequest, DashboardKnowledgeSourceKind, KnowledgeFileInput,
    KnowledgeFileMetadata, KnowledgeSourceDocument, KnowledgeSourceFileResponse,
    KnowledgeSourceListResponse, KnowledgeSourceStatus,
};
pub use label::{
    Confidentiality, Integrity, LabelBasis, LabelBasisSet, LabelPolicyStatus, LabelResolution,
    Labels, Origin, Source, SourceLabelEvidence, Trust,
};
pub use label_policy::{
    SourceLabelPolicy, SourceLabelPolicyEntry, SourceLabelPolicyListResponse,
    UpsertSourceLabelPolicyRequest,
};
pub use llm_pricing::{
    LlmModelPrice, LlmPriceSource, LlmPricingListResponse, UpsertLlmModelPriceRequest,
};
pub use llm_usage::{
    LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageKind, LlmUsageListResponse,
    LlmUsageResponse,
};
pub use mcp_gateway::{
    CreateMcpGatewayConnectionRequest, McpGatewayAuthKind, McpGatewayCatalogStatus,
    McpGatewayConnectInfo, McpGatewayConnection, McpGatewayConnectionListResponse,
    McpGatewayCredentialStatus, McpGatewaySyncResponse, McpGatewaySyncStatus, McpGatewayTool,
    McpGatewayToolAssignment, McpGatewayToolAssignmentsResponse, McpGatewayToolListResponse,
    ReplaceMcpGatewayToolAssignmentsRequest, UpdateMcpGatewayConnectionRequest,
    UpdateMcpGatewayToolRequest,
};
pub use policy::{
    AiEditRequest, AiEditResponse, EntityVersionDetail, EntityVersionListResponse,
    EntityVersionSummary, GuardrailGenerateResponse, GuardrailListResponse,
    PolicyBatchSetEnabledRequest, PolicyBatchSetEnabledResponse, PolicyDocument, PolicyDraft,
    PolicyDraftRequest, PolicyDraftResponse, PolicyFamily, PolicyListResponse, PolicyMatchType,
    PolicySetEnabledRequest, PolicySummary, PolicyValidateResponse, PolicyValidationIssue,
};
pub use provenance::ProvenanceMap;
pub use redteam::{
    AttackVector, ComparedAttackStatus, CreateReportRequest, HardenCandidate,
    HardenCandidateOperation, HardenRejection, HardenRejectionReason, HardenRequest,
    HardenResponse, JobStatus, RedteamAttackRecord, RedteamAttackRecordListResponse,
    RedteamAttackSession, RedteamAttackSurface, RedteamComparedAttack, RedteamDispatchRequest,
    RedteamDocumentTemplate, RedteamJobDetail, RedteamJobListResponse, RedteamJobSummary,
    RedteamPlanListResponse, RedteamPlanRequest, RedteamPlanResponse, RedteamReportAggregates,
    RedteamReportComparison, RedteamReportFinding, RedteamReportPayload, RedteamReportShare,
    RedteamRunMode, RedteamSessionEvent, ReportSeverity, VerifyResult, WorkflowPath,
};
pub use redteam_runner::{
    RunnerAttackSession, RunnerAttackSurface, RunnerAttackVector, RunnerDispatch,
    RunnerDocumentTemplate, RunnerHandle, RunnerPlanRequest, RunnerPlanResponse, RunnerReport,
    RunnerRunMode, RunnerSessionEvent, RunnerStatus,
};
pub use run::{
    CreateRunEventRequest, CreateRunRequest, RunBudgetWindowSnapshot, RunDetail, RunEventKind,
    RunEventListResponse, RunEventSummary, RunGuardrailUsage, RunKind, RunListResponse,
    RunLlmBudgetDecision, RunProviderUsage, RunStatus, RunSummary, UpdateRunRequest,
};
pub use team::{
    CreateInviteRequest, CreateInviteResponse, CreateWorkspaceRequest, InviteListResponse,
    InviteStatus, MemberListResponse, MyWorkspace, MyWorkspacesResponse, WorkspaceInvite,
    WorkspaceMember, WorkspaceRole,
};
pub use tier::{Tier, TierResult, TierStatus};
pub use tool::{
    AllowedSource, ApprovalRule, LimitAction, ParamLimit, ParamRole, ParamSpec, ToolIdentity,
    ToolMetadata, ToolMetadataEntry, ToolMetadataListResponse, ToolResolution,
    UpsertToolMetadataRequest,
};
pub use trace::{new_trace_id, TraceListResponse, TraceSummary};

/// Historical workspace id used by migrations and explicit test fixtures.
/// Request handling must resolve workspace context from auth/session/API-key
/// state and must not fall back to this value when context is missing.
pub const DEFAULT_WORKSPACE_ID: &str = "default";

/// Backwards-compatible production environment used for existing runtime
/// data and for internal/admin calls that do not select an environment yet.
pub const DEFAULT_ENVIRONMENT_ID: &str = "production";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_worst_with_uses_canonical_precedence() {
        use AuthorizationEffect::{Defer, Deny, Permit, RequireApproval, Transform};
        let ordered = [Permit, Transform, RequireApproval, Defer, Deny];
        for (weaker_rank, weaker) in ordered.iter().enumerate() {
            for stronger in &ordered[weaker_rank..] {
                assert_eq!(weaker.worst_with(*stronger), *stronger);
                assert_eq!(stronger.worst_with(*weaker), *stronger);
            }
        }
    }

    #[test]
    fn allow_helper_sets_permit_effect() {
        let d = Decision::allow("t-1");
        assert_eq!(d.effect, AuthorizationEffect::Permit);
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
    fn decision_allow_omits_empty_event_evidence() {
        let serialized = serde_json::to_string(&Decision::allow("t-1")).unwrap();
        assert!(!serialized.contains("violated_rule"));
        assert!(!serialized.contains("remediation"));
        assert!(!serialized.contains("source_chain"));
        assert!(!serialized.contains("risk_source"));
        assert!(!serialized.contains("risk_code"));
        assert!(!serialized.contains("harm_class"));
        assert!(!serialized.contains("constraints"));
    }

    #[test]
    fn event_kind_serializes_to_dotted_taxonomy() {
        let serialized = serde_json::to_string(&EventKind::OutputProposed).unwrap();
        assert_eq!(serialized, r#""output.proposed""#);

        let parsed: EventKind = serde_json::from_str(r#""tool.call.proposed""#).unwrap();
        assert_eq!(parsed, EventKind::ToolCallProposed);
    }

    #[test]
    fn guard_event_defaults_optional_collections() {
        let json = r#"{
            "kind": "output.proposed",
            "principal": {
                "workspace_id": "ws_1",
                "environment_id": "production",
                "agent_id": "a"
            },
            "action": {
                "operation": "output",
                "parameters": { "text": "hello" }
            }
        }"#;
        let event: GuardEvent = serde_json::from_str(json).unwrap();
        assert!(event.sources.is_empty());
        assert!(event.provenance.is_empty());
        assert!(event.context.is_null());
        assert!(event.action.side_effect.is_none());
        assert!(event.checks.is_empty());
        assert!(event.signals.is_empty());

        // Empty evidence collections stay off the wire.
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.contains("checks"));
        assert!(!serialized.contains("signals"));
    }

    #[test]
    fn labels_default_to_unknown_values() {
        let labels = Labels::default();
        assert_eq!(labels.trust, Trust::Unknown);
        assert_eq!(labels.confidentiality, Confidentiality::Unknown);
        assert_eq!(labels.integrity, Integrity::Unknown);
    }

    #[test]
    fn provenance_map_defaults_to_empty_object() {
        let serialized = serde_json::to_string(&ProvenanceMap::default()).unwrap();
        assert_eq!(serialized, "{}");
    }

    #[test]
    fn tool_metadata_omits_absent_optional_fields() {
        let metadata = ToolMetadata {
            tool: "send_email".into(),
            side_effect: SideEffectClass::ExternalCommunication,
            reversible: false,
            params: vec![],
            approval: None,
            sandbox_hint: None,
        };

        let serialized = serde_json::to_string(&metadata).unwrap();
        assert!(!serialized.contains("approval"));
        assert!(!serialized.contains("sandbox_hint"));
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
