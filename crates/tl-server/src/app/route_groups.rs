use std::sync::Arc;

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    agents, analytics, auth_user, dashboard_admin, environments, gateway, human_review,
    knowledge_sources, policies, runs, team, traces, AgentState, AppState, AuthUserState,
    PolicyState,
};

pub(super) fn public_routes(
    state: &AppState,
    jwt_signer: Option<Arc<crate::jwt::JwtSigner>>,
) -> Router {
    let auth_user_state = AuthUserState {
        store: state.user_store.clone(),
        password_auth_enabled: state.password_auth_enabled,
        jwt_signer,
    };
    let auth_user_routes = Router::new()
        .route("/v1/auth/signup", post(auth_user::signup))
        .route("/v1/auth/login", post(auth_user::login))
        .route("/v1/auth/password", post(auth_user::change_password))
        .with_state(auth_user_state);

    Router::new()
        .route("/health", get(crate::api::guard::health))
        .merge(auth_user_routes)
}

pub(super) fn auth_identity_routes(
    state: &AppState,
    jwt_signer: Option<Arc<crate::jwt::JwtSigner>>,
) -> Router {
    Router::new()
        .route("/v1/identity/oauth-session", post(auth_user::oauth_session))
        .with_state(AuthUserState {
            store: state.user_store.clone(),
            password_auth_enabled: false,
            jwt_signer,
        })
}

pub(super) fn agent_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/agents",
            post(agents::upsert_agent).get(agents::list_agents),
        )
        .route(
            "/v1/agents/:id",
            get(agents::get_agent).delete(agents::delete_agent),
        )
        .with_state(AgentState {
            store: state.agent_store.clone(),
            policy_store: Some(state.policy_store.clone()),
        })
}

pub(super) fn policy_routes(
    state: &AppState,
    draft_llm: Option<Arc<dyn tl_llm::LlmClient>>,
    draft_model: String,
) -> Router {
    Router::new()
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
        .route("/v1/policies/ai-edit", post(policies::ai_edit_policy))
        .route(
            "/v1/policies/:id/versions",
            get(policies::list_policy_versions),
        )
        .route(
            "/v1/policies/:id/versions/:version",
            get(policies::get_policy_version),
        )
        .with_state(PolicyState {
            store: state.policy_store.clone(),
            environment_store: state.environment_store.clone(),
            draft_llm,
            draft_model,
        })
}

pub(super) fn guardrail_routes(
    state: &AppState,
    draft_llm: Option<Arc<dyn tl_llm::LlmClient>>,
    draft_model: String,
) -> Router {
    Router::new()
        .route(
            "/v1/agents/:id/guardrails/generate",
            post(policies::generate_guardrails),
        )
        .route("/v1/agents/:id/guardrails", get(policies::list_guardrails))
        .with_state(policies::GuardrailState {
            agent_store: state.agent_store.clone(),
            policy_store: state.policy_store.clone(),
            environment_store: state.environment_store.clone(),
            draft_llm,
            draft_model,
        })
}

pub(super) fn trace_routes(state: &AppState) -> Router {
    Router::new()
        .route("/v1/traces", get(traces::list_traces))
        .with_state(traces::TraceState {
            store: state.trace_store.clone(),
            environment_store: state.environment_store.clone(),
        })
}

pub(super) fn human_review_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/traces/:trace_id/review-events",
            get(human_review::list_review_events).post(human_review::create_review_event),
        )
        .route(
            "/v1/analytics/human-review",
            get(human_review::human_review_analytics),
        )
        .with_state(human_review::HumanReviewState {
            store: state.human_review_store.clone(),
        })
}

pub(super) fn analytics_routes(state: &AppState) -> Router {
    Router::new()
        .route("/v1/analytics/catalog", get(analytics::catalog))
        .route("/v1/analytics/query", post(analytics::query))
        .route(
            "/v1/analytics/views",
            get(analytics::list_views).post(analytics::create_view),
        )
        .route(
            "/v1/analytics/views/:id",
            patch(analytics::update_view).delete(analytics::delete_view),
        )
        .with_state(analytics::AnalyticsState {
            store: state.analytics_store.clone(),
            environment_store: state.environment_store.clone(),
            team_store: state.team_store.clone(),
        })
}

pub(super) fn run_routes(state: &AppState) -> Router {
    Router::new()
        .route("/v1/runs", get(runs::list_runs).post(runs::create_run))
        .route("/v1/runs/:id", get(runs::get_run).patch(runs::update_run))
        .route(
            "/v1/runs/:id/events",
            get(runs::list_run_events).post(runs::create_run_event),
        )
        .route("/v1/runs/:id/traces", get(runs::list_run_traces))
        .with_state(runs::RunState {
            store: state.run_store.clone(),
            environment_store: state.environment_store.clone(),
        })
}

pub(super) fn dashboard_admin_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/api-keys",
            get(dashboard_admin::list_api_keys).post(dashboard_admin::create_api_key),
        )
        .route(
            "/v1/api-keys/batch/revoke",
            patch(dashboard_admin::batch_revoke_api_keys),
        )
        .route("/v1/settings", get(dashboard_admin::get_settings))
        .with_state(dashboard_admin::DashboardAdminState {
            api_key_store: state.api_key_store.clone(),
            environment_store: state.environment_store.clone(),
            settings_store: state.settings_store.clone(),
            team_store: state.team_store.clone(),
        })
}

pub(super) fn environment_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/environments",
            get(environments::list_environments).post(environments::create_environment),
        )
        .route(
            "/v1/environments/:id",
            patch(environments::update_environment).delete(environments::delete_environment),
        )
        .with_state(environments::EnvironmentState {
            store: state.environment_store.clone(),
        })
}

pub(super) fn gateway_routes(state: &AppState, gateway_seal_key: [u8; 32]) -> Router {
    Router::new()
        .route(
            "/v1/gateway/provider-connections",
            get(gateway::list_gateway_provider_connections)
                .post(gateway::create_gateway_provider_connection),
        )
        .route(
            "/v1/gateway/provider-connections/:id",
            patch(gateway::patch_gateway_provider_connection),
        )
        .route(
            "/v1/enforcement-profiles",
            get(gateway::list_enforcement_profiles).post(gateway::create_enforcement_profile),
        )
        .route(
            "/v1/enforcement-profiles/:id",
            patch(gateway::patch_enforcement_profile),
        )
        .route(
            "/v1/gateway/routes",
            get(gateway::list_gateway_routes).post(gateway::create_gateway_route),
        )
        .route(
            "/v1/gateway/routes/:id",
            patch(gateway::patch_gateway_route),
        )
        .route(
            "/v1/gateway/:route_id/openai/chat/completions",
            post(gateway::proxy_openai_chat_completions),
        )
        .route(
            "/v1/gateway/:route_id/anthropic/v1/messages",
            post(gateway::proxy_anthropic_messages),
        )
        .with_state(gateway::GatewayState {
            app: state.clone(),
            store: state.gateway_store.clone(),
            http: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("gateway HTTP client"),
            seal_key: gateway_seal_key,
        })
}

pub(super) fn knowledge_routes(state: &AppState) -> Router {
    Router::new()
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
        })
}

pub(super) fn team_routes(state: &AppState) -> Router {
    Router::new()
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
        .with_state(team::TeamState {
            store: state.team_store.clone(),
            workspace_self_service_enabled: state.workspace_self_service_enabled,
        })
}
