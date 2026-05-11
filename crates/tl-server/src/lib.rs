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
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

pub mod admin;
pub mod agents;
pub mod auth;
pub mod escalation;
pub mod state;
pub use agents::{AgentState, AgentStore, AgentStoreError, MemoryAgentStore};
#[cfg(feature = "postgres")]
pub use auth::AuthenticatedUser;
pub use auth::{AdminConfig, AuthConfig, AuthLayer, EnvError as AuthEnvError};
pub use escalation::{spawn_escalation_worker, EscalationConfig, EscalationPayload, RetryPolicy};
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
        admin::create_key,
        admin::list_keys,
        admin::revoke_key,
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
        admin::CreateKeyRequest,
        admin::CreateKeyResponse,
        admin::ApiKeyView,
        admin::ApiKeyListResponse,
    )),
    tags(
        (name = "guard", description = "Real-time guard checks"),
        (name = "agents", description = "Agent profile registration and lookup"),
        (name = "admin", description = "API key minting and revocation"),
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

    // Escalations: fire to the webhook worker on Escalate verdicts.
    // Same try_send semantics — channel full means we drop with a log;
    // the request path never blocks on webhook delivery.
    if decision.verdict == tl_core::Verdict::Escalate {
        if let Some(tx) = state.escalation_tx.as_ref() {
            let payload = escalation::EscalationPayload {
                trace_id: decision.trace_id.clone(),
                agent_id: req.agent_id.clone(),
                domain: req
                    .domain
                    .clone()
                    .unwrap_or_else(|| "customer_support".to_string()),
                decision: decision.clone(),
            };
            if let Err(e) = tx.try_send(payload) {
                tracing::warn!(error = %e, "escalation channel full or closed; dropped");
            }
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
/// `admin` gates `/v1/admin/*` separately so the dashboard can hold a
/// dedicated, rotatable secret server-side without sharing the per-user
/// `TL_API_KEY`. When `None`, the admin routes are still mounted but
/// unauthenticated — `main` logs a warning at boot so this is visible
/// in dev only.
pub fn router(
    state: AppState,
    auth: Option<Arc<AuthConfig>>,
    admin: Option<Arc<AdminConfig>>,
) -> Router {
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

    let admin_routes = build_admin_routes(&state, admin);

    let auth_layer = build_auth_layer(&state, auth);

    let mut protected = Router::new()
        .route("/v1/check", post(check))
        .with_state(state)
        .merge(agent_routes);

    if let Some(layer) = auth_layer {
        protected = protected.layer(from_fn_with_state(layer, auth::require_auth));
    }

    public
        .merge(protected)
        .merge(admin_routes)
        .layer(TraceLayer::new_for_http())
}

#[cfg(feature = "postgres")]
fn build_auth_layer(state: &AppState, auth: Option<Arc<AuthConfig>>) -> Option<Arc<AuthLayer>> {
    match state.api_key_repo.clone() {
        Some(repo) => Some(Arc::new(AuthLayer::with_repo(auth, repo))),
        None => auth.map(|c| Arc::new(AuthLayer::static_only(c))),
    }
}

#[cfg(not(feature = "postgres"))]
fn build_auth_layer(_state: &AppState, auth: Option<Arc<AuthConfig>>) -> Option<Arc<AuthLayer>> {
    auth.map(|c| Arc::new(AuthLayer::static_only(c)))
}

#[cfg(feature = "postgres")]
fn build_admin_routes(state: &AppState, admin: Option<Arc<AdminConfig>>) -> Router {
    use axum::routing::delete;

    let Some(repo) = state.api_key_repo.clone() else {
        // No Postgres at runtime — no admin surface to mount.
        return Router::new();
    };
    let admin_state = admin::AdminState { repo };
    let mut r = Router::new()
        .route(
            "/v1/admin/keys",
            post(admin::create_key).get(admin::list_keys),
        )
        .route("/v1/admin/keys/:id", delete(admin::revoke_key))
        .with_state(admin_state);
    if let Some(cfg) = admin {
        r = r.layer(from_fn_with_state(cfg, auth::require_admin_bearer));
    }
    r
}

#[cfg(not(feature = "postgres"))]
fn build_admin_routes(_state: &AppState, admin: Option<Arc<AdminConfig>>) -> Router {
    use axum::routing::delete;

    let admin_state = admin::AdminState;
    let mut r = Router::new()
        .route(
            "/v1/admin/keys",
            post(admin::create_key).get(admin::list_keys),
        )
        .route("/v1/admin/keys/:id", delete(admin::revoke_key))
        .with_state(admin_state);
    if let Some(cfg) = admin {
        r = r.layer(from_fn_with_state(cfg, auth::require_admin_bearer));
    }
    r
}
