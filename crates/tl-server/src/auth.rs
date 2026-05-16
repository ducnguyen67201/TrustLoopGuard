//! Bearer-token authentication middleware.
//!
//! Two credential formats today, one middleware:
//!
//! 1. **`TL_API_KEY`** — the static internal/admin key. Used by the
//!    web dashboard's same-origin proxy and operator tooling. Const-
//!    time byte-compare; if it matches, the request is admitted with
//!    no per-user context.
//! 2. **User-session JWT** — minted by `crate::jwt` on
//!    `/v1/auth/{signup,login}`. Verified here; on success the
//!    decoded [`UserContext`] is attached to the request extension
//!    so handlers can read `user_id` without re-parsing headers.
//!
//! Order: try API-key first (no signature verify), fall back to JWT
//! verification. Either path lets the request through. Anything
//! missing or unrecognized yields `401 Unauthorized` with the
//! canonical [`ApiError`] envelope.
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

use crate::jwt::{JwtSigner, UserContext};

/// Holds the expected API key plus the optional JWT signer. `Arc`'d
/// so the layer is cheap to clone and so future variants
/// (per-workspace key lookup, rotation) can swap the inner state
/// without churning the middleware signature.
#[derive(Debug)]
pub struct AuthConfig {
    pub api_key: String,
    pub jwt: Option<Arc<JwtSigner>>,
}

impl AuthConfig {
    pub fn new(api_key: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            api_key: api_key.into(),
            jwt: None,
        })
    }

    /// Returns a new `AuthConfig` with the JWT signer attached. The
    /// existing instance is cheap to drop because `AuthConfig` is
    /// already small; we don't try to mutate it in place because
    /// every caller already shares it via `Arc`.
    pub fn with_jwt(self: &Arc<Self>, signer: Option<Arc<JwtSigner>>) -> Arc<Self> {
        Arc::new(Self {
            api_key: self.api_key.clone(),
            jwt: signer,
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
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let presented = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let Some(token) = presented else {
        return Err(unauthorized("missing bearer token"));
    };

    // 1. Internal API key — fast path, const-time compare.
    if subtle_eq(token.as_bytes(), cfg.api_key.as_bytes()) {
        return Ok(next.run(req).await);
    }

    // 2. User JWT — only attempted if a signer is configured.
    //    On success, attach UserContext to the request extension so
    //    handlers (e.g. /v1/team/my-workspaces) can read user_id
    //    without trusting raw X-TLG-User-Id headers.
    if let Some(signer) = cfg.jwt.as_ref() {
        if let Ok(claims) = signer.verify(token) {
            // Parsed in JwtSigner::verify, but redo here so the type
            // is uuid::Uuid for downstream consumers.
            if let Ok(user_id) = uuid::Uuid::parse_str(&claims.sub) {
                req.extensions_mut().insert(UserContext {
                    user_id,
                    username: claims.username,
                });
                return Ok(next.run(req).await);
            }
        }
    }

    Err(unauthorized("invalid bearer token"))
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
        std::env::set_var("TL_API_KEY", ""); // empty
        let err = AuthConfig::from_env().unwrap_err();
        assert!(matches!(err, EnvError::Empty));
    }
}
