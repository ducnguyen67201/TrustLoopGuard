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
//! canonical [`tl_core::ApiError`] envelope.
//!
//! `/health` is intentionally exempt so liveness probes don't need a
//! key. `tl-server::router` wires this layer onto the protected sub-
//! router and merges the public health route on top.

mod approval;
mod response;
#[cfg(test)]
mod tests;
mod token;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

use crate::auth_user::UserStore;
use crate::jwt::{JwtSigner, UserContext};
use approval::{forwarded_user_id, require_approved_user};
use response::unauthorized;
pub(crate) use token::sha256_hex;
use token::{jwt_path_allowed, subtle_eq};

/// Holds the expected API key plus the optional JWT signer. `Arc`'d
/// so the layer is cheap to clone and so future variants
/// (per-workspace key lookup, rotation) can swap the inner state
/// without churning the middleware signature.
pub struct AuthConfig {
    pub api_key: String,
    pub jwt: Option<Arc<JwtSigner>>,
    pub workspace_keys: Option<Arc<dyn WorkspaceApiKeyVerifier>>,
    pub user_store: Option<Arc<dyn UserStore>>,
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("api_key", &"<redacted>")
            .field("jwt", &self.jwt.is_some())
            .field("workspace_keys", &self.workspace_keys.is_some())
            .field("user_store", &self.user_store.is_some())
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
        })
    }

    pub fn with_user_approval(
        self: &Arc<Self>,
        user_store: Option<Arc<dyn UserStore>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            api_key: self.api_key.clone(),
            jwt: self.jwt.clone(),
            workspace_keys: self.workspace_keys.clone(),
            user_store,
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
    pub environment_id: String,
    /// Principal the key was bound to at creation time, if any.
    /// Downstream handlers attribute grants and budgets to it.
    pub principal_id: Option<String>,
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
///     .route("/v1/events", post(submit_event))
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
                let token_type = claims.token_type.clone();
                let token_workspace = claims.workspace_id.clone();
                req.extensions_mut().insert(UserContext {
                    user_id,
                    username: claims.username,
                });
                // OAuth access token: workspace-scoped. Stamp the workspace
                // from the signed claim (callers cannot steer it) and allow
                // the MCP + /v1 surface — this is the resource-server lane.
                if token_type.as_deref() == Some("access") {
                    if let Some(ws) = token_workspace.as_deref() {
                        let ws_header = HeaderValue::from_str(ws)
                            .map_err(|_| unauthorized("invalid workspace in access token"))?;
                        req.headers_mut().insert("x-tlg-workspace-id", ws_header);
                        return Ok(next.run(req).await);
                    }
                    return Err(unauthorized("access token missing workspace scope"));
                }
                if jwt_path_allowed(req.uri().path()) {
                    return Ok(next.run(req).await);
                }
                return Err(unauthorized("user JWT is not permitted for this route"));
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
                    let environment_header = HeaderValue::from_str(&context.environment_id)
                        .map_err(|_| unauthorized("invalid environment attached to API key"))?;
                    req.headers_mut()
                        .insert("x-tlg-environment-id", environment_header);
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

/// Middleware that only accepts the internal `TL_API_KEY` bearer token.
/// Used for endpoints that trust upstream identity assertions from the
/// dashboard server and therefore must not admit user JWT/workspace keys.
pub async fn require_internal_bearer(
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

    if subtle_eq(token.as_bytes(), cfg.api_key.as_bytes()) {
        if let Some(user_id) = forwarded_user_id(&req) {
            require_approved_user(&cfg, user_id).await?;
        }
        req.extensions_mut().insert(InternalServiceContext);
        return Ok(next.run(req).await);
    }

    Err(unauthorized("invalid bearer token"))
}
