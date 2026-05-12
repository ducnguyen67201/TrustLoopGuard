//! Bearer-token authentication middleware.
//!
//! Static-key auth is the v0 surface — one or more shared keys per
//! deployment, checked against `Authorization: Bearer <token>`.
//! Anything missing or mismatched yields a `401 Unauthorized`; valid
//! credentials without the required permission yield `403 Forbidden`.
//!
//! `/health` is intentionally exempt so liveness probes don't need a
//! key. `tl-server::router` wires this layer onto the protected sub-
//! router and merges the public health route on top.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{ApiError, ApiErrorCode};

/// Permission attached to a bearer key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyScope {
    /// Can call runtime guard checks only.
    Runtime,
    /// Can manage policies/agents and can also call runtime checks.
    Admin,
}

impl ApiKeyScope {
    fn allows(self, required: ApiKeyScope) -> bool {
        matches!(self, ApiKeyScope::Admin) || self == required
    }
}

#[derive(Debug, Clone)]
struct ApiKey {
    token: String,
    scope: ApiKeyScope,
}

/// Holds accepted API keys and their scopes. `Arc`'d so the layer is
/// cheap to clone and so future variants (rotation, JWKS) can swap the
/// inner state without churning the middleware signature.
#[derive(Debug)]
pub struct AuthConfig {
    keys: Vec<ApiKey>,
}

impl AuthConfig {
    /// Backwards-compatible constructor for the legacy single shared
    /// key. Legacy keys are admin-scoped so existing deployments keep
    /// the same behavior until they opt into scoped env vars.
    pub fn new(api_key: impl Into<String>) -> Arc<Self> {
        Self::with_admin_key(api_key)
    }

    pub fn with_admin_key(api_key: impl Into<String>) -> Arc<Self> {
        Self::with_keys([(api_key, ApiKeyScope::Admin)])
    }

    pub fn with_keys<I, S>(keys: I) -> Arc<Self>
    where
        I: IntoIterator<Item = (S, ApiKeyScope)>,
        S: Into<String>,
    {
        let keys = keys
            .into_iter()
            .map(|(token, scope)| ApiKey {
                token: token.into(),
                scope,
            })
            .collect();
        Arc::new(Self { keys })
    }

    /// Read API keys from the environment.
    ///
    /// `TL_ADMIN_API_KEY` can manage policies/agents and run checks.
    /// `TL_RUNTIME_API_KEY` can run `/v1/check` only.
    /// `TL_API_KEY` remains supported as a legacy admin key.
    pub fn from_env() -> Result<Arc<Self>, EnvError> {
        let admin = read_optional_env("TL_ADMIN_API_KEY");
        let runtime = read_optional_env("TL_RUNTIME_API_KEY");
        let legacy = read_optional_env("TL_API_KEY");

        if [admin.as_ref(), runtime.as_ref(), legacy.as_ref()]
            .iter()
            .any(|value| matches!(value, Some(Err(_))))
        {
            return Err(EnvError::Empty);
        }

        let mut keys = Vec::new();
        if let Some(Ok(token)) = admin {
            keys.push((token, ApiKeyScope::Admin));
        }
        if let Some(Ok(token)) = runtime {
            keys.push((token, ApiKeyScope::Runtime));
        }
        if let Some(Ok(token)) = legacy {
            keys.push((token, ApiKeyScope::Admin));
        }

        if keys.is_empty() {
            return Err(EnvError::Missing);
        }

        Ok(Self::with_keys(keys))
    }

    fn scope_for_token(&self, token: &str) -> Option<ApiKeyScope> {
        self.keys
            .iter()
            .find(|key| subtle_eq(token.as_bytes(), key.token.as_bytes()))
            .map(|key| key.scope)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnvError {
    #[error("no API key env var is set (TL_ADMIN_API_KEY, TL_RUNTIME_API_KEY, or TL_API_KEY)")]
    Missing,
    #[error("API key env var is set but empty")]
    Empty,
}

/// Middleware that enforces a scoped bearer token. Apply to the
/// protected sub-router via:
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
    let required_scope = required_scope(req.method(), req.uri().path());

    let presented = header_value
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    match presented {
        Some(token) => match cfg.scope_for_token(token) {
            Some(scope) if scope.allows(required_scope) => Ok(next.run(req).await),
            Some(_) => Err(forbidden("api key is not allowed to access this endpoint")),
            None => Err(unauthorized("invalid bearer token")),
        },
        None => Err(unauthorized("missing bearer token")),
    }
}

fn read_optional_env(name: &str) -> Option<Result<String, EnvError>> {
    match std::env::var(name) {
        Ok(value) if value.trim().is_empty() => Some(Err(EnvError::Empty)),
        Ok(value) => Some(Ok(value)),
        Err(_) => None,
    }
}

fn required_scope(method: &Method, path: &str) -> ApiKeyScope {
    match (method, path) {
        (&Method::POST, "/v1/check") => ApiKeyScope::Runtime,
        _ => ApiKeyScope::Admin,
    }
}

fn forbidden(message: &str) -> Response {
    let body = ApiError {
        code: ApiErrorCode::Forbidden,
        message: message.into(),
        retriable: false,
        details: serde_json::Value::Null,
    };
    (StatusCode::FORBIDDEN, Json(body)).into_response()
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
        std::env::remove_var("TL_ADMIN_API_KEY");
        std::env::remove_var("TL_RUNTIME_API_KEY");
        std::env::set_var("TL_API_KEY", ""); // empty
        let err = AuthConfig::from_env().unwrap_err();
        assert!(matches!(err, EnvError::Empty));
    }

    #[test]
    fn scoped_keys_return_expected_permissions() {
        let cfg = AuthConfig::with_keys([
            ("sk-runtime", ApiKeyScope::Runtime),
            ("sk-admin", ApiKeyScope::Admin),
        ]);

        assert_eq!(
            cfg.scope_for_token("sk-runtime"),
            Some(ApiKeyScope::Runtime)
        );
        assert_eq!(cfg.scope_for_token("sk-admin"), Some(ApiKeyScope::Admin));
        assert_eq!(cfg.scope_for_token("sk-missing"), None);
    }

    #[test]
    fn admin_scope_includes_runtime_endpoints() {
        assert!(ApiKeyScope::Admin.allows(ApiKeyScope::Runtime));
        assert!(ApiKeyScope::Admin.allows(ApiKeyScope::Admin));
        assert!(!ApiKeyScope::Runtime.allows(ApiKeyScope::Admin));
    }
}
