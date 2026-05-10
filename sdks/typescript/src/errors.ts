// Typed errors for the TrustLoopGuard TypeScript SDK.
//
// Mirrors `tl-sdk-rust`'s `SdkError` and the Python SDK's exception
// hierarchy. Callers branch on `instanceof` (or on `error.code`) instead
// of inspecting status codes:
//
//   try {
//     await client.check(req);
//   } catch (e) {
//     if (e instanceof RateLimited) {
//       await sleep((e.retryAfter ?? 1) * 1000);
//     } else if (e instanceof Unauthorized) {
//       refreshToken();
//     }
//   }
//
// The status -> code fallback table and retriable-by-default set are kept
// in lockstep with `tl-core::ApiErrorCode::from_http_status` and the
// Python `_STATUS_TO_CODE` dict — the parity is asserted by tests in
// every SDK so they cannot drift silently.

import type { ApiError } from './generated/ApiError';
import type { ApiErrorCode } from './generated/ApiErrorCode';

const STATUS_TO_CODE: Record<number, ApiErrorCode> = {
  400: 'invalid',
  401: 'unauthorized',
  403: 'forbidden',
  404: 'not_found',
  410: 'gone',
  422: 'unprocessable',
  429: 'rate_limited',
  500: 'internal',
  501: 'internal',
  502: 'unavailable',
  503: 'unavailable',
  504: 'unavailable',
};

const DEFAULT_RETRIABLE: ReadonlySet<ApiErrorCode> = new Set<ApiErrorCode>([
  'rate_limited',
  'unavailable',
]);

export function codeFromHttpStatus(status: number): ApiErrorCode {
  if (status in STATUS_TO_CODE) {
    return STATUS_TO_CODE[status]!;
  }
  if (status >= 500 && status < 600) return 'internal';
  return 'invalid';
}

export function synthesizeApiError(status: number, body: string): ApiError {
  const code = codeFromHttpStatus(status);
  return {
    code,
    message: body || `server returned status ${status}`,
    retriable: DEFAULT_RETRIABLE.has(code),
    details: null,
  };
}

export class SdkError extends Error {
  readonly error: ApiError;
  constructor(error: ApiError) {
    super(`${error.code}: ${error.message}`);
    this.name = 'SdkError';
    this.error = error;
    // Preserve prototype chain for `instanceof` to work after transpile.
    Object.setPrototypeOf(this, new.target.prototype);
  }
  get code(): ApiErrorCode {
    return this.error.code;
  }
  isRetriable(): boolean {
    return this.error.retriable;
  }
}

// One subclass per ApiErrorCode. The classes carry no extra behavior
// beyond their name — they exist so callers can use `instanceof` and
// stack traces are labeled. RateLimited is the one exception: it carries
// the parsed Retry-After value.

export class Invalid extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'Invalid';
  }
}
export class Unauthorized extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'Unauthorized';
  }
}
export class Forbidden extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'Forbidden';
  }
}
export class NotFound extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'NotFound';
  }
}
export class Gone extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'Gone';
  }
}
export class Unprocessable extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'Unprocessable';
  }
}
export class RateLimited extends SdkError {
  readonly retryAfter: number | undefined;
  constructor(error: ApiError, retryAfter?: number) {
    super(error);
    this.name = 'RateLimited';
    this.retryAfter = retryAfter;
  }
}
export class Internal extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'Internal';
  }
}
export class Unavailable extends SdkError {
  constructor(error: ApiError) {
    super(error);
    this.name = 'Unavailable';
  }
}
export class Transport extends SdkError {
  constructor(message: string) {
    super({ code: 'unavailable', message, retriable: true, details: null });
    this.name = 'Transport';
  }
}
export class Decode extends SdkError {
  constructor(message: string) {
    super({ code: 'internal', message, retriable: false, details: null });
    this.name = 'Decode';
  }
}

const CODE_TO_CLASS: Record<ApiErrorCode, new (e: ApiError) => SdkError> = {
  invalid: Invalid,
  unauthorized: Unauthorized,
  forbidden: Forbidden,
  not_found: NotFound,
  gone: Gone,
  unprocessable: Unprocessable,
  rate_limited: RateLimited,
  internal: Internal,
  unavailable: Unavailable,
};

function isApiError(value: unknown): value is ApiError {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Partial<ApiError>;
  return (
    typeof v.code === 'string' &&
    typeof v.message === 'string' &&
    typeof v.retriable === 'boolean' &&
    v.code in CODE_TO_CLASS
  );
}

export function fromResponse(
  status: number,
  body: string,
  retryAfter?: number,
): SdkError {
  let apiErr: ApiError;
  try {
    const parsed: unknown = JSON.parse(body);
    apiErr = isApiError(parsed) ? parsed : synthesizeApiError(status, body);
  } catch {
    apiErr = synthesizeApiError(status, body);
  }
  if (apiErr.code === 'rate_limited') {
    return new RateLimited(apiErr, retryAfter);
  }
  const Cls = CODE_TO_CLASS[apiErr.code];
  return new Cls(apiErr);
}

export function parseRetryAfter(header: string | null | undefined): number | undefined {
  if (header === null || header === undefined) return undefined;
  const trimmed = header.trim();
  const n = Number(trimmed);
  if (!Number.isFinite(n)) return undefined;
  return n;
}
