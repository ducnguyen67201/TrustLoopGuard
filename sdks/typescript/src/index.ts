// Public surface of the TrustLoopGuard TypeScript SDK.
// Type definitions are generated from Rust by `cargo run -p tl-codegen`.
// See README.md in src/generated for regen instructions.

export * from './generated/Decision';
export * from './generated/Verdict';
export * from './generated/Channel';
export * from './generated/Severity';
export * from './generated/TriggeredPolicy';
export * from './generated/AgentAuthority';
export * from './generated/AgentListResponse';
export * from './generated/AgentProfile';
export * from './generated/WorkflowDefinition';
export * from './generated/WorkflowRequirement';
export * from './generated/AgentScope';
export * from './generated/AgentTone';
export * from './generated/KnowledgeSource';
export * from './generated/CreateKnowledgeSourceRequest';
export * from './generated/DashboardKnowledgeSourceKind';
export * from './generated/KnowledgeFileInput';
export * from './generated/KnowledgeFileMetadata';
export * from './generated/KnowledgeSourceDocument';
export * from './generated/KnowledgeSourceFileResponse';
export * from './generated/KnowledgeSourceListResponse';
export * from './generated/KnowledgeSourceStatus';
export * from './generated/ApiError';
export * from './generated/ApiErrorCode';
export * from './generated/PolicyDocument';
export * from './generated/PolicyFamily';
export * from './generated/PolicyBatchSetEnabledRequest';
export * from './generated/PolicyBatchSetEnabledResponse';
export * from './generated/PolicyListResponse';
export * from './generated/PolicySetEnabledRequest';
export * from './generated/PolicySummary';
export * from './generated/PolicyValidateResponse';
export * from './generated/PolicyValidationIssue';
export * from './generated/PolicyAction';
export * from './generated/PolicyMatchType';
export * from './generated/PolicyDraft';
export * from './generated/PolicyDraftRequest';
export * from './generated/PolicyDraftResponse';
export * from './generated/GuardrailGenerateResponse';
export * from './generated/GuardrailListResponse';
export * from './generated/ApiKeyBatchRevokeRequest';
export * from './generated/ApiKeyBatchRevokeResponse';
export * from './generated/ApiKeyListResponse';
export * from './generated/CreateApiKeyRequest';
export * from './generated/CreateApiKeyResponse';
export * from './generated/DashboardApiKey';
export * from './generated/WorkspaceSettings';
export * from './generated/TraceListResponse';
export * from './generated/TraceSummary';
export * from './generated/CreateRunEventRequest';
export * from './generated/CreateRunRequest';
export * from './generated/ApprovalRequirement';
export * from './generated/AgenticPaymentAuthorizationResponse';
export * from './generated/AgenticPaymentAuthorizeRequest';
export * from './generated/AgenticPaymentCommitRequest';
export * from './generated/AgenticPaymentDecision';
export * from './generated/AgenticPaymentMandateScope';
export * from './generated/AgenticPaymentRecord';
export * from './generated/AgenticPaymentReservation';
export * from './generated/AgenticPaymentReservationStatus';
export * from './generated/AgenticPaymentRollbackRequest';
export * from './generated/CounterpartyRef';
export * from './generated/CreateFinancialActionRequest';
export * from './generated/CreateFinancialMandateRequest';
export * from './generated/CreateFinancialPolicyRequest';
export * from './generated/EvidenceRef';
export * from './generated/FinancialAction';
export * from './generated/FinancialActionDecision';
export * from './generated/FinancialActionDecisionReceipt';
export * from './generated/FinancialActionKind';
export * from './generated/FinancialActionListResponse';
export * from './generated/FinancialActionOutcome';
export * from './generated/FinancialActionOutcomeStatus';
export * from './generated/FinancialActionPrecondition';
export * from './generated/FinancialActionRecord';
export * from './generated/FinancialActionStatus';
export * from './generated/FinancialApprovalRequest';
export * from './generated/BudgetAlertConfig';
export * from './generated/BudgetAlertConfigListResponse';
export * from './generated/BudgetAlertFiring';
export * from './generated/BudgetAlertFiringListResponse';
export * from './generated/BudgetAlertThresholdType';
export * from './generated/BudgetAlertWindow';
export * from './generated/CreateBudgetAlertConfigRequest';
export * from './generated/UpdateBudgetAlertConfigRequest';
export * from './generated/FinancialApprovalRequestListResponse';
export * from './generated/FinancialApprovalRequestStatus';
export * from './generated/FinancialAuthorizationScopeProof';
export * from './generated/FinancialDecision';
export * from './generated/FinancialDecisionRisk';
export * from './generated/FinancialDecisionRiskCode';
export * from './generated/FinancialEligibilityCheck';
export * from './generated/FinancialEligibilityResult';
export * from './generated/FinancialEligibilityStatus';
export * from './generated/FinancialEvidenceProof';
export * from './generated/FinancialExecutionProof';
export * from './generated/FinancialExecutionProofStatus';
export * from './generated/FinancialMandate';
export * from './generated/FinancialMandateListResponse';
export * from './generated/FinancialMandateStatus';
export * from './generated/FinancialOutcomeListResponse';
export * from './generated/FinancialPolicyListResponse';
export * from './generated/FinancialPolicyRecord';
export * from './generated/FinancialPolicySelector';
export * from './generated/FinancialRail';
export * from './generated/FinancialReceipt';
export * from './generated/SpendMeter';
export * from './generated/MandateRef';
export * from './generated/MoneyAmount';
export * from './generated/RecoveryStatus';
export * from './generated/ReversalCapability';
export * from './generated/X402NormalizedPaymentRequirement';
export * from './generated/X402PaymentRequirement';
export * from './generated/X402SettlementProof';
export * from './generated/UpdateRunRequest';
export * from './generated/RunDetail';
export * from './generated/RunEventKind';
export * from './generated/RunEventListResponse';
export * from './generated/RunEventSummary';
export * from './generated/RunKind';
export * from './generated/RunListResponse';
export * from './generated/RunStatus';
export * from './generated/RunSummary';
export * from './generated/CreateGatewayProviderConnectionRequest';
export * from './generated/CreateGatewayRouteRequest';
export * from './generated/GatewayCredentialStatus';
export * from './generated/GatewayProviderConnection';
export * from './generated/GatewayProviderConnectionListResponse';
export * from './generated/GatewayProviderKind';
export * from './generated/GatewayRoute';
export * from './generated/GatewayRouteListResponse';
export * from './generated/UpdateGatewayProviderConnectionRequest';
export * from './generated/UpdateGatewayRouteRequest';
export * from './generated/LlmUsageBucket';
export * from './generated/LlmUsageBucketsResponse';
export * from './generated/LlmUsageEvent';
export * from './generated/LlmUsageKind';
export * from './generated/LlmUsageListResponse';
export * from './generated/LlmUsageResponse';
export * from './generated/LlmModelPrice';
export * from './generated/LlmPriceSource';
export * from './generated/LlmPricingListResponse';
export * from './generated/UpsertLlmModelPriceRequest';
export * from './generated/RunProviderUsage';
export * from './generated/RunGuardrailUsage';
export * from './generated/RunBudgetWindowSnapshot';
export * from './generated/RunLlmBudgetDecision';
export * from './generated/Action';
export * from './generated/AllowedSource';
export * from './generated/ApprovalRule';
export * from './generated/CheckerFindingEvidence';
export * from './generated/CheckerRun';
export * from './generated/Confidentiality';
export * from './generated/EnforcementMode';
export * from './generated/EventKind';
export * from './generated/GuardEvent';
export * from './generated/Integrity';
export * from './generated/LabelBasis';
export * from './generated/LabelBasisSet';
export * from './generated/LabelPolicyStatus';
export * from './generated/LabelResolution';
export * from './generated/Labels';
export * from './generated/Origin';
export * from './generated/ParamRole';
export * from './generated/ParamSpec';
export * from './generated/Principal';
export * from './generated/ProvenanceMap';
export * from './generated/SideEffectClass';
export * from './generated/SignalEvidence';
export * from './generated/Source';
export * from './generated/SourceLabelEvidence';
export * from './generated/SourceLabelPolicy';
export * from './generated/SourceLabelPolicyEntry';
export * from './generated/SourceLabelPolicyListResponse';
export * from './generated/ToolMetadata';
export * from './generated/ToolMetadataEntry';
export * from './generated/ToolMetadataListResponse';
export * from './generated/ToolResolution';
export * from './generated/Trust';
export * from './generated/UpsertSourceLabelPolicyRequest';
export * from './generated/UpsertToolMetadataRequest';
export * from './generated/JobStatus';
export * from './generated/RedteamDispatchRequest';
export * from './generated/RedteamAttackSession';
export * from './generated/RedteamSessionEvent';
export * from './generated/RedteamJobSummary';
export * from './generated/RedteamJobDetail';
export * from './generated/RedteamJobListResponse';
export * from './generated/ReportSeverity';
export * from './generated/ComparedAttackStatus';
export * from './generated/RedteamReportFinding';
export * from './generated/RedteamReportAggregates';
export * from './generated/RedteamComparedAttack';
export * from './generated/RedteamReportComparison';
export * from './generated/RedteamReportPayload';
export * from './generated/CreateReportRequest';
export * from './generated/RedteamReportShare';
export * from './generated/RedteamAttackRecord';
export * from './generated/RedteamAttackRecordListResponse';
export * from './generated/HardenRequest';
export * from './generated/HardenResponse';
export * from './generated/HardenCandidate';
export * from './generated/VerifyResult';
export * from './generated/AttackVector';
export * from './generated/WorkflowPath';
export * from './generated/RedteamPlanRequest';
export * from './generated/RedteamPlanResponse';
export * from './generated/RedteamPlanListResponse';

export { Client } from './client';
export type {
  ActiveRun,
  ClientOptions,
  FinancialOperation,
  FinancialOperationRunOptions,
  FinancialOperationSpec,
  GuardToolCallOptions,
  ListTracesOptions,
  WithRunOptions,
} from './client';

export { GuardMode, guard } from './guard';
export type {
  GuardCallbacks,
  GuardOptions,
  GuardFactoryOptions,
  GuardCallOptions,
  GuardStreamCallOptions,
  GuardLogEvent,
  OutputGuard,
  RegenerateFeedback,
} from './guard';

export { DEFAULT_RETRY, nextDelay } from './retry';
export type { RetryConfig } from './retry';

export {
  SdkError,
  Invalid,
  Unauthorized,
  Forbidden,
  NotFound,
  Gone,
  Unprocessable,
  RateLimited,
  Internal,
  Unavailable,
  Transport,
  Decode,
  codeFromHttpStatus,
  synthesizeApiError,
  fromResponse,
  parseRetryAfter,
} from './errors';
