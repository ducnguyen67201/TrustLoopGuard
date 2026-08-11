//! Application state + boot wiring.
//!
//! `AppState` aggregates the runtime objects every request handler needs.
//! The module tree separates state shape, memory wiring, environment gates,
//! optional Postgres wiring, and worker bootstrapping.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tl_cache::MokaCache;
use tl_engine::{Engine, EventPipelineCtx, FuzzyChecker, HandlerCtx, NoOpFuzzyChecker};
use tl_llm::{LlmRouteKind, LlmRouter, RouterBuildError, RouterConfig};
use tl_policy::Policy;
#[cfg(feature = "postgres")]
use tl_storage::EscalationRepo;
use tokio::sync::mpsc as tokio_mpsc;

use crate::escalation::{
    spawn_escalation_worker, spawn_webhook_delivery_worker, EscalationConfig, EscalationPayload,
    RetryPolicy,
};
use crate::github_integration::{
    spawn_github_integration_worker, GitHubIntegrationMessage, GitHubIntegrationStore,
    ReqwestGitHubClient,
};
use crate::redteam::{
    spawn_dispatch_worker, DispatchConfig, DispatchJob, RedteamJobStore, RedteamRunnerClient,
};

pub mod app_state;
mod env;
pub mod memory;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "postgres")]
mod postgres_adapters;

pub use app_state::{AppState, BuildOptions};
pub use memory::memory_app_state;

use env::password_auth_enabled_from_env;
#[cfg(not(feature = "postgres"))]
use memory::build_memory_layer;
#[cfg(feature = "postgres")]
use postgres::build_postgres_layer;

pub async fn build_app_state(opts: BuildOptions) -> Result<AppState> {
    // -- Policies --
    let policy_dir = opts
        .policy_dir
        .or_else(|| std::env::var("TL_POLICY_DIR").ok())
        .unwrap_or_else(|| "./policies".to_string());
    let policies = load_policies(Path::new(&policy_dir))?;
    tracing::info!(
        path = %policy_dir,
        content_count = policies.content.len(),
        family_count = policies.families.len(),
        "loaded tenant policies",
    );

    // -- LLM Router (optional) --
    let llm = build_llm_router()?;

    // -- Cache --
    let cache: Arc<MokaCache> = Arc::new(MokaCache::with_defaults());

    // -- Postgres-backed pieces (or in-memory fallback) --
    #[cfg(feature = "postgres")]
    let (
        agent_store,
        profile_resolver,
        policy_store,
        trace_store,
        run_store,
        evaluation_store,
        otel_store,
        analytics_store,
        human_review_store,
        financial_store,
        llm_usage_store,
        llm_pricing_store,
        budget_alert_store,
        knowledge_store,
        api_key_store,
        environment_store,
        settings_store,
        user_store,
        team_store,
        gateway_store,
        oauth_store,
        mcp_gateway_store,
        notification_store,
        notification_transport_configured,
        tool_metadata_store,
        tool_metadata_provider,
        authorization_store,
        label_policy_store,
        label_policy_provider,
        escalation_repo,
        redteam_job_store,
        redteam_plan_store,
        redteam_report_share_store,
        github_integration_store,
    ) = build_postgres_layer(opts.database_url, &policies, llm.clone()).await?;

    #[cfg(not(feature = "postgres"))]
    let (
        agent_store,
        profile_resolver,
        policy_store,
        trace_store,
        run_store,
        evaluation_store,
        otel_store,
        analytics_store,
        human_review_store,
        financial_store,
        llm_usage_store,
        llm_pricing_store,
        budget_alert_store,
        knowledge_store,
        api_key_store,
        environment_store,
        settings_store,
        user_store,
        team_store,
        gateway_store,
        oauth_store,
        mcp_gateway_store,
        notification_store,
        notification_transport_configured,
        tool_metadata_store,
        tool_metadata_provider,
        authorization_store,
        label_policy_store,
        label_policy_provider,
        redteam_job_store,
        redteam_plan_store,
        redteam_report_share_store,
        github_integration_store,
    ) = build_memory_layer(&policies);

    // -- Tier 2 fuzzy: stub by default. PR 6 left a real HnswFuzzyChecker
    // available; wiring it requires the embedder model on disk and
    // per-tenant index build, which is deferred to a follow-up.
    let fuzzy: Arc<dyn FuzzyChecker> = Arc::new(NoOpFuzzyChecker);

    // -- Engine + ctx --
    let engine = Arc::new(Engine::new(policies.content.clone()));
    let handler_ctx = HandlerCtx {
        profile_resolver,
        cache,
        fuzzy,
        llm: llm.clone(),
    };

    // -- Escalation worker (optional) --
    let escalation_tx = build_escalation_worker(
        #[cfg(feature = "postgres")]
        escalation_repo.clone(),
    );

    // -- Budget alert delivery worker (always on) --
    // Unlike escalations there is no single global URL to gate on:
    // each alert config carries its own webhook target, so the worker
    // spawns unconditionally and idles until a firing arrives. Shares
    // the escalations persistence table and retry policy.
    let budget_alert_tx = {
        let (tx, _handle) = spawn_webhook_delivery_worker(
            RetryPolicy::default(),
            1024,
            #[cfg(feature = "postgres")]
            escalation_repo,
        );
        tracing::info!("budget alert delivery worker spawned");
        Some(tx)
    };

    // -- Red-team dispatch worker (optional) --
    let redteam_dispatch_tx = build_dispatch_worker(redteam_job_store.clone());

    // -- GitHub-assisted installation worker (optional) --
    let github_integration_tx =
        build_github_integration_worker(github_integration_store.clone(), llm);

    let jwt_signer = crate::jwt::JwtSigner::from_env();
    if jwt_signer.is_some() {
        tracing::info!("JWT signer enabled (TL_JWT_SECRET configured)");
    } else {
        tracing::warn!(
            "TL_JWT_SECRET not configured — signup/login responses will not carry a jwt; \
             the web will fall back to header-forwarded identity via TL_API_KEY"
        );
    }

    let password_auth_enabled = password_auth_enabled_from_env();
    if password_auth_enabled {
        tracing::info!("username/password auth enabled for local development");
    } else {
        tracing::info!("username/password auth disabled; OAuth login is required");
    }
    tracing::info!("user approval gate enabled for authenticated dashboard users");

    let authorization_coordinator = Arc::new(crate::authorization::AuthorizationCoordinator::new(
        authorization_store.clone(),
        policy_store.clone(),
        Arc::new(crate::authorization::adapters::AuthorizationAdapterRegistry::new()),
    ));

    let state = AppState {
        engine,
        handler_ctx,
        // Live tool-metadata resolution (action semantics), label
        // resolution, provenance propagation, and deterministic checkers.
        // Checker enforcement modes default to off per workspace. Tool-metadata
        // lookup failure remains an always-enforced defer invariant.
        event_pipeline: Arc::new(EventPipelineCtx {
            tool_metadata: tool_metadata_provider,
            label_resolver: Arc::new(tl_engine::PolicyLabelResolver::new(label_policy_provider)),
            provenance_resolver: Arc::new(tl_engine::ProvenancePropagator),
            checkers: vec![
                Arc::new(tl_engine::InformationFlowChecker),
                Arc::new(tl_engine::MemoryChecker),
                Arc::new(tl_engine::ParameterAuthChecker),
                Arc::new(tl_engine::ValueLimitChecker),
                Arc::new(tl_engine::ApprovalChecker),
            ],
            composer: Arc::new(tl_engine::ModeAwareDecisionComposer),
            ..EventPipelineCtx::no_op()
        }),
        agent_store,
        policy_store,
        tool_metadata_store,
        authorization_store,
        authorization_coordinator,
        label_policy_store,
        trace_store,
        run_store,
        evaluation_store,
        otel_store,
        analytics_store,
        human_review_store,
        financial_store,
        financial_executor: None,
        llm_usage_store,
        llm_pricing_store,
        budget_alert_store,
        budget_alert_tx,
        knowledge_store,
        api_key_store,
        environment_store,
        settings_store,
        user_store,
        password_auth_enabled,
        // Self-service is open to every APPROVED user: the auth middleware's
        // approval gate (not this flag) excludes unapproved accounts, so
        // approved first-timers can create their workspace via onboarding.
        workspace_self_service_enabled: true,
        team_store,
        gateway_store,
        oauth_store,
        mcp_gateway_store,
        notification_store,
        notification_transport_configured,
        jwt_signer,
        escalation_tx,
        redteam_job_store,
        redteam_plan_store,
        redteam_report_share_store,
        redteam_dispatch_tx,
        github_integration_store,
        github_integration_tx,
    };
    let _gateway_session_worker = crate::gateway::spawn_gateway_session_worker(state.clone());
    tracing::info!("gateway session boundary worker spawned");
    Ok(state)
}

/// Spawn the in-process red-team dispatch worker when a runner URL is
/// configured. Returns `None` (dispatch disabled) when `REDTEAM_RUNNER_URL`
/// is unset, mirroring `build_escalation_worker`.
fn build_dispatch_worker(
    store: Arc<dyn RedteamJobStore>,
) -> Option<tokio_mpsc::Sender<DispatchJob>> {
    let runner = RedteamRunnerClient::from_env()?;
    let (tx, _handle) = spawn_dispatch_worker(Arc::new(runner), store, DispatchConfig::default());
    tracing::info!("redteam dispatch worker spawned");
    Some(tx)
}

fn build_github_integration_worker(
    store: Arc<dyn GitHubIntegrationStore>,
    llm: Arc<LlmRouter>,
) -> Option<tokio_mpsc::Sender<GitHubIntegrationMessage>> {
    let github = match ReqwestGitHubClient::from_env() {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::info!(
                error = %error,
                "github integration disabled; GitHub App config incomplete"
            );
            return None;
        }
    };
    if !llm.has_workload_route(LlmRouteKind::GitHubIntegration) {
        tracing::info!("github integration disabled; canonical GitHub LLM route is unavailable");
        return None;
    }
    let tx = spawn_github_integration_worker(store, github, llm);
    tracing::info!("github integration worker spawned");
    Some(tx)
}

fn build_escalation_worker(
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) -> Option<tokio_mpsc::Sender<EscalationPayload>> {
    let url = std::env::var("TL_ESCALATION_WEBHOOK_URL").ok()?;
    if url.trim().is_empty() {
        return None;
    }
    let cfg = EscalationConfig::new(url.clone());
    let (tx, _handle) = spawn_escalation_worker(
        cfg,
        #[cfg(feature = "postgres")]
        repo,
    );
    tracing::info!(url, "escalation worker spawned");
    Some(tx)
}

#[derive(Debug, Default)]
pub(super) struct LoadedPolicies {
    pub(super) content: Vec<Policy>,
    pub(super) families: Vec<tl_policy::FamilyPolicy>,
}

fn load_policies(dir: &Path) -> Result<LoadedPolicies> {
    if !dir.exists() {
        tracing::warn!(path = %dir.display(), "policy dir not found; running with no tenant policies");
        return Ok(LoadedPolicies::default());
    }
    let mut out = LoadedPolicies::default();
    for entry in std::fs::read_dir(dir).context("read policy dir")? {
        let entry = entry?;
        let p = entry.path();
        // Skip subdirectories (e.g. `policies/agents/`); top-level
        // files only. Skip schema files and non-yaml extensions.
        if !p.is_file() {
            continue;
        }
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        let yaml = std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
        match tl_policy::load_any_str(&yaml) {
            Ok(tl_policy::AnyPolicy::Content(policy)) => out.content.push(policy),
            Ok(tl_policy::AnyPolicy::Family(policy)) => out.families.push(policy),
            Err(e) => {
                tracing::warn!(
                    path = %p.display(),
                    error = %e,
                    "skipping invalid policy file"
                );
            }
        }
    }
    Ok(out)
}

fn build_llm_router() -> Result<Arc<LlmRouter>> {
    let config = RouterConfig::bundled().context("parse bundled llm-routing manifest")?;
    match LlmRouter::from_config(&config) {
        Ok(router) => {
            tracing::info!("bundled llm-routing manifest loaded");
            Ok(Arc::new(router))
        }
        Err(RouterBuildError::MissingEnv(name)) => {
            tracing::info!(
                credential_env = name,
                "LLM routes disabled because provider credential is not configured"
            );
            Ok(Arc::new(LlmRouter::empty()))
        }
        Err(error) => Err(anyhow::anyhow!(error)).context("build bundled llm-routing manifest"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use tl_core::PolicyFamily;

    use super::load_policies;

    #[test]
    fn local_policy_directory_keeps_family_policies() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("featherlane-ai-policy-load-{unique}"));
        std::fs::create_dir(&directory).expect("create policy directory");
        let path = directory.join("command-policy.yaml");
        std::fs::write(
            &path,
            r#"
family: tool
id: local-command-policy
when: { side_effects: [shell_exec] }
match:
  fact: { key: shell.risk, equals: filesystem_recursive_delete }
action: deny
reason: Local shell policy.
"#,
        )
        .expect("write policy");

        let loaded = load_policies(&directory).expect("load policies");
        std::fs::remove_dir_all(&directory).expect("remove policy directory");

        assert!(loaded.content.is_empty());
        assert_eq!(loaded.families.len(), 1);
        assert_eq!(loaded.families[0].family(), PolicyFamily::Tool);
    }
}
