// Public surface of the TrustLoopGuard TypeScript SDK.
// Type definitions are generated from Rust by `cargo run -p tl-codegen`.
// See README.md in src/generated for regen instructions.

export * from './generated/CheckRequest';
export * from './generated/Decision';
export * from './generated/Verdict';
export * from './generated/Channel';
export * from './generated/Severity';
export * from './generated/TriggeredPolicy';
export * from './generated/AgentAuthority';
export * from './generated/AgentListResponse';
export * from './generated/AgentProfile';
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
export * from './generated/UpdateRunRequest';
export * from './generated/RunDetail';
export * from './generated/RunEventKind';
export * from './generated/RunEventListResponse';
export * from './generated/RunEventSummary';
export * from './generated/RunKind';
export * from './generated/RunListResponse';
export * from './generated/RunStatus';
export * from './generated/RunSummary';
export * from './generated/CreateEnforcementProfileRequest';
export * from './generated/CreateGatewayProviderConnectionRequest';
export * from './generated/CreateGatewayRouteRequest';
export * from './generated/EnforcementProfile';
export * from './generated/EnforcementProfileListResponse';
export * from './generated/FailMode';
export * from './generated/GatewayCredentialStatus';
export * from './generated/GatewayInputAction';
export * from './generated/GatewayOutputAction';
export * from './generated/GatewayProviderConnection';
export * from './generated/GatewayProviderConnectionListResponse';
export * from './generated/GatewayProviderKind';
export * from './generated/GatewayRoute';
export * from './generated/GatewayRouteListResponse';
export * from './generated/ResponseMode';
export * from './generated/RetentionMode';
export * from './generated/UpdateEnforcementProfileRequest';
export * from './generated/UpdateGatewayProviderConnectionRequest';
export * from './generated/UpdateGatewayRouteRequest';
export * from './generated/Action';
export * from './generated/AllowedSource';
export * from './generated/ApprovalRule';
export * from './generated/Confidentiality';
export * from './generated/EventKind';
export * from './generated/GuardEvent';
export * from './generated/Integrity';
export * from './generated/Labels';
export * from './generated/Origin';
export * from './generated/ParamRole';
export * from './generated/ParamSpec';
export * from './generated/Principal';
export * from './generated/ProvenanceMap';
export * from './generated/SideEffectClass';
export * from './generated/Source';
export * from './generated/ToolMetadata';
export * from './generated/Trust';

export { Client } from './client';
export type { ClientOptions } from './client';

export { redactCheckRequest } from './redaction';
export type {
  RedactedCheckRequest,
  RedactedEntity,
  RedactionEntityType,
  RedactionInfo,
  RedactionMode,
  RedactionOptions,
  RedactionResult,
  RedactionStatus,
} from './redaction';

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
