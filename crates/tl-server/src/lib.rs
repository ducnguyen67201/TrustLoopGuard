//! HTTP routes and OpenAPI doc. Split from `main.rs` so `tl-codegen` can
//! pull `ApiDoc` without booting the runtime.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use tl_core::{CheckRequest, Decision};
use tl_engine::Engine;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

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
        (status = 400, description = "Malformed request"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 410, description = "API version no longer served"),
        (status = 429, description = "Rate limited"),
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

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/check", post(check))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
