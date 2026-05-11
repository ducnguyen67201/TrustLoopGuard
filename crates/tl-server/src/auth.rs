//! Bearer-token authentication middleware.
//!
//! Two accepted bearer shapes:
//!
//! 1. The static `TL_API_KEY` — constant-time match against the configured
//!    value. Same surface as the v0 server; CI, examples, and the
//!    quickstart all rely on it staying functional.
//! 2. Per-user keys minted via `/v1/admin/keys` — SHA-256 hash compared
//!    against the unrevoked rows in `"ApiKey"`, with a small moka cache
//!    in front so the DB isn't hit on every request.
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

#[cfg(feature = "postgres")]
use moka::future::Cache;
#[cfg(feature = "postgres")]
use std::time::Duration;
#[cfg(feature = "postgres")]
use tl_storage::ApiKeyRepo;
#[cfg(feature = "postgres")]
use uuid::Uuid;

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
    #[error("env var is not set")]
    Missing,
    #[error("env var is set but empty")]
    Empty,
}

/// Bearer for service-to-service admin endpoints. Distinct from
/// `AuthConfig`'s per-user key — that one authenticates SDK callers
/// against `/v1/check`; this one authenticates the dashboard against
/// `/v1/admin/*`.
#[derive(Debug)]
pub struct AdminConfig {
    pub admin_key: String,
}

impl AdminConfig {
    pub fn new(admin_key: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            admin_key: admin_key.into(),
        })
    }

    pub fn from_env() -> Result<Arc<Self>, EnvError> {
        let raw = std::env::var("TL_ADMIN_KEY").map_err(|_| EnvError::Missing)?;
        if raw.trim().is_empty() {
            return Err(EnvError::Empty);
        }
        Ok(Self::new(raw))
    }
}

pub async fn require_admin_bearer(
    State(cfg): State<Arc<AdminConfig>>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let header_value = req.headers().get(header::AUTHORIZATION);
    let presented = header_value
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    match presented {
        Some(token) if subtle_eq(token.as_bytes(), cfg.admin_key.as_bytes()) => {
            Ok(next.run(req).await)
        }
        Some(_) => Err(unauthorized("invalid admin bearer token")),
        None => Err(unauthorized("missing admin bearer token")),
    }
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

/// Composite bearer resolver: static `TL_API_KEY` plus, when available,
/// a DB-backed lookup of per-user keys. Clone-cheap (all inner state is
/// `Arc` or `Cache`-internal `Arc`).
#[derive(Clone)]
pub struct AuthLayer {
    static_key: Option<Arc<str>>,
    #[cfg(feature = "postgres")]
    resolver: Option<KeyResolver>,
}

#[cfg(feature = "postgres")]
#[derive(Clone)]
struct KeyResolver {
    repo: ApiKeyRepo,
    /// Positive hits: hash -> (key_id, user_id). 60s TTL bounds how
    /// long a revoked key keeps working from cache.
    hits: Cache<[u8; 32], (Uuid, String)>,
    /// Negative results: keyed-by-hash sentinel with a shorter TTL so
    /// a retry storm on a bad token doesn't pound the DB but still
    /// gives newly-minted keys a chance to authenticate quickly.
    misses: Cache<[u8; 32], ()>,
}

#[cfg(feature = "postgres")]
const POSITIVE_TTL: Duration = Duration::from_secs(60);
#[cfg(feature = "postgres")]
const NEGATIVE_TTL: Duration = Duration::from_secs(30);
#[cfg(feature = "postgres")]
const CACHE_CAPACITY: u64 = 10_000;

impl AuthLayer {
    /// Static-key-only. Used when Postgres isn't wired (memory-only
    /// deployments, the legacy code path).
    pub fn static_only(cfg: Arc<AuthConfig>) -> Self {
        Self {
            static_key: Some(Arc::from(cfg.api_key.as_str())),
            #[cfg(feature = "postgres")]
            resolver: None,
        }
    }

    /// Static key + DB-backed per-user keys. `static_cfg` may be None
    /// if the deployment opted out of the legacy bearer.
    #[cfg(feature = "postgres")]
    pub fn with_repo(static_cfg: Option<Arc<AuthConfig>>, repo: ApiKeyRepo) -> Self {
        Self::with_repo_and_ttls(static_cfg, repo, POSITIVE_TTL, NEGATIVE_TTL)
    }

    /// Same as `with_repo` but lets callers (mostly tests) shorten the
    /// TTLs so revocation visibility can be asserted without sleeping.
    #[cfg(feature = "postgres")]
    pub fn with_repo_and_ttls(
        static_cfg: Option<Arc<AuthConfig>>,
        repo: ApiKeyRepo,
        positive_ttl: Duration,
        negative_ttl: Duration,
    ) -> Self {
        let hits = Cache::builder()
            .max_capacity(CACHE_CAPACITY)
            .time_to_live(positive_ttl)
            .build();
        let misses = Cache::builder()
            .max_capacity(CACHE_CAPACITY)
            .time_to_live(negative_ttl)
            .build();
        Self {
            static_key: static_cfg.map(|c| Arc::from(c.api_key.as_str())),
            resolver: Some(KeyResolver { repo, hits, misses }),
        }
    }

    pub fn is_configured(&self) -> bool {
        #[cfg(feature = "postgres")]
        {
            self.static_key.is_some() || self.resolver.is_some()
        }
        #[cfg(not(feature = "postgres"))]
        {
            self.static_key.is_some()
        }
    }
}

/// Set in request extensions on successful per-user auth so handlers
/// can attribute the request. Static-key callers don't get this — they
/// are implicitly the legacy / service caller.
#[cfg(feature = "postgres")]
#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub key_id: Uuid,
    pub user_id: String,
}

pub async fn require_auth(
    State(layer): State<Arc<AuthLayer>>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let token = match req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|t| t.to_owned())
    {
        Some(t) if !t.is_empty() => t,
        _ => return Err(unauthorized("missing bearer token")),
    };

    if let Some(static_key) = layer.static_key.as_ref() {
        if subtle_eq(token.as_bytes(), static_key.as_bytes()) {
            return Ok(next.run(req).await);
        }
    }

    #[cfg(feature = "postgres")]
    {
        if let Some(resolver) = layer.resolver.as_ref() {
            return resolve_user_key(resolver, &token, req, next).await;
        }
    }

    Err(unauthorized("invalid bearer token"))
}

#[cfg(feature = "postgres")]
async fn resolve_user_key(
    resolver: &KeyResolver,
    token: &str,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let hash = tl_storage::hash_plaintext(token);

    if resolver.misses.get(&hash).await.is_some() {
        return Err(unauthorized("invalid bearer token"));
    }

    let cached = resolver.hits.get(&hash).await;
    let (key_id, user_id) = match cached {
        Some(v) => v,
        None => match resolver.repo.lookup_by_hash(&hash).await {
            Ok(Some(record)) => {
                let pair = (record.id, record.user_id.clone());
                resolver.hits.insert(hash, pair.clone()).await;
                pair
            }
            Ok(None) => {
                resolver.misses.insert(hash, ()).await;
                return Err(unauthorized("invalid bearer token"));
            }
            Err(e) => {
                // DB blip: reject rather than fail-open. Don't cache —
                // a transient error shouldn't lock a key out for 30s.
                tracing::warn!(error = %e, "api key lookup failed; rejecting");
                return Err(unauthorized("invalid bearer token"));
            }
        },
    };

    req.extensions_mut()
        .insert(AuthenticatedUser { key_id, user_id });

    let repo = resolver.repo.clone();
    tokio::spawn(async move {
        if let Err(e) = repo.touch(key_id).await {
            tracing::debug!(error = %e, "touch last_used_at failed");
        }
    });

    Ok(next.run(req).await)
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

    #[test]
    fn auth_layer_static_only_is_configured() {
        let cfg = AuthConfig::new("sk-test");
        let layer = AuthLayer::static_only(cfg);
        assert!(layer.is_configured());
    }
}
