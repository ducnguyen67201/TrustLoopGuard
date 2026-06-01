use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum TlError {
    #[error("policy compile error: {0}")]
    PolicyCompile(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Canonical error envelope returned on non-2xx responses from every
/// TrustLoopGuard endpoint. SDKs deserialize this body to produce typed
/// errors; integrators don't have to inspect status codes by hand.
///
/// The shape is intentionally minimal — `code` drives SDK-side fan-out,
/// `message` is for logs, `retriable` tells callers whether the same
/// request may be retried, and `details` is opaque so the server can
/// add validation field paths without breaking SDKs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    /// Whether the caller may retry the same request without modification.
    /// SDKs honor `Retry-After` when present in addition to this flag.
    pub retriable: bool,
    /// Opaque structured details (e.g. validation field path).
    /// Defaults to `null`; servers may add fields without breaking SDKs.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub details: serde_json::Value,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

/// Stable error code dictionary. Add variants here when introducing new
/// failure modes; never repurpose an existing variant — SDK callers may
/// be branching on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ApiErrorCode {
    /// 400 — request malformed or failed validation.
    Invalid,
    /// 401 — missing or invalid credentials.
    Unauthorized,
    /// 403 — credentials valid but caller lacks permission.
    Forbidden,
    /// 404 — referenced resource not found.
    NotFound,
    /// 410 — API version retired; caller must upgrade.
    Gone,
    /// 422 — well-formed but semantically rejected.
    Unprocessable,
    /// 429 — rate limited. Honor `Retry-After` header when present.
    RateLimited,
    /// 500 — server-side bug; not retriable without server fix.
    Internal,
    /// 502 / 503 / 504 — transient infra issue; retriable with backoff.
    Unavailable,
}

impl ApiErrorCode {
    /// Map an HTTP status code to the canonical error code. Used by SDKs
    /// when the server returned a body that doesn't match `ApiError` —
    /// gives us a fallback that's still useful to integrators.
    pub fn from_http_status(status: u16) -> Self {
        match status {
            400 => Self::Invalid,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            410 => Self::Gone,
            422 => Self::Unprocessable,
            429 => Self::RateLimited,
            500..=501 => Self::Internal,
            502..=504 => Self::Unavailable,
            _ if (500..600).contains(&status) => Self::Internal,
            _ => Self::Invalid,
        }
    }

    /// Default retriable flag for this code. The server may override via
    /// `ApiError.retriable`; this is only used when synthesizing an
    /// `ApiError` from a raw HTTP status.
    pub fn default_retriable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable)
    }
}
