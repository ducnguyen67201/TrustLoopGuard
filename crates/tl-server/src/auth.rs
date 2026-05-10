//! Bearer-token authentication middleware.
//!
//! Static-key auth is the v0 surface — one shared key per deployment,
//! checked against `Authorization: Bearer <token>`. Anything missing or
//! mismatched yields a `401 Unauthorized` with the canonical `ApiError`
//! envelope so SDK clients can branch on `code` without parsing the
//! status line.
//!
//! `/health` is intentionally exempt so liveness probes don't need a
//! key. `tl-server::router` wires this layer onto the protected sub-
//! router and merges the public health route on top.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{ApiError, ApiErrorCode};

/// Holds the expected API key. `Arc`'d so the layer is cheap to clone
/// and so future variants (rotation, JWKS) can swap the inner state
/// without churning the middleware signature.
#[derive(Debug)]
pub struct AuthConfig {
    pub api_key: String,
}

impl AuthConfig {
    pub fn new(api_key: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            api_key: api_key.into(),
        })
    }

    /// Read `TL_API_KEY` from the environment. Errors if unset or empty
    /// — the server should fail fast at boot rather than silently allow
    /// every request through.
    pub fn from_env() -> Result<Arc<Self>, EnvError> {
        let raw = std::env::var("TL_API_KEY").map_err(|_| EnvError::Missing)?;
        if raw.trim().is_empty() {
            return Err(EnvError::Empty);
        }
        Ok(Self::new(raw))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnvError {
    #[error("TL_API_KEY env var is not set")]
    Missing,
    #[error("TL_API_KEY env var is set but empty")]
    Empty,
}

/// Middleware that enforces a bearer token against `cfg.api_key`. Apply
/// to the protected sub-router via:
///
/// ```ignore
/// Router::new()
///     .route("/v1/check", post(check))
///     .layer(axum::middleware::from_fn_with_state(cfg, require_bearer));
/// ```
pub async fn require_bearer(
    State(cfg): State<Arc<AuthConfig>>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let header_value = req.headers().get(header::AUTHORIZATION);

    let presented = header_value
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    match presented {
        Some(token) if subtle_eq(token.as_bytes(), cfg.api_key.as_bytes()) => {
            Ok(next.run(req).await)
        }
        Some(_) => Err(unauthorized("invalid bearer token")),
        None => Err(unauthorized("missing bearer token")),
    }
}

/// Constant-time byte comparison so 401 latency doesn't leak the
/// shared prefix of the configured key. Cheap enough for short tokens.
fn subtle_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn unauthorized(message: &str) -> Response {
    let body = ApiError {
        code: ApiErrorCode::Unauthorized,
        message: message.into(),
        retriable: false,
        details: serde_json::Value::Null,
    };
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtle_eq_handles_unequal_lengths() {
        assert!(!subtle_eq(b"abc", b"abcd"));
        assert!(!subtle_eq(b"abcd", b"abc"));
    }

    #[test]
    fn subtle_eq_matches_equal_bytes() {
        assert!(subtle_eq(b"sk-abcdef", b"sk-abcdef"));
    }

    #[test]
    fn subtle_eq_rejects_byte_mismatch() {
        assert!(!subtle_eq(b"sk-abcdef", b"sk-abcdex"));
    }

    #[test]
    fn from_env_rejects_missing() {
        std::env::remove_var("TL_API_KEY_TEST_MISSING");
        std::env::set_var("TL_API_KEY", ""); // empty
        let err = AuthConfig::from_env().unwrap_err();
        assert!(matches!(err, EnvError::Empty));
    }
}
