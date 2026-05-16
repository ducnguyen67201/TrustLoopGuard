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
export * from './generated/ApiKeyListResponse';
export * from './generated/CreateApiKeyRequest';
export * from './generated/CreateApiKeyResponse';
export * from './generated/DashboardApiKey';
export * from './generated/WorkspaceSettings';
export * from './generated/TraceListResponse';
export * from './generated/TraceSummary';

export { Client } from './client';
export type { ClientOptions } from './client';

export { GuardMode, guard } from './guard';
export type {
  GuardCallbacks,
  GuardOptions,
  GuardFactoryOptions,
  GuardCallOptions,
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
