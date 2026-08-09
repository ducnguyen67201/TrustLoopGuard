"""Typed errors for the Featherlane AI Python SDK.

Mirrors `tl-sdk-rust`'s `SdkError` and the canonical `ApiError` envelope
defined in `tl-core`. Callers branch on exception type instead of
inspecting status codes:

    try:
        client.submit_event(event)
    except RateLimited as e:
        time.sleep(e.retry_after or 1)
    except Unauthorized as e:
        refresh_token(); ...

The mapping rules match the Rust SDK exactly (see `error.rs`):
  - 2xx                    -> no exception
  - non-2xx with our body  -> exception class chosen by `body.code`
  - non-2xx other body     -> exception class chosen by HTTP status fallback
  - httpx transport error  -> Transport
  - JSON decode failure    -> Decode
"""

from __future__ import annotations

from typing import Any

from featherlane_ai._generated.types import ApiError, ApiErrorCode

# Status -> code fallback table. Mirrors `ApiErrorCode::from_http_status`
# in tl-core. Tested for parity in tests/test_errors_parity.py.
_STATUS_TO_CODE: dict[int, ApiErrorCode] = {
    400: ApiErrorCode.invalid,
    401: ApiErrorCode.unauthorized,
    403: ApiErrorCode.forbidden,
    404: ApiErrorCode.not_found,
    409: ApiErrorCode.conflict,
    410: ApiErrorCode.gone,
    422: ApiErrorCode.unprocessable,
    429: ApiErrorCode.rate_limited,
    500: ApiErrorCode.internal,
    501: ApiErrorCode.internal,
    502: ApiErrorCode.unavailable,
    503: ApiErrorCode.unavailable,
    504: ApiErrorCode.unavailable,
}

# Codes that are retriable by default when synthesizing an ApiError from
# a raw HTTP status. The server may override via `ApiError.retriable`.
_DEFAULT_RETRIABLE: frozenset[ApiErrorCode] = frozenset(
    {ApiErrorCode.rate_limited, ApiErrorCode.unavailable}
)


def code_from_http_status(status: int) -> ApiErrorCode:
    """Map a raw HTTP status to the canonical error code.

    Used as a fallback when the response body isn't a recognizable
    ApiError. Mirrors `ApiErrorCode::from_http_status` in Rust.
    """
    if status in _STATUS_TO_CODE:
        return _STATUS_TO_CODE[status]
    if 500 <= status < 600:
        return ApiErrorCode.internal
    return ApiErrorCode.invalid


def synthesize_api_error(status: int, body: str) -> ApiError:
    """Build an ApiError from a raw HTTP response when the body isn't ours."""
    code = code_from_http_status(status)
    return ApiError(
        code=code,
        message=body or f"server returned status {status}",
        retriable=code in _DEFAULT_RETRIABLE,
        details=None,
    )


class SdkError(Exception):
    """Base class for every error the SDK raises. Always carries an ApiError
    payload (synthesized if the server didn't return our envelope).
    """

    def __init__(self, error: ApiError) -> None:
        super().__init__(f"{error.code.value}: {error.message}")
        self.error = error

    @property
    def code(self) -> ApiErrorCode:
        return self.error.code

    def is_retriable(self) -> bool:
        """True when retrying the same request without modification is
        reasonable. Honored by the retry middleware in PR 6.
        """
        return self.error.retriable


class Invalid(SdkError):
    """400 — request malformed or failed validation."""


class Unauthorized(SdkError):
    """401 — missing or invalid credentials."""


class Forbidden(SdkError):
    """403 — credentials valid but caller lacks permission."""


class NotFound(SdkError):
    """404 — referenced resource not found."""


class Conflict(SdkError):
    """409 — first-write-wins or optimistic-concurrency conflict."""


class Gone(SdkError):
    """410 — API version retired; caller must upgrade."""


class Unprocessable(SdkError):
    """422 — well-formed but semantically rejected."""


class RateLimited(SdkError):
    """429 — rate limited. Honor `retry_after` when set."""

    def __init__(self, error: ApiError, retry_after: float | None = None) -> None:
        super().__init__(error)
        self.retry_after = retry_after


class Internal(SdkError):
    """5xx — server-side bug."""


class Unavailable(SdkError):
    """502 / 503 / 504 — transient infra issue. Retriable with backoff."""


class Transport(SdkError):
    """Pre-server failure (network, DNS, timeout). No ApiError from the
    server, so we synthesize one carrying the underlying message.
    """

    def __init__(self, message: str) -> None:
        super().__init__(
            ApiError(
                code=ApiErrorCode.unavailable,
                message=message,
                retriable=True,
                details=None,
            )
        )


class Decode(SdkError):
    """Server returned 2xx but the body didn't parse as Decision."""

    def __init__(self, message: str) -> None:
        super().__init__(
            ApiError(
                code=ApiErrorCode.internal,
                message=message,
                retriable=False,
                details=None,
            )
        )


_CODE_TO_CLASS: dict[ApiErrorCode, type[SdkError]] = {
    ApiErrorCode.invalid: Invalid,
    ApiErrorCode.unauthorized: Unauthorized,
    ApiErrorCode.forbidden: Forbidden,
    ApiErrorCode.not_found: NotFound,
    ApiErrorCode.conflict: Conflict,
    ApiErrorCode.gone: Gone,
    ApiErrorCode.unprocessable: Unprocessable,
    ApiErrorCode.rate_limited: RateLimited,
    ApiErrorCode.internal: Internal,
    ApiErrorCode.unavailable: Unavailable,
}


def from_response(
    status: int, body: str, retry_after: float | None = None
) -> SdkError:
    """Build a typed SdkError from a raw HTTP response.

    Mirrors `SdkError::from_response` in Rust. Tries to parse the body as
    ApiError first; falls back to status-based synthesis.
    """
    api_err: ApiError
    try:
        api_err = ApiError.model_validate_json(body)
    except Exception:  # noqa: BLE001 — any parse failure means "not our body"
        api_err = synthesize_api_error(status, body)

    cls = _CODE_TO_CLASS[api_err.code]
    if cls is RateLimited:
        return RateLimited(api_err, retry_after=retry_after)
    return cls(api_err)


def parse_retry_after(header: str | None) -> float | None:
    """Parse the Retry-After header into seconds. Returns None if absent
    or unparseable (HTTP-date forms aren't supported yet — only delta-seconds).
    """
    if header is None:
        return None
    try:
        return float(header.strip())
    except (TypeError, ValueError):
        return None


__all__: list[str] = [
    "SdkError",
    "Invalid",
    "Unauthorized",
    "Forbidden",
    "NotFound",
    "Conflict",
    "Gone",
    "Unprocessable",
    "RateLimited",
    "Internal",
    "Unavailable",
    "Transport",
    "Decode",
    "code_from_http_status",
    "synthesize_api_error",
    "from_response",
    "parse_retry_after",
    "ApiError",
    "ApiErrorCode",
]


# Suppress unused-import warning: ApiError/ApiErrorCode are re-exported
# so callers can write `except SdkError as e: e.error: ApiError` cleanly.
_ = (Any,)
