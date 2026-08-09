//! Typed errors produced by the Featherlane AI Rust SDK.
//!
//! `SdkError` is the only error type users of the SDK see. Variants line up
//! 1:1 with `ApiErrorCode` plus two infrastructure variants (`Transport`
//! and `Decode`) for failures that happen before we ever see a server body.
//!
//! Mapping rules:
//!   - 2xx                   → no error (`Client::check` returns `Ok`)
//!   - non-2xx with our body → variant determined by `body.code`
//!   - non-2xx other body    → variant determined by HTTP status fallback
//!   - reqwest transport err → `Transport`
//!   - JSON decode failure   → `Decode`
//!
//! `RateLimited` carries the parsed `Retry-After` value when present so
//! the retry middleware (PR 5) can honor it without re-parsing headers.

use std::time::Duration;

use tl_core::{ApiError, ApiErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("invalid request: {0}")]
    Invalid(ApiError),

    #[error("unauthorized: {0}")]
    Unauthorized(ApiError),

    #[error("forbidden: {0}")]
    Forbidden(ApiError),

    #[error("not found: {0}")]
    NotFound(ApiError),

    #[error("conflict: {0}")]
    Conflict(ApiError),

    #[error("api version gone: {0}")]
    Gone(ApiError),

    #[error("unprocessable: {0}")]
    Unprocessable(ApiError),

    #[error("rate limited: {error}")]
    RateLimited {
        error: ApiError,
        retry_after: Option<Duration>,
    },

    #[error("server error: {0}")]
    Internal(ApiError),

    #[error("upstream unavailable: {0}")]
    Unavailable(ApiError),

    #[error("transport: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("decode: {0}")]
    Decode(#[from] serde_json::Error),
}

impl SdkError {
    /// Build a typed `SdkError` from a raw HTTP response. Tries to parse
    /// the body as `ApiError` first; falls back to HTTP-status synthesis
    /// when the body is empty or doesn't match our schema.
    pub(crate) fn from_response(status: u16, body: &str, retry_after: Option<Duration>) -> Self {
        let api_err = serde_json::from_str::<ApiError>(body)
            .unwrap_or_else(|_| crate::synthesize_api_error(status, body));
        Self::from_api_error(api_err, retry_after)
    }

    fn from_api_error(err: ApiError, retry_after: Option<Duration>) -> Self {
        match err.code {
            ApiErrorCode::Invalid => Self::Invalid(err),
            ApiErrorCode::Unauthorized => Self::Unauthorized(err),
            ApiErrorCode::Forbidden => Self::Forbidden(err),
            ApiErrorCode::NotFound => Self::NotFound(err),
            ApiErrorCode::Conflict => Self::Conflict(err),
            ApiErrorCode::Gone => Self::Gone(err),
            ApiErrorCode::Unprocessable => Self::Unprocessable(err),
            ApiErrorCode::RateLimited => Self::RateLimited {
                error: err,
                retry_after,
            },
            ApiErrorCode::Internal => Self::Internal(err),
            ApiErrorCode::Unavailable => Self::Unavailable(err),
        }
    }

    /// True when retrying the same request without modification is
    /// reasonable. The retry middleware in PR 5 consults this; integrators
    /// who roll their own retry can do the same.
    pub fn is_retriable(&self) -> bool {
        match self {
            Self::RateLimited { error, .. } => error.retriable,
            Self::Unavailable(err) => err.retriable,
            Self::Internal(err) => err.retriable,
            Self::Transport(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    /// Underlying canonical error code, for callers that prefer to switch
    /// on a flat enum instead of variant matching.
    pub fn code(&self) -> Option<ApiErrorCode> {
        match self {
            Self::Invalid(e)
            | Self::Unauthorized(e)
            | Self::Forbidden(e)
            | Self::NotFound(e)
            | Self::Conflict(e)
            | Self::Gone(e)
            | Self::Unprocessable(e)
            | Self::Internal(e)
            | Self::Unavailable(e) => Some(e.code),
            Self::RateLimited { error, .. } => Some(error.code),
            Self::Transport(_) | Self::Decode(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_body_to_typed_variant() {
        let body = r#"{"code":"unauthorized","message":"bad token","retriable":false}"#;
        let err = SdkError::from_response(401, body, None);
        assert!(matches!(err, SdkError::Unauthorized(_)));
        assert_eq!(err.code(), Some(ApiErrorCode::Unauthorized));
        assert!(!err.is_retriable());
    }

    #[test]
    fn carries_retry_after_for_rate_limit() {
        let body = r#"{"code":"rate_limited","message":"slow down","retriable":true}"#;
        let err = SdkError::from_response(429, body, Some(Duration::from_secs(7)));
        match err {
            SdkError::RateLimited { retry_after, error } => {
                assert_eq!(retry_after, Some(Duration::from_secs(7)));
                assert!(error.retriable);
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_status_when_body_unrecognized() {
        let err = SdkError::from_response(503, "<html>upstream down</html>", None);
        assert!(matches!(err, SdkError::Unavailable(_)));
        assert!(err.is_retriable());
    }

    #[test]
    fn empty_body_500_synthesizes_internal_error() {
        let err = SdkError::from_response(500, "", None);
        assert!(matches!(err, SdkError::Internal(_)));
        assert_eq!(err.code(), Some(ApiErrorCode::Internal));
    }

    #[test]
    fn body_with_unknown_code_falls_back_to_status() {
        // Server returned a code we don't know about — fall back via the
        // raw status code so we still produce a useful variant.
        let body = r#"{"code":"teapot","message":"i'm a teapot","retriable":false}"#;
        let err = SdkError::from_response(418, body, None);
        // 418 is unmapped → Invalid (catch-all for non-server statuses).
        assert!(matches!(err, SdkError::Invalid(_)));
    }
}
