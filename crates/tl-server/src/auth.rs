//! Bearer-token authentication middleware.
//!
//! Three credential formats today, one middleware:
//!
//! 1. **`TL_API_KEY`** — the static internal/admin key. Used by the
//!    web dashboard's same-origin proxy and operator tooling. Const-
//!    time byte-compare; if it matches, the request is admitted with
//!    no per-user context.
//! 2. **User-session JWT** — minted by `crate::jwt` on
//!    `/v1/auth/{signup,login}`. Verified here; on success the
//!    decoded [`UserContext`] is attached to the request extension
//!    so handlers can read `user_id` without re-parsing headers.
//! 3. **Workspace API key** — `tl_live_...` keys issued from
//!    `/v1/api-keys`. The full key is SHA-256 hashed for lookup; on
//!    success the workspace from storage overrides caller-provided
//!    workspace headers/body fields.
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

use async_trait::async_trait;
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sha2::{Digest, Sha256};
use tl_core::{ApiError, ApiErrorCode};

use crate::auth_user::{UserStore, UserStoreError};
use crate::jwt::{JwtSigner, UserContext};

/// Holds the expected API key plus the optional JWT signer. `Arc`'d
/// so the layer is cheap to clone and so future variants
/// (per-workspace key lookup, rotation) can swap the inner state
/// without churning the middleware signature.
pub struct AuthConfig {
    pub api_key: String,
    pub jwt: Option<Arc<JwtSigner>>,
    pub workspace_keys: Option<Arc<dyn WorkspaceApiKeyVerifier>>,
    pub user_store: Option<Arc<dyn UserStore>>,
    pub hosted_user_approval_required: bool,
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("api_key", &"<redacted>")
            .field("jwt", &self.jwt.is_some())
            .field("workspace_keys", &self.workspace_keys.is_some())
            .field("user_store", &self.user_store.is_some())
            .field(
                "hosted_user_approval_required",
                &self.hosted_user_approval_required,
            )
            .finish()
    }
}

impl AuthConfig {
    pub fn new(api_key: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            api_key: api_key.into(),
            jwt: None,
            workspace_keys: None,
            user_store: None,
            hosted_user_approval_required: false,
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
            workspace_keys: self.workspace_keys.clone(),
            user_store: self.user_store.clone(),
            hosted_user_approval_required: self.hosted_user_approval_required,
        })
    }

    pub fn with_workspace_keys(
        self: &Arc<Self>,
        verifier: Option<Arc<dyn WorkspaceApiKeyVerifier>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            api_key: self.api_key.clone(),
            jwt: self.jwt.clone(),
            workspace_keys: verifier,
            user_store: self.user_store.clone(),
            hosted_user_approval_required: self.hosted_user_approval_required,
        })
    }

    pub fn with_user_approval(
        self: &Arc<Self>,
        user_store: Option<Arc<dyn UserStore>>,
        hosted_user_approval_required: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            api_key: self.api_key.clone(),
            jwt: self.jwt.clone(),
            workspace_keys: self.workspace_keys.clone(),
            user_store,
            hosted_user_approval_required,
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

#[derive(Debug, Clone)]
pub struct WorkspaceKeyContext {
    pub api_key_id: String,
    pub workspace_id: String,
}

/// Marker attached when the request authenticated with the internal
/// service/dashboard bearer token rather than a user JWT or runtime key.
#[derive(Debug, Clone, Copy)]
pub struct InternalServiceContext;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceApiKeyVerifyError {
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait WorkspaceApiKeyVerifier: Send + Sync {
    async fn verify_workspace_api_key(
        &self,
        key_hash: &str,
    ) -> Result<Option<WorkspaceKeyContext>, WorkspaceApiKeyVerifyError>;
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
        if let Some(user_id) = forwarded_user_id(&req) {
            require_approved_user(&cfg, user_id).await?;
        }
        req.extensions_mut().insert(InternalServiceContext);
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
                require_approved_user(&cfg, user_id).await?;
                req.extensions_mut().insert(UserContext {
                    user_id,
                    username: claims.username,
                });
                return Ok(next.run(req).await);
            }
        }
    }

    // 3. Customer/runtime workspace API key. This lane decides the
    // workspace from the stored key row, then overwrites the workspace
    // header so existing handlers cannot be steered cross-workspace by
    // caller-controlled request fields.
    if token.starts_with("tl_live_") {
        if let Some(verifier) = cfg.workspace_keys.as_ref() {
            match verifier
                .verify_workspace_api_key(&sha256_hex(token.as_bytes()))
                .await
            {
                Ok(Some(context)) => {
                    let workspace_header = HeaderValue::from_str(&context.workspace_id)
                        .map_err(|_| unauthorized("invalid workspace attached to API key"))?;
                    req.headers_mut()
                        .insert("x-tlg-workspace-id", workspace_header);
                    req.extensions_mut().insert(context);
                    return Ok(next.run(req).await);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::error!(error = %e, "workspace API key verification failed");
                    return Err(unauthorized("invalid bearer token"));
                }
            }
        }
    }

    Err(unauthorized("invalid bearer token"))
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
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
    api_error(
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::Unauthorized,
        message,
    )
}

fn forbidden(message: &str) -> Response {
    api_error(StatusCode::FORBIDDEN, ApiErrorCode::Forbidden, message)
}

fn internal_error(message: &str) -> Response {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        message,
    )
}

fn api_error(status: StatusCode, code: ApiErrorCode, message: &str) -> Response {
    let body = ApiError {
        code,
        message: message.into(),
        retriable: false,
        details: serde_json::Value::Null,
    };
    (status, Json(body)).into_response()
}

fn forwarded_user_id(req: &Request) -> Option<uuid::Uuid> {
    req.headers()
        .get("x-tlg-user-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| uuid::Uuid::parse_str(value.trim()).ok())
}

async fn require_approved_user(cfg: &AuthConfig, user_id: uuid::Uuid) -> Result<(), Response> {
    if !cfg.hosted_user_approval_required {
        return Ok(());
    }

    let Some(store) = cfg.user_store.as_ref() else {
        tracing::error!(
            user_id = %user_id,
            "hosted approval gate enabled without a user store"
        );
        return Err(forbidden(
            "user approval is required for this hosted deployment",
        ));
    };

    match store.is_approved(user_id).await {
        Ok(true) => Ok(()),
        Ok(false) | Err(UserStoreError::NotFound) => {
            Err(forbidden("user is not approved for this hosted deployment"))
        }
        Err(e) => {
            tracing::error!(
                user_id = %user_id,
                error = %e,
                "user approval lookup failed"
            );
            Err(internal_error("user approval lookup failed"))
        }
    }
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
