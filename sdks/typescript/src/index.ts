// Public surface of the TrustLoopGuard TypeScript SDK.
// Type definitions are generated from Rust by `cargo run -p tl-codegen`.
// See README.md in src/generated for regen instructions.

export * from './generated/CheckRequest';
export * from './generated/Decision';
export * from './generated/Verdict';
export * from './generated/Channel';
export * from './generated/Severity';
export * from './generated/TriggeredPolicy';
export * from './generated/ApiError';
export * from './generated/ApiErrorCode';
export * from './generated/PolicyDocument';
export * from './generated/PolicyListResponse';
export * from './generated/PolicySetEnabledRequest';
export * from './generated/PolicySummary';
export * from './generated/PolicyValidateResponse';
export * from './generated/PolicyValidationIssue';

export { Client } from './client';
export type { ClientOptions } from './client';

export { guard } from './guard';
export type {
  GuardCallbacks,
  GuardOptions,
  GuardFactoryOptions,
  GuardCallOptions,
  GuardLogEvent,
  OutputGuard,
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
