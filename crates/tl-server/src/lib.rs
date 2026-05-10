//! HTTP routes and OpenAPI doc. Split from `main.rs` so `tl-codegen` can
//! pull `ApiDoc` without booting the runtime.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    middleware::from_fn_with_state,
    routing::{get, post},
    Json, Router,
};
use tl_core::{CheckRequest, Decision};
use tl_engine::Engine;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

pub mod auth;
pub use auth::{AuthConfig, EnvError as AuthEnvError};

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Engine>,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "TrustLoopGuard API",
        version = "0.0.1",
        description = "Real-time guardrail runtime for AI agents.",
        license(name = "Apache-2.0"),
    ),
    paths(check, health),
    components(schemas(
        tl_core::CheckRequest,
        tl_core::Decision,
        tl_core::Verdict,
        tl_core::Channel,
        tl_core::Severity,
        tl_core::TriggeredPolicy,
        tl_core::ApiError,
        tl_core::ApiErrorCode,
    )),
    tags(
        (name = "guard", description = "Real-time guard checks")
    ),
)]
pub struct ApiDoc;

#[utoipa::path(
    post,
    path = "/v1/check",
    tag = "guard",
    request_body = CheckRequest,
    responses(
        (status = 200, description = "Decision returned", body = Decision),
        // Error responses share the canonical ApiError envelope so SDK
        // clients can deserialize once and branch on `code`. See
        // docs/SDK_DRIVEN.md for the discipline this enforces.
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 410, description = "API version no longer served", body = ApiError),
        (status = 429, description = "Rate limited", body = ApiError),
    ),
)]
pub async fn check(
    State(state): State<AppState>,
    Json(req): Json<CheckRequest>,
) -> Result<Json<Decision>, StatusCode> {
    let decision = state.engine.check(&req);
    Ok(Json(decision))
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "guard",
    responses((status = 200, description = "Liveness probe", body = String)),
)]
pub async fn health() -> &'static str {
    "ok"
}

/// Build the application router.
///
/// `auth` is optional so deployments without exposed endpoints (local
/// dev, integration tests) can run without setting `TL_API_KEY`. When
/// `Some`, every `/v1/*` route requires `Authorization: Bearer <key>`;
/// `/health` is always public so liveness probes don't need a secret.
pub fn router(state: AppState, auth: Option<Arc<AuthConfig>>) -> Router {
    let public = Router::new().route("/health", get(health));

    let mut protected = Router::new()
        .route("/v1/check", post(check))
        .with_state(state);

    if let Some(cfg) = auth {
        protected = protected.layer(from_fn_with_state(cfg, auth::require_bearer));
    }

    public.merge(protected).layer(TraceLayer::new_for_http())
}
