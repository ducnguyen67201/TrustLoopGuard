//! HTTP routes and OpenAPI doc. Split from `main.rs` so `tl-codegen` can
//! pull `ApiDoc` without booting the runtime.

use std::{sync::Arc, time::Instant};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{from_fn, from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode, CheckRequest, DEFAULT_WORKSPACE_ID};
use utoipa::OpenApi;

pub mod agents;
pub mod auth;
pub mod auth_user;
pub mod dashboard_admin;
pub mod escalation;
pub mod jwt;
pub mod knowledge_sources;
pub mod policies;
pub mod state;
pub mod team;
pub mod traces;
pub use agents::{AgentState, AgentStore, AgentStoreError, MemoryAgentStore};
pub use auth::{AuthConfig, EnvError as AuthEnvError};
pub use auth_user::{AuthUserState, MemoryUserStore, UserStore, UserStoreError};
pub use dashboard_admin::{ApiKeyStore, DashboardAdminState, SettingsStore};
pub use escalation::{spawn_escalation_worker, EscalationConfig, EscalationPayload, RetryPolicy};
pub use policies::{GuardrailState, MemoryPolicyStore, PolicyState, PolicyStore, PolicyStoreError};
pub use state::{build_app_state, memory_app_state, AppState, BuildOptions};
pub use team::{MemoryTeamStore, TeamState, TeamStore, TeamStoreError};

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
        policies::batch_set_policy_enabled,
        policies::delete_policy,
        policies::draft_policy,
        policies::generate_guardrails,
        policies::list_guardrails,
        traces::list_traces,
        dashboard_admin::list_api_keys,
        dashboard_admin::create_api_key,
        dashboard_admin::get_settings,
        knowledge_sources::list_knowledge_sources,
        knowledge_sources::create_knowledge_source,
        knowledge_sources::get_knowledge_source_file,
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
        tl_core::PolicyBatchSetEnabledRequest,
        tl_core::PolicyBatchSetEnabledResponse,
        tl_core::PolicySummary,
        tl_core::PolicyDraft,
        tl_core::PolicyDraftRequest,
        tl_core::PolicyDraftResponse,
        tl_core::PolicyMatchType,
        tl_core::PolicyAction,
        tl_core::GuardrailGenerateResponse,
        tl_core::GuardrailListResponse,
        tl_core::TraceSummary,
        tl_core::TraceListResponse,
        tl_core::DashboardApiKey,
        tl_core::ApiKeyListResponse,
        tl_core::CreateApiKeyRequest,
        tl_core::CreateApiKeyResponse,
        tl_core::WorkspaceSettings,
        tl_core::DashboardKnowledgeSourceKind,
        tl_core::KnowledgeSourceStatus,
        tl_core::KnowledgeFileInput,
        tl_core::KnowledgeFileMetadata,
        tl_core::KnowledgeSourceDocument,
        tl_core::KnowledgeSourceListResponse,
        tl_core::CreateKnowledgeSourceRequest,
        tl_core::KnowledgeSourceFileResponse,
        tl_core::AuthRequest,
        tl_core::AuthResponse,
        tl_core::ChangePasswordRequest,
        tl_core::WorkspaceRole,
        tl_core::InviteStatus,
        tl_core::WorkspaceMember,
        tl_core::WorkspaceInvite,
        tl_core::CreateInviteRequest,
        tl_core::CreateInviteResponse,
        tl_core::MemberListResponse,
        tl_core::InviteListResponse,
        tl_core::MyWorkspace,
        tl_core::MyWorkspacesResponse,
        tl_core::CreateWorkspaceRequest,
    )),
    tags(
        (name = "guard", description = "Real-time guard checks"),
        (name = "agents", description = "Agent profile registration and lookup"),
        (name = "policies", description = "Policy authoring and validation"),
        (name = "traces", description = "Persisted guard decision traces"),
        (name = "api-keys", description = "Workspace runtime API keys"),
        (name = "settings", description = "Workspace runtime settings"),
        (name = "knowledge-sources", description = "Workspace knowledge source metadata and files"),
        (name = "auth", description = "Username/password authentication for self-hosters"),
        (name = "team", description = "Workspace team membership and invites"),
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
    log_api_error(status, code, &message);
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

pub(crate) fn log_api_error(status: StatusCode, code: ApiErrorCode, message: &str) {
    let status = status.as_u16();
    if status >= 500 {
        tracing::error!(status, code = ?code, error = %message, "api error response");
    } else if status >= 400 {
        tracing::warn!(status, code = ?code, error = %message, "api error response");
    } else {
        tracing::info!(status, code = ?code, message = %message, "api response");
    }
}

async fn log_http_response(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let content_type = header_value(request.headers(), "content-type");
    let content_length = header_value(request.headers(), "content-length");
    let user_agent = header_value(request.headers(), "user-agent");
    let workspace_id = header_value(request.headers(), "x-tlg-workspace-id");
    let user_id = header_value(request.headers(), "x-tlg-user-id");
    let has_authorization = request.headers().contains_key("authorization");

    tracing::info!(
        method = %method,
        path = %path,
        content_type = content_type.as_deref().unwrap_or(""),
        content_length = content_length.as_deref().unwrap_or(""),
        user_agent = user_agent.as_deref().unwrap_or(""),
        workspace_id = workspace_id.as_deref().unwrap_or(""),
        user_id = user_id.as_deref().unwrap_or(""),
        has_authorization,
        "http request"
    );

    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status();
    let latency_ms = started.elapsed().as_millis();

    match status.as_u16() {
        500..=599 => tracing::error!(
            method = %method,
            path = %path,
            status = status.as_u16(),
            latency_ms,
            outcome = "error",
            "http response"
        ),
        400..=499 => tracing::warn!(
            method = %method,
            path = %path,
            status = status.as_u16(),
            latency_ms,
            outcome = "warn",
            "http response"
        ),
        _ => tracing::info!(
            method = %method,
            path = %path,
            status = status.as_u16(),
            latency_ms,
            outcome = "ok",
            "http response"
        ),
    }

    response
}

fn header_value(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
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
    // Snapshot the signer up front so it survives the later
    // `.with_state(state)` move on the protected sub-router.
    let jwt_signer = state.jwt_signer.clone();
    let api_key_store = state.api_key_store.clone();

    let auth_user_state = AuthUserState {
        store: state.user_store.clone(),
        jwt_signer: jwt_signer.clone(),
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
            "/v1/policies/batch/enabled",
            patch(policies::batch_set_policy_enabled),
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

    let trace_routes = Router::new()
        .route("/v1/traces", get(traces::list_traces))
        .with_state(traces::TraceState {
            store: state.trace_store.clone(),
        });

    let dashboard_admin_routes = Router::new()
        .route(
            "/v1/api-keys",
            get(dashboard_admin::list_api_keys).post(dashboard_admin::create_api_key),
        )
        .route("/v1/settings", get(dashboard_admin::get_settings))
        .with_state(dashboard_admin::DashboardAdminState {
            api_key_store: state.api_key_store.clone(),
            settings_store: state.settings_store.clone(),
        });

    let knowledge_routes = Router::new()
        .route(
            "/v1/knowledge-sources",
            get(knowledge_sources::list_knowledge_sources)
                .post(knowledge_sources::create_knowledge_source),
        )
        .route(
            "/v1/knowledge-sources/:id/file",
            get(knowledge_sources::get_knowledge_source_file),
        )
        .with_state(knowledge_sources::KnowledgeState {
            store: state.knowledge_store.clone(),
        });

    let team_state = team::TeamState {
        store: state.team_store.clone(),
    };
    let team_routes = Router::new()
        .route("/v1/team/members", get(team::list_members))
        .route(
            "/v1/team/invites",
            get(team::list_invites).post(team::create_invite),
        )
        .route(
            "/v1/team/invites/:id",
            axum::routing::delete(team::revoke_invite),
        )
        .route(
            "/v1/team/my-workspaces",
            get(team::list_my_workspaces).post(team::create_my_workspace),
        )
        .with_state(team_state.clone());

    let mut protected = Router::new()
        .route("/v1/check", post(check))
        .route("/v1/policies/validate", post(policies::validate_policy))
        .with_state(state)
        .merge(agent_routes)
        .merge(policy_routes)
        .merge(guardrail_routes)
        .merge(trace_routes)
        .merge(dashboard_admin_routes)
        .merge(knowledge_routes)
        .merge(team_routes);

    if let Some(cfg) = auth {
        // Attach the JWT signer (if configured) so the middleware
        // accepts user-session tokens in addition to TL_API_KEY.
        let cfg = cfg.with_jwt(jwt_signer);
        let cfg = cfg.with_workspace_keys(Some(api_key_store));
        protected = protected.layer(from_fn_with_state(cfg, auth::require_bearer));
    }

    public.merge(protected).layer(from_fn(log_http_response))
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
