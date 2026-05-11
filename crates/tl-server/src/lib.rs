//! HTTP routes and OpenAPI doc. Split from `main.rs` so `tl-codegen` can
//! pull `ApiDoc` without booting the runtime.

use std::sync::Arc;

use axum::{
    extract::State,
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tl_core::CheckRequest;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;

pub mod agents;
pub mod auth;
pub mod state;
pub use agents::{AgentState, AgentStore, AgentStoreError, MemoryAgentStore};
pub use auth::{AuthConfig, EnvError as AuthEnvError};
pub use state::{build_app_state, memory_app_state, AppState, BuildOptions};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "TrustLoopGuard API",
        version = "0.0.1",
        description = "Real-time guardrail runtime for AI agents.",
        license(name = "Apache-2.0"),
    ),
    paths(
        check,
        health,
        agents::upsert_agent,
        agents::get_agent,
        agents::delete_agent,
        agents::list_agents,
    ),
    components(schemas(
        tl_core::CheckRequest,
        tl_core::Decision,
        tl_core::Verdict,
        tl_core::Channel,
        tl_core::Severity,
        tl_core::TriggeredPolicy,
        tl_core::ApiError,
        tl_core::ApiErrorCode,
        tl_core::AgentProfile,
        tl_core::AgentScope,
        tl_core::AgentAuthority,
        tl_core::AgentTone,
        tl_core::KnowledgeSource,
        agents::AgentListResponse,
    )),
    tags(
        (name = "guard", description = "Real-time guard checks"),
        (name = "agents", description = "Agent profile registration and lookup"),
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
pub async fn check(State(state): State<AppState>, Json(req): Json<CheckRequest>) -> Response {
    // Run the full pipeline: cache lookup → tier 1+2+3 with parallel
    // cancellation → aggregate. The handler ctx carries every
    // collaborator (profile resolver, cache, fuzzy, llm router).
    let decision = state.engine.check_async(&req, &state.handler_ctx).await;

    // Fire trace persistence non-blockingly. `try_send` returns Full if
    // the writer is backed up — we deliberately drop with a metric
    // rather than block the request path. PR 12's writer absorbs
    // 1000 traces in <5ms per call so this branch is rarely hit.
    #[cfg(feature = "postgres")]
    if let Some(tx) = state.trace_tx.as_ref() {
        let trace = tl_storage::TraceWrite {
            decision: decision.clone(),
            domain: req
                .domain
                .clone()
                .unwrap_or_else(|| "customer_support".to_string()),
        };
        if let Err(e) = tx.try_send(trace) {
            tracing::warn!(error = %e, "trace channel full or closed; dropped");
        }
    }

    Json(decision).into_response()
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
///
/// The agent CRUD endpoints are always wired now — `AppState` carries
/// the store, so there's no need for a separate constructor argument.
pub fn router(state: AppState, auth: Option<Arc<AuthConfig>>) -> Router {
    let public = Router::new().route("/health", get(health));

    let agent_state = AgentState {
        store: state.agent_store.clone(),
    };
    let agent_routes = Router::new()
        .route(
            "/v1/agents",
            post(agents::upsert_agent).get(agents::list_agents),
        )
        .route(
            "/v1/agents/:id",
            get(agents::get_agent).delete(agents::delete_agent),
        )
        .with_state(agent_state);

    let mut protected = Router::new()
        .route("/v1/check", post(check))
        .with_state(state)
        .merge(agent_routes);

    if let Some(cfg) = auth {
        protected = protected.layer(from_fn_with_state(cfg, auth::require_bearer));
    }

    public
        .merge(protected)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
}

/// Build the CORS layer.
///
/// `TL_CORS_ALLOWED_ORIGINS` is a comma-separated list of exact origins
/// (e.g. `https://app.example.com,https://staging.example.com`). When
/// unset, we default to `http://localhost:3000` so the bundled web
/// dashboard works out of the box. The special value `*` allows any
/// origin — useful for local hacking, never appropriate for prod.
///
/// The layer always permits the methods + headers SDK clients use:
/// `GET`, `POST`, `DELETE` for CRUD; `Authorization` and `Content-Type`
/// for the bearer-token + JSON body. We do NOT allow credentials
/// (cookies); auth is bearer-only by design.
fn cors_layer() -> CorsLayer {
    use axum::http::{header, Method};

    let raw = std::env::var("TL_CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let origins = if raw.trim() == "*" {
        AllowOrigin::any()
    } else {
        let parsed: Vec<_> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        AllowOrigin::list(parsed)
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}
