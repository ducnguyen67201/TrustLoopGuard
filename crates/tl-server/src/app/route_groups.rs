use std::sync::Arc;
use std::time::Duration;

use axum::{
    routing::{get, patch, post, put},
    Router,
};

mod gateway_routes;

use crate::{
    agents, analytics, auth_user, authorization, budget_alerts, dashboard_admin, environments,
    financial, github_integration, human_review, knowledge_sources, label_policy, llm_pricing,
    llm_usage, policies, redteam, runs, team, tool_metadata, traces, AgentState, AppState,
    AuthUserState, LabelPolicyState, PolicyState, ToolMetadataState,
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

    // Public, token-authenticated red-team report read. The token is the bearer
    // capability, so this route sits outside the `/v1/*` API-key layer; a
    // prospect can open a shared report without a dashboard account.
    // Per-token cap on the unauthenticated read: 60 requests / 60s per link.
    let report_rate_limiter =
        Arc::new(redteam::ReportRateLimiter::new(Duration::from_secs(60), 60));
    let public_report_routes = Router::new()
        .route(
            "/v1/redteam/reports/:token",
            get(redteam::get_public_report),
        )
        .with_state(redteam::PublicReportState {
            store: state.redteam_job_store.clone(),
            report_share_store: state.redteam_report_share_store.clone(),
            rate_limiter: report_rate_limiter,
        });

    let github_webhook_routes = Router::new()
        .route(
            "/v1/github-integration/webhooks",
            post(github_integration::webhooks::github_webhook),
        )
        .with_state(github_integration_state(state));

    Router::new()
        .route("/health", get(crate::api::guard::health))
        .merge(auth_user_routes)
        .merge(public_report_routes)
        .merge(github_webhook_routes)
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

pub(super) fn tool_metadata_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/tool-metadata",
            post(tool_metadata::upsert_tool_metadata).get(tool_metadata::list_tool_metadata),
        )
        .route(
            "/v1/tool-metadata/:tool",
            get(tool_metadata::get_tool_metadata).delete(tool_metadata::delete_tool_metadata),
        )
        .with_state(ToolMetadataState {
            store: state.tool_metadata_store.clone(),
            team_store: state.team_store.clone(),
        })
}

pub(super) fn authorization_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/authorization/approvals",
            get(authorization::list_approvals),
        )
        .route(
            "/v1/authorization/approvals/:id",
            get(authorization::get_approval),
        )
        .route(
            "/v1/authorization/approvals/:id/decide",
            post(authorization::decide_approval),
        )
        .route(
            "/v1/authorization/grants",
            get(authorization::list_grants).post(authorization::create_grant),
        )
        .route(
            "/v1/authorization/grants/:id/revoke",
            post(authorization::revoke_grant),
        )
        .route(
            "/v1/authorization/leases/:id/complete",
            post(authorization::complete_lease),
        )
        .route(
            "/v1/authorization/receipts/:id",
            get(authorization::get_receipt),
        )
        .with_state(authorization::AuthorizationState {
            store: state.authorization_store.clone(),
            coordinator: state.authorization_coordinator.clone(),
            team_store: state.team_store.clone(),
        })
}

pub(super) fn label_policy_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/label-policies",
            post(label_policy::upsert_label_policy).get(label_policy::list_label_policies),
        )
        .route(
            "/v1/label-policies/:origin",
            get(label_policy::get_label_policy).delete(label_policy::delete_label_policy),
        )
        .with_state(LabelPolicyState {
            store: state.label_policy_store.clone(),
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
    let guardrails = Router::new()
        .route(
            "/v1/agents/:id/guardrails/generate",
            post(policies::generate_guardrails),
        )
        .route("/v1/agents/:id/guardrails", get(policies::list_guardrails))
        // Static (preventive) policies from the workflow analyzer — the
        // no-runnable-target twin of harden.
        .route(
            "/v1/agents/:id/redteam/static-policies",
            post(redteam::generate_static_policies),
        )
        .with_state(policies::GuardrailState {
            agent_store: state.agent_store.clone(),
            policy_store: state.policy_store.clone(),
            environment_store: state.environment_store.clone(),
            draft_llm: draft_llm.clone(),
            draft_model: draft_model.clone(),
        });

    // Attack-vector generation lives in the private runner; Rust persists each
    // plan, so this state carries the durable plan store too.
    let planner = redteam::RedteamRunnerClient::from_env()
        .map(|client| Arc::new(client) as Arc<dyn redteam::RedteamPlanner>);
    let plans = Router::new()
        .route(
            "/v1/agents/:id/redteam/plan",
            post(redteam::plan_attack_vectors),
        )
        .route("/v1/agents/:id/redteam/plans", get(redteam::list_plans))
        .route(
            "/v1/redteam/plans/:id",
            axum::routing::delete(redteam::delete_plan),
        )
        .with_state(redteam::PlanState {
            agent_store: state.agent_store.clone(),
            plan_store: state.redteam_plan_store.clone(),
            environment_store: state.environment_store.clone(),
            planner,
        });

    guardrails.merge(plans)
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

pub(super) fn financial_routes(state: &AppState, gateway_seal_key: [u8; 32]) -> Router {
    let payment_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("financial payment HTTP client configuration should be valid");
    let executor: Arc<dyn financial::FinancialExecutor> =
        state.financial_executor.clone().unwrap_or_else(|| {
            Arc::new(financial::PaymentHttpFinancialExecutor::new(
                state.gateway_store.clone(),
                gateway_seal_key,
                payment_client,
            ))
        });
    let service = financial::FinancialAuthorizationService::with_policy_store_and_executor(
        state.financial_store.clone(),
        state.policy_store.clone(),
        state.authorization_coordinator.as_ref().clone(),
        executor,
    )
    .with_budget_alerts(budget_alerts::BudgetAlertRuntime {
        store: state.budget_alert_store.clone(),
        settings: state.settings_store.clone(),
        delivery_tx: state.budget_alert_tx.clone(),
    });
    Router::new()
        .route(
            "/v1/financial/actions",
            post(financial::create_action).get(financial::list_actions),
        )
        .route(
            "/v1/financial/agentic-payments/authorize",
            post(financial::authorize_agentic_payment),
        )
        .route(
            "/v1/financial/agentic-payments/:id",
            get(financial::get_agentic_payment),
        )
        .route(
            "/v1/financial/agentic-payments/:id/commit",
            post(financial::commit_agentic_payment),
        )
        .route(
            "/v1/financial/agentic-payments/:id/rollback",
            post(financial::rollback_agentic_payment),
        )
        .route(
            "/v1/financial/agentic-payments/:id/receipt",
            get(financial::get_agentic_payment_receipt),
        )
        .route(
            "/v1/financial/policies",
            post(financial::create_policy).get(financial::list_policies),
        )
        .route("/v1/financial/receipts/:id", get(financial::get_receipt))
        .route("/v1/financial/actions/:id", get(financial::get_action))
        .route(
            "/v1/financial/actions/:id/outcomes",
            post(financial::record_action_outcome).get(financial::list_action_outcomes),
        )
        .route(
            "/v1/financial/actions/:id/execute",
            post(financial::execute_action),
        )
        .with_state(financial::FinancialState { service })
}

pub(super) fn budget_alert_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/financial/budget-alerts",
            post(budget_alerts::create_budget_alert).get(budget_alerts::list_budget_alerts),
        )
        .route(
            "/v1/financial/budget-alerts/:id",
            patch(budget_alerts::update_budget_alert).delete(budget_alerts::delete_budget_alert),
        )
        .route(
            "/v1/financial/budget-alerts/:id/firings",
            get(budget_alerts::list_budget_alert_firings),
        )
        .with_state(budget_alerts::BudgetAlertApiState {
            store: state.budget_alert_store.clone(),
            policy_store: state.policy_store.clone(),
            team_store: state.team_store.clone(),
        })
}

pub(super) fn llm_usage_routes(state: &AppState) -> Router {
    Router::new()
        .route("/v1/llm-usage", get(llm_usage::list_llm_usage))
        .with_state(llm_usage::LlmUsageState {
            store: state.llm_usage_store.clone(),
            pricing_store: state.llm_pricing_store.clone(),
        })
}

pub(super) fn llm_pricing_routes(state: &AppState) -> Router {
    Router::new()
        .route("/v1/llm-pricing", get(llm_pricing::list_llm_pricing))
        .route(
            "/v1/llm-pricing/:model",
            put(llm_pricing::put_llm_price).delete(llm_pricing::delete_llm_price),
        )
        .with_state(llm_pricing::LlmPricingState {
            store: state.llm_pricing_store.clone(),
            team_store: state.team_store.clone(),
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

pub(super) fn redteam_routes(state: &AppState) -> Router {
    Router::new()
        .route("/v1/redteam/dispatch", post(redteam::dispatch_job))
        .route("/v1/redteam/jobs", get(redteam::list_jobs))
        .route("/v1/redteam/attacks", get(redteam::list_attack_records))
        .route("/v1/redteam/jobs/:id", get(redteam::get_job))
        .route("/v1/redteam/jobs/:id/report", get(redteam::get_report))
        .route("/v1/redteam/jobs/:id/harden", post(redteam::harden_job))
        .route("/v1/redteam/jobs/:id/cancel", post(redteam::cancel_job))
        .route("/v1/redteam/reports", post(redteam::create_report))
        .route(
            "/v1/redteam/reports/:token/revoke",
            post(redteam::revoke_report),
        )
        .with_state(redteam::RedteamState {
            store: state.redteam_job_store.clone(),
            agent_store: Some(state.agent_store.clone()),
            environment_store: state.environment_store.clone(),
            report_share_store: state.redteam_report_share_store.clone(),
            dispatch_tx: state.redteam_dispatch_tx.clone(),
            policy_store: state.policy_store.clone(),
            llm: state.handler_ctx.llm.clone(),
        })
}

pub(super) fn github_integration_routes(state: &AppState) -> Router {
    Router::new()
        .route(
            "/v1/github-integration/install-url",
            post(github_integration::handlers::install_url),
        )
        .route(
            "/v1/github-integration/callback",
            post(github_integration::handlers::callback),
        )
        .route(
            "/v1/github-integration/repositories",
            get(github_integration::handlers::repositories),
        )
        .route(
            "/v1/github-integration/connections",
            get(github_integration::handlers::list_connections)
                .post(github_integration::handlers::create_connection),
        )
        .route(
            "/v1/github-integration/connections/:id",
            axum::routing::delete(github_integration::handlers::disconnect_connection),
        )
        .route(
            "/v1/github-integration/jobs",
            post(github_integration::handlers::create_job),
        )
        .route(
            "/v1/github-integration/jobs/:id",
            get(github_integration::handlers::get_job),
        )
        .route(
            "/v1/github-integration/jobs/:id/approve",
            post(github_integration::handlers::approve_job),
        )
        .route(
            "/v1/github-integration/jobs/:id/cancel",
            post(github_integration::handlers::cancel_job),
        )
        .with_state(github_integration_state(state))
}

fn github_integration_state(state: &AppState) -> github_integration::GitHubIntegrationState {
    let github = github_integration::ReqwestGitHubClient::from_env()
        .ok()
        .map(|client| Arc::new(client) as Arc<dyn github_integration::GitHubClient>);
    let llm = tl_llm::OpenAiClient::from_env()
        .ok()
        .map(|client| Arc::new(client) as Arc<dyn tl_llm::LlmClient>);
    github_integration::GitHubIntegrationState {
        store: state.github_integration_store.clone(),
        team_store: state.team_store.clone(),
        agent_store: state.agent_store.clone(),
        environment_store: state.environment_store.clone(),
        trace_store: state.trace_store.clone(),
        github,
        llm,
        model: std::env::var("TL_GITHUB_INTEGRATION_MODEL")
            .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        worker_tx: state.github_integration_tx.clone(),
    }
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
        .route(
            "/v1/settings",
            get(dashboard_admin::get_settings).patch(dashboard_admin::update_settings),
        )
        .route(
            "/v1/environments/:id/checker-modes",
            get(dashboard_admin::get_environment_checker_modes)
                .put(dashboard_admin::put_environment_checker_modes),
        )
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
    gateway_routes::gateway_routes(state, gateway_seal_key)
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
