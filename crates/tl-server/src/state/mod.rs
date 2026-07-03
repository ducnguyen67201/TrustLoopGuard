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
use tl_llm::{LlmRouter, RouterConfig};
use tl_policy::Policy;
#[cfg(feature = "postgres")]
use tl_storage::EscalationRepo;
use tokio::sync::mpsc as tokio_mpsc;

use crate::escalation::{spawn_escalation_worker, EscalationConfig, EscalationPayload};
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

use env::{hosted_user_approval_required_from_env, password_auth_enabled_from_env};
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
        count = policies.len(),
        "loaded tenant policies",
    );

    // -- LLM Router (optional) --
    let llm = build_llm_router(opts.llm_config_path.as_deref());

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
        analytics_store,
        human_review_store,
        knowledge_store,
        api_key_store,
        environment_store,
        settings_store,
        user_store,
        team_store,
        gateway_store,
        tool_metadata_store,
        tool_metadata_provider,
        label_policy_store,
        label_policy_provider,
        escalation_repo,
        redteam_job_store,
        redteam_plan_store,
        redteam_report_share_store,
    ) = build_postgres_layer(opts.database_url, &policies).await?;

    #[cfg(not(feature = "postgres"))]
    let (
        agent_store,
        profile_resolver,
        policy_store,
        trace_store,
        run_store,
        analytics_store,
        human_review_store,
        knowledge_store,
        api_key_store,
        environment_store,
        settings_store,
        user_store,
        team_store,
        gateway_store,
        tool_metadata_store,
        tool_metadata_provider,
        label_policy_store,
        label_policy_provider,
        redteam_job_store,
        redteam_plan_store,
        redteam_report_share_store,
    ) = build_memory_layer(&policies);

    // -- Tier 2 fuzzy: stub by default. PR 6 left a real HnswFuzzyChecker
    // available; wiring it requires the embedder model on disk and
    // per-tenant index build, which is deferred to a follow-up.
    let fuzzy: Arc<dyn FuzzyChecker> = Arc::new(NoOpFuzzyChecker);

    // -- Engine + ctx --
    let engine = Arc::new(Engine::new(policies));
    let handler_ctx = HandlerCtx {
        profile_resolver,
        cache,
        fuzzy,
        llm,
    };

    // -- Escalation worker (optional) --
    let escalation_tx = build_escalation_worker(
        #[cfg(feature = "postgres")]
        escalation_repo,
    );

    // -- Red-team dispatch worker (optional) --
    let redteam_dispatch_tx = build_dispatch_worker(redteam_job_store.clone());

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
    let hosted_user_approval_required = hosted_user_approval_required_from_env();
    if hosted_user_approval_required {
        tracing::info!("hosted user approval gate enabled");
    }

    Ok(AppState {
        engine,
        handler_ctx,
        // Live tool-metadata resolution (action semantics), label
        // resolution, provenance propagation, and deterministic checkers.
        // Checker enforcement modes default to off per workspace, so the
        // decision is untouched unless a workspace opts in.
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
        label_policy_store,
        trace_store,
        run_store,
        analytics_store,
        human_review_store,
        knowledge_store,
        api_key_store,
        environment_store,
        settings_store,
        user_store,
        password_auth_enabled,
        hosted_user_approval_required,
        // Self-service is open to every APPROVED user: the auth middleware's
        // approval gate (not this flag) excludes unapproved accounts, so
        // approved first-timers can create their workspace via onboarding
        // even on hosted deployments.
        workspace_self_service_enabled: true,
        team_store,
        gateway_store,
        jwt_signer,
        escalation_tx,
        redteam_job_store,
        redteam_plan_store,
        redteam_report_share_store,
        redteam_dispatch_tx,
    })
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

fn load_policies(dir: &Path) -> Result<Vec<Policy>> {
    if !dir.exists() {
        tracing::warn!(path = %dir.display(), "policy dir not found; running with no tenant policies");
        return Ok(vec![]);
    }
    let mut out = vec![];
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
            Ok(tl_policy::AnyPolicy::Content(policy)) => out.push(policy),
            Ok(tl_policy::AnyPolicy::Family(policy)) => {
                // Family policies parse and validate but have no runtime
                // evaluation path yet; a clear skip beats a misleading
                // "invalid policy" warning from the content parser.
                tracing::warn!(
                    path = %p.display(),
                    policy_id = policy.id(),
                    "skipping family policy: runtime evaluation is not implemented yet"
                );
            }
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

fn build_llm_router(explicit: Option<&str>) -> Arc<LlmRouter> {
    let path = explicit
        .map(String::from)
        .or_else(|| std::env::var("TL_LLM_CONFIG").ok())
        .unwrap_or_else(|| "./config/llm-routing.toml".to_string());

    if !Path::new(&path).exists() {
        tracing::info!(
            path,
            "no llm-routing config; running with empty LlmRouter (Tier 3 disabled)"
        );
        return Arc::new(LlmRouter::empty());
    }

    match RouterConfig::from_path(&path) {
        Ok(cfg) => match LlmRouter::from_config(&cfg) {
            Ok(router) => {
                tracing::info!(path, "llm-routing config loaded");
                Arc::new(router)
            }
            Err(e) => {
                tracing::warn!(
                    path,
                    error = %e,
                    "llm-routing config rejected by router; falling back to empty"
                );
                Arc::new(LlmRouter::empty())
            }
        },
        Err(e) => {
            tracing::warn!(path, error = %e, "failed to parse llm-routing config; falling back to empty");
            Arc::new(LlmRouter::empty())
        }
    }
}
