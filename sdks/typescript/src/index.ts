// Public surface of the TrustLoopGuard TypeScript SDK.
// Type definitions are generated from Rust by `cargo run -p tl-codegen`.
// See README.md in src/generated for regen instructions.

export type * from './generated/AuthorizationDecision.js';
export type * from './generated/AuthorizationDomain.js';
export type * from './generated/AuthorizationEffect.js';
export type * from './generated/AuthorizationIntentStatus.js';
export type * from './generated/AuthorizationCapabilityId.js';
export type * from './generated/AuthorizationSubject.js';
export type * from './generated/AuthorizationFinding.js';
export type * from './generated/AuthorityRequirement.js';
export type * from './generated/AuthorizationClaim.js';
export type * from './generated/AuthorizationApproval.js';
export type * from './generated/AuthorizationApprovalSummary.js';
export type * from './generated/AuthorizationApprovalListResponse.js';
export type * from './generated/AuthorizationGrant.js';
export type * from './generated/AuthorizationGrantRef.js';
export type * from './generated/AuthorizationGrantListResponse.js';
export type * from './generated/AuthorizationGrantScope.js';
export type * from './generated/AuthorizationReceipt.js';
export type * from './generated/AuthorizationLease.js';
export type * from './generated/ApprovalEnvelope.js';
export type * from './generated/ApprovalDecision.js';
export type * from './generated/ApprovalStatus.js';
export type * from './generated/GrantMode.js';
export type * from './generated/GrantStatus.js';
export type * from './generated/LeaseStatus.js';
export type * from './generated/CreateAuthorizationGrantRequest.js';
export type * from './generated/DecideAuthorizationApprovalRequest.js';
export type * from './generated/DecideAuthorizationApprovalResponse.js';
export type * from './generated/CompleteAuthorizationLeaseRequest.js';
export type * from './generated/Channel.js';
export type * from './generated/Severity.js';
export type * from './generated/TriggeredPolicy.js';
export type * from './generated/ShellActionParameters.js';
export type * from './generated/ShellLanguage.js';
export type * from './generated/AgentAuthority.js';
export type * from './generated/AgentListResponse.js';
export type * from './generated/AgentProfile.js';
export type * from './generated/WorkflowDefinition.js';
export type * from './generated/WorkflowRequirement.js';
export type * from './generated/AgentScope.js';
export type * from './generated/AgentTone.js';
export type * from './generated/KnowledgeSource.js';
export type * from './generated/CreateKnowledgeSourceRequest.js';
export type * from './generated/DashboardKnowledgeSourceKind.js';
export type * from './generated/KnowledgeFileInput.js';
export type * from './generated/KnowledgeFileMetadata.js';
export type * from './generated/KnowledgeSourceDocument.js';
export type * from './generated/KnowledgeSourceFileResponse.js';
export type * from './generated/KnowledgeSourceListResponse.js';
export type * from './generated/KnowledgeSourceStatus.js';
export type * from './generated/ApiError.js';
export type * from './generated/ApiErrorCode.js';
export type * from './generated/PolicyDocument.js';
export type * from './generated/PolicyFamily.js';
export type * from './generated/PolicyBatchSetEnabledRequest.js';
export type * from './generated/PolicyBatchSetEnabledResponse.js';
export type * from './generated/PolicyListResponse.js';
export type * from './generated/PolicySetEnabledRequest.js';
export type * from './generated/PolicySummary.js';
export type * from './generated/PolicyValidateResponse.js';
export type * from './generated/PolicyValidationIssue.js';
export type * from './generated/PolicyMatchType.js';
export type * from './generated/PolicyDraft.js';
export type * from './generated/PolicyDraftRequest.js';
export type * from './generated/PolicyDraftResponse.js';
export type * from './generated/GuardrailGenerateResponse.js';
export type * from './generated/GuardrailListResponse.js';
export type * from './generated/ApiKeyBatchRevokeRequest.js';
export type * from './generated/ApiKeyBatchRevokeResponse.js';
export type * from './generated/ApiKeyListResponse.js';
export type * from './generated/CreateApiKeyRequest.js';
export type * from './generated/CreateApiKeyResponse.js';
export type * from './generated/DashboardApiKey.js';
export type * from './generated/WorkspaceSettings.js';
export type * from './generated/TraceListResponse.js';
export type * from './generated/TraceSummary.js';
export type * from './generated/CreateRunEventRequest.js';
export type * from './generated/CreateRunRequest.js';
export type * from './generated/AgenticPaymentAuthorizationResponse.js';
export type * from './generated/AgenticPaymentAuthorizeRequest.js';
export type * from './generated/AgenticPaymentCommitRequest.js';
export type * from './generated/AgenticPaymentRecord.js';
export type * from './generated/AgenticPaymentReservation.js';
export type * from './generated/AgenticPaymentReservationStatus.js';
export type * from './generated/AgenticPaymentRollbackRequest.js';
export type * from './generated/CounterpartyRef.js';
export type * from './generated/CreateFinancialActionRequest.js';
export type * from './generated/CreateFinancialPolicyRequest.js';
export type * from './generated/EvidenceRef.js';
export type * from './generated/FinancialAction.js';
export type * from './generated/FinancialActionKind.js';
export type * from './generated/FinancialActionListResponse.js';
export type * from './generated/FinancialActionOutcome.js';
export type * from './generated/FinancialActionOutcomeStatus.js';
export type * from './generated/FinancialActionPrecondition.js';
export type * from './generated/FinancialActionRecord.js';
export type * from './generated/FinancialActionState.js';
export type * from './generated/FinancialExecutionStatus.js';
export type * from './generated/BudgetAlertConfig.js';
export type * from './generated/BudgetAlertConfigListResponse.js';
export type * from './generated/BudgetAlertFiring.js';
export type * from './generated/BudgetAlertFiringListResponse.js';
export type * from './generated/BudgetAlertThresholdType.js';
export type * from './generated/BudgetAlertWindow.js';
export type * from './generated/CreateBudgetAlertConfigRequest.js';
export type * from './generated/UpdateBudgetAlertConfigRequest.js';
export type * from './generated/FinancialEligibilityCheck.js';
export type * from './generated/FinancialEligibilityResult.js';
export type * from './generated/FinancialEligibilityStatus.js';
export type * from './generated/FinancialOutcomeListResponse.js';
export type * from './generated/FinancialPolicyListResponse.js';
export type * from './generated/FinancialPolicyRecord.js';
export type * from './generated/FinancialPolicySelector.js';
export type * from './generated/FinancialRail.js';
export type * from './generated/FinancialReceipt.js';
export type * from './generated/SpendMeter.js';
export type * from './generated/MoneyAmount.js';
export type * from './generated/RecoveryStatus.js';
export type * from './generated/ReversalCapability.js';
export type * from './generated/X402NormalizedPaymentRequirement.js';
export type * from './generated/X402PaymentRequirement.js';
export type * from './generated/X402SettlementProof.js';
export type * from './generated/UpdateRunRequest.js';
export type * from './generated/RunDetail.js';
export type * from './generated/RunEventKind.js';
export type * from './generated/RunEventListResponse.js';
export type * from './generated/RunEventSummary.js';
export type * from './generated/RunKind.js';
export type * from './generated/RunListResponse.js';
export type * from './generated/RunStatus.js';
export type * from './generated/RunSummary.js';
export type * from './generated/CreateGatewayProviderConnectionRequest.js';
export type * from './generated/CreateGatewayRouteRequest.js';
export type * from './generated/GatewayCredentialStatus.js';
export type * from './generated/GatewayProviderConnection.js';
export type * from './generated/GatewayProviderConnectionListResponse.js';
export type * from './generated/GatewayProviderKind.js';
export type * from './generated/GatewayRoute.js';
export type * from './generated/GatewayRouteListResponse.js';
export type * from './generated/UpdateGatewayProviderConnectionRequest.js';
export type * from './generated/UpdateGatewayRouteRequest.js';
export type * from './generated/LlmUsageBucket.js';
export type * from './generated/LlmUsageBucketsResponse.js';
export type * from './generated/LlmUsageEvent.js';
export type * from './generated/LlmUsageKind.js';
export type * from './generated/LlmUsageListResponse.js';
export type * from './generated/LlmUsageResponse.js';
export type * from './generated/LlmModelPrice.js';
export type * from './generated/LlmPriceSource.js';
export type * from './generated/LlmPricingListResponse.js';
export type * from './generated/UpsertLlmModelPriceRequest.js';
export type * from './generated/RunProviderUsage.js';
export type * from './generated/RunGuardrailUsage.js';
export type * from './generated/RunBudgetWindowSnapshot.js';
export type * from './generated/RunLlmBudgetDecision.js';
export type * from './generated/Action.js';
export type * from './generated/AllowedSource.js';
export type * from './generated/ApprovalRule.js';
export type * from './generated/CheckerFindingEvidence.js';
export type * from './generated/CheckerRun.js';
export type * from './generated/Confidentiality.js';
export type * from './generated/EnforcementMode.js';
export type * from './generated/EnvironmentCheckerModes.js';
export type * from './generated/UpdateEnvironmentCheckerModesRequest.js';
export type * from './generated/EventKind.js';
export type * from './generated/GuardEvent.js';
export type * from './generated/Integrity.js';
export type * from './generated/LabelBasis.js';
export type * from './generated/LabelBasisSet.js';
export type * from './generated/LabelPolicyStatus.js';
export type * from './generated/LabelResolution.js';
export type * from './generated/Labels.js';
export type * from './generated/Origin.js';
export type * from './generated/ParamRole.js';
export type * from './generated/ParamSpec.js';
export type * from './generated/Principal.js';
export type * from './generated/ProvenanceMap.js';
export type * from './generated/SideEffectClass.js';
export type * from './generated/SignalEvidence.js';
export type * from './generated/Source.js';
export type * from './generated/SourceLabelEvidence.js';
export type * from './generated/SourceLabelPolicy.js';
export type * from './generated/SourceLabelPolicyEntry.js';
export type * from './generated/SourceLabelPolicyListResponse.js';
export type * from './generated/ToolMetadata.js';
export type * from './generated/ToolMetadataEntry.js';
export type * from './generated/ToolMetadataListResponse.js';
export type * from './generated/ToolResolution.js';
export type * from './generated/Trust.js';
export type * from './generated/UpsertSourceLabelPolicyRequest.js';
export type * from './generated/UpsertToolMetadataRequest.js';
export type * from './generated/JobStatus.js';
export type * from './generated/RedteamDispatchRequest.js';
export type * from './generated/RedteamAttackSession.js';
export type * from './generated/RedteamSessionEvent.js';
export type * from './generated/RedteamJobSummary.js';
export type * from './generated/RedteamJobDetail.js';
export type * from './generated/RedteamJobListResponse.js';
export type * from './generated/ReportSeverity.js';
export type * from './generated/ComparedAttackStatus.js';
export type * from './generated/RedteamReportFinding.js';
export type * from './generated/RedteamReportAggregates.js';
export type * from './generated/RedteamComparedAttack.js';
export type * from './generated/RedteamReportComparison.js';
export type * from './generated/RedteamReportPayload.js';
export type * from './generated/CreateReportRequest.js';
export type * from './generated/RedteamReportShare.js';
export type * from './generated/RedteamAttackRecord.js';
export type * from './generated/RedteamAttackRecordListResponse.js';
export type * from './generated/HardenRequest.js';
export type * from './generated/HardenResponse.js';
export type * from './generated/HardenCandidate.js';
export type * from './generated/VerifyResult.js';
export type * from './generated/AttackVector.js';
export type * from './generated/WorkflowPath.js';
export type * from './generated/RedteamPlanRequest.js';
export type * from './generated/RedteamPlanResponse.js';
export type * from './generated/RedteamPlanListResponse.js';
export type * from './generated/ToolIdentity.js';

export { Client } from './client.js';
export type {
  ActiveRun,
  AuthorizedActionOptions,
  AuthorizedActionResult,
  AuthorizedShellActionOptions,
  ClientOptions,
  FinancialOperation,
  FinancialOperationRunOptions,
  FinancialOperationSpec,
  GuardToolCallOptions,
  ListTracesOptions,
  WithRunOptions,
} from './client.js';

export { GuardMode, guard, guardAgent } from './guard.js';
export type {
  GuardCallbacks,
  GuardOptions,
  GuardFactoryOptions,
  GuardCallOptions,
  GuardStreamCallOptions,
  GuardWrapOptions,
  GuardLogEvent,
  OutputGuard,
  ReplyAgent,
  RegenerateFeedback,
} from './guard.js';

export { DEFAULT_RETRY, nextDelay } from './retry.js';
export type { RetryConfig } from './retry.js';

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
} from './errors.js';
