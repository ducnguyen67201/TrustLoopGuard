//! HTTP routes and OpenAPI doc. Split from `main.rs` so `tl-codegen` can
//! pull `ApiDoc` without booting the runtime.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode, CheckRequest, DEFAULT_WORKSPACE_ID};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

pub mod agents;
pub mod auth;
pub mod auth_user;
pub mod escalation;
pub mod policies;
pub mod state;
pub use agents::{AgentState, AgentStore, AgentStoreError, MemoryAgentStore};
pub use auth::{AuthConfig, EnvError as AuthEnvError};
pub use auth_user::{AuthUserState, MemoryUserStore, UserStore, UserStoreError};
pub use escalation::{spawn_escalation_worker, EscalationConfig, EscalationPayload, RetryPolicy};
pub use policies::{GuardrailState, MemoryPolicyStore, PolicyState, PolicyStore, PolicyStoreError};
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
        policies::validate_policy,
        policies::upsert_policy,
        policies::list_policies,
        policies::get_policy,
        policies::set_policy_enabled,
        policies::delete_policy,
        policies::draft_policy,
        policies::generate_guardrails,
        policies::list_guardrails,
        auth_user::signup,
        auth_user::login,
        auth_user::change_password,
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
        tl_core::AgentListResponse,
        tl_core::AgentProfile,
        tl_core::AgentScope,
        tl_core::AgentAuthority,
        tl_core::AgentTone,
        tl_core::KnowledgeSource,
        tl_core::KnowledgeSourceKind,
        tl_core::PolicyValidateResponse,
        tl_core::PolicyValidationIssue,
        tl_core::PolicyDocument,
        tl_core::PolicyListResponse,
        tl_core::PolicySetEnabledRequest,
        tl_core::PolicySummary,
        tl_core::PolicyDraft,
        tl_core::PolicyDraftRequest,
        tl_core::PolicyDraftResponse,
        tl_core::PolicyMatchType,
        tl_core::PolicyAction,
        tl_core::GuardrailGenerateResponse,
        tl_core::GuardrailListResponse,
        tl_core::AuthRequest,
        tl_core::AuthResponse,
        tl_core::ChangePasswordRequest,
    )),
    tags(
        (name = "guard", description = "Real-time guard checks"),
        (name = "agents", description = "Agent profile registration and lookup"),
        (name = "policies", description = "Policy authoring and validation"),
        (name = "auth", description = "Username/password authentication for self-hosters"),
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
        (status = 500, description = "Runtime policy resolution failed", body = ApiError),
    ),
)]
pub async fn check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut req): Json<CheckRequest>,
) -> Response {
    let workspace_id = workspace_id_for_check(&headers, &req);
    req.workspace_id = Some(workspace_id.clone());
    let runtime_policies = match state.policy_store.list_enabled(&workspace_id).await {
        Ok(policies) => policies,
        Err(e) => {
            return api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                format!("runtime policy resolution failed: {e}"),
            );
        }
    };
    let policies: Vec<_> = runtime_policies
        .iter()
        .map(|policy| policy.as_ref().clone())
        .collect();

    // Run the full pipeline: cache lookup → tier 1+2+3 with parallel
    // cancellation → aggregate. The handler ctx carries every
    // collaborator (profile resolver, cache, fuzzy, llm router).
    let decision = state
        .engine
        .check_async_with_policies(&req, &state.handler_ctx, &policies)
        .await;

    // Fire trace persistence non-blockingly. `try_send` returns Full if
    // the writer is backed up — we deliberately drop with a metric
    // rather than block the request path. PR 12's writer absorbs
    // 1000 traces in <5ms per call so this branch is rarely hit.
    #[cfg(feature = "postgres")]
    if let Some(tx) = state.trace_tx.as_ref() {
        let trace = tl_storage::TraceWrite {
            decision: decision.clone(),
            workspace_id: workspace_id.clone(),
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

fn workspace_id_for_check(headers: &HeaderMap, req: &CheckRequest) -> String {
    headers
        .get("x-tlg-workspace-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            req.workspace_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| DEFAULT_WORKSPACE_ID.to_string())
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
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
    let auth_user_state = AuthUserState {
        store: state.user_store.clone(),
    };
    let auth_user_routes = Router::new()
        .route("/v1/auth/signup", post(auth_user::signup))
        .route("/v1/auth/login", post(auth_user::login))
        .route("/v1/auth/password", post(auth_user::change_password))
        .with_state(auth_user_state);

    let public = Router::new()
        .route("/health", get(health))
        .merge(auth_user_routes);

    let draft_llm = build_policy_draft_llm();
    let draft_model =
        std::env::var("TL_POLICY_DRAFT_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let agent_state = AgentState {
        store: state.agent_store.clone(),
        policy_store: Some(state.policy_store.clone()),
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

    let policy_state = PolicyState {
        store: state.policy_store.clone(),
        draft_llm: draft_llm.clone(),
        draft_model: draft_model.clone(),
    };
    let policy_routes = Router::new()
        .route(
            "/v1/policies",
            post(policies::upsert_policy).get(policies::list_policies),
        )
        .route(
            "/v1/policies/:id",
            get(policies::get_policy).delete(policies::delete_policy),
        )
        .route(
            "/v1/policies/:id/enabled",
            patch(policies::set_policy_enabled),
        )
        .route("/v1/policies/draft", post(policies::draft_policy))
        .with_state(policy_state);

    let guardrail_state = policies::GuardrailState {
        agent_store: state.agent_store.clone(),
        policy_store: state.policy_store.clone(),
        draft_llm,
        draft_model,
    };
    let guardrail_routes = Router::new()
        .route(
            "/v1/agents/:id/guardrails/generate",
            post(policies::generate_guardrails),
        )
        .route("/v1/agents/:id/guardrails", get(policies::list_guardrails))
        .with_state(guardrail_state);

    let mut protected = Router::new()
        .route("/v1/check", post(check))
        .route("/v1/policies/validate", post(policies::validate_policy))
        .with_state(state)
        .merge(agent_routes)
        .merge(policy_routes)
        .merge(guardrail_routes);

    if let Some(cfg) = auth {
        protected = protected.layer(from_fn_with_state(cfg, auth::require_bearer));
    }

    public.merge(protected).layer(TraceLayer::new_for_http())
}

/// Builds the LLM client used by `POST /v1/policies/draft`. Returns
/// `None` when `OPENAI_API_KEY` is unset — the draft handler then
/// surfaces a 503 to the caller. Kept separate from the Tier-3 router
/// because drafting is a one-shot authoring helper, not part of the
/// per-request evaluation pipeline.
fn build_policy_draft_llm() -> Option<Arc<dyn tl_llm::LlmClient>> {
    match tl_llm::OpenAiClient::from_env() {
        Ok(client) => {
            tracing::info!("policy-draft LLM enabled (OpenAI)");
            Some(Arc::new(client))
        }
        Err(e) => {
            tracing::info!(
                error = %e,
                "policy-draft LLM not configured; POST /v1/policies/draft will return 503"
            );
            None
        }
    }
}
