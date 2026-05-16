//! Application state + boot wiring.
//!
//! `AppState` aggregates the runtime objects every request handler
//! needs: the engine, the orchestrator's `HandlerCtx`, an optional
//! trace channel, and the agent/policy stores. Construction lives in
//! [`build_app_state`] which reads the deployment's environment +
//! config files and wires concrete impls behind the trait surfaces.
//!
//! Two boot shapes:
//!
//! - **Memory-only** (default; what local dev runs). No Postgres
//!   connection, `MemoryAgentStore` for profiles, `MemoryPolicyStore`
//!   for policy authoring, no trace writer.
//!   Useful for `cargo run` and integration tests.
//!
//! - **Postgres** (with the `postgres` feature). Connects
//!   `DATABASE_URL`, runs Diesel migrations, spawns the batched trace
//!   writer, swaps `MemoryAgentStore` for an `AgentRepo` adapter.
//!
//! In either shape the LLM router is loaded from `TL_LLM_CONFIG`
//! when the file exists; missing/parse-failure falls back to
//! `LlmRouter::empty()` so Tier 3 reports `Skipped`.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tl_cache::MokaCache;
use tl_core::AgentProfile;
use tl_engine::{Engine, FuzzyChecker, HandlerCtx, NoOpFuzzyChecker, ProfileResolver};
use tl_llm::{LlmRouter, RouterConfig};
use tl_policy::Policy;

#[cfg(feature = "postgres")]
use crate::agents::AgentStoreError;
use crate::agents::{AgentStore, MemoryAgentStore};
#[cfg(feature = "postgres")]
use crate::auth_user::UserStoreError;
use crate::auth_user::{MemoryUserStore, UserStore};
use crate::dashboard_admin::{ApiKeyStore, MemoryApiKeyStore, MemorySettingsStore, SettingsStore};
use crate::escalation::{spawn_escalation_worker, EscalationConfig, EscalationPayload};
use crate::knowledge_sources::{KnowledgeStore, MemoryKnowledgeStore};
#[cfg(feature = "postgres")]
use crate::policies::PolicyStoreError;
use crate::policies::{MemoryPolicyStore, PolicyStore};
use crate::team::{MemoryTeamStore, TeamStore};
use crate::traces::{MemoryTraceStore, TraceStore};

#[cfg(feature = "postgres")]
use {
    base64::{engine::general_purpose::STANDARD, Engine as _},
    tl_storage::{
        connect_postgres, migrate_postgres, spawn_writer, AgentRepo, DashboardAdminRepo,
        EscalationRepo, KnowledgeRepo, NewKnowledgeFile, NewKnowledgeSource, PolicyRepo, TeamRepo,
        TraceRepo, TraceWrite, UserRepo, WriterConfig,
    },
    tokio::sync::mpsc,
};

// Always-on import for escalation_tx (works regardless of `postgres`).
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Engine>,
    pub handler_ctx: HandlerCtx,
    /// Channel into the background trace writer. `None` when the
    /// server runs without Postgres (no persistence).
    #[cfg(feature = "postgres")]
    pub trace_tx: Option<mpsc::Sender<TraceWrite>>,
    pub agent_store: Arc<dyn AgentStore>,
    pub policy_store: Arc<dyn PolicyStore>,
    pub trace_store: Arc<dyn TraceStore>,
    pub knowledge_store: Arc<dyn KnowledgeStore>,
    pub api_key_store: Arc<dyn ApiKeyStore>,
    pub settings_store: Arc<dyn SettingsStore>,
    /// Backing store for username/password accounts. Memory-only when
    /// the server runs without Postgres.
    pub user_store: Arc<dyn UserStore>,
    /// Backing store for workspace team membership + invites.
    pub team_store: Arc<dyn TeamStore>,
    /// Channel into the escalation webhook worker. `None` when no
    /// `TL_ESCALATION_WEBHOOK_URL` is configured — Escalate decisions
    /// are still produced, just never delivered downstream.
    pub escalation_tx: Option<tokio_mpsc::Sender<EscalationPayload>>,
}

#[derive(Default)]
pub struct BuildOptions {
    /// Path to the policy YAML directory. Defaults to `TL_POLICY_DIR`
    /// env var, then `./policies`.
    pub policy_dir: Option<String>,
    /// Path to the LlmRouter TOML config. Defaults to `TL_LLM_CONFIG`
    /// env var, then `./config/llm-routing.toml`. If the file is
    /// missing the router boots empty (Tier 3 → Skipped).
    pub llm_config_path: Option<String>,
    /// Postgres connection string. Defaults to `DATABASE_URL`. When
    /// unset, the server runs memory-only.
    #[cfg(feature = "postgres")]
    pub database_url: Option<String>,
}

/// Build an in-memory `AppState` from the given engine. Useful in
/// tests and for callers that construct the engine themselves
/// (e.g. plugging a custom `TierRunner` for deterministic mocks).
/// Skips all I/O — no Postgres, no llm-routing, no policy directory.
pub fn memory_app_state(engine: Arc<Engine>) -> AppState {
    let mem = Arc::new(MemoryAgentStore::new());
    let agent_store: Arc<dyn AgentStore> = mem.clone();
    let profile_resolver: Arc<dyn ProfileResolver> = mem;
    let policy_store: Arc<dyn PolicyStore> =
        Arc::new(MemoryPolicyStore::with_policies(engine.policies()));
    let cache: Arc<MokaCache> = Arc::new(MokaCache::with_defaults());
    let fuzzy: Arc<dyn FuzzyChecker> = Arc::new(NoOpFuzzyChecker);
    let llm = Arc::new(LlmRouter::empty());
    let handler_ctx = HandlerCtx {
        profile_resolver,
        cache,
        fuzzy,
        llm,
    };
    AppState {
        engine,
        handler_ctx,
        #[cfg(feature = "postgres")]
        trace_tx: None,
        agent_store,
        policy_store,
        trace_store: Arc::new(MemoryTraceStore),
        knowledge_store: Arc::new(MemoryKnowledgeStore::new()),
        api_key_store: Arc::new(MemoryApiKeyStore),
        settings_store: Arc::new(MemorySettingsStore),
        user_store: Arc::new(MemoryUserStore::new()),
        team_store: Arc::new(MemoryTeamStore::new()),
        escalation_tx: None,
    }
}

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
        knowledge_store,
        api_key_store,
        settings_store,
        user_store,
        team_store,
        trace_tx,
        escalation_repo,
    ) = build_postgres_layer(opts.database_url, &policies).await?;

    #[cfg(not(feature = "postgres"))]
    let (
        agent_store,
        profile_resolver,
        policy_store,
        trace_store,
        knowledge_store,
        api_key_store,
        settings_store,
        user_store,
        team_store,
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

    Ok(AppState {
        engine,
        handler_ctx,
        #[cfg(feature = "postgres")]
        trace_tx,
        agent_store,
        policy_store,
        trace_store,
        knowledge_store,
        api_key_store,
        settings_store,
        user_store,
        team_store,
        escalation_tx,
    })
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
        match tl_policy::load_str(&yaml) {
            Ok(policy) => out.push(policy),
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

#[cfg(feature = "postgres")]
#[allow(clippy::type_complexity)]
async fn build_postgres_layer(
    database_url: Option<String>,
    fallback_policies: &[Policy],
) -> Result<(
    Arc<dyn AgentStore>,
    Arc<dyn ProfileResolver>,
    Arc<dyn PolicyStore>,
    Arc<dyn TraceStore>,
    Arc<dyn KnowledgeStore>,
    Arc<dyn ApiKeyStore>,
    Arc<dyn SettingsStore>,
    Arc<dyn UserStore>,
    Arc<dyn TeamStore>,
    Option<mpsc::Sender<TraceWrite>>,
    Option<Arc<EscalationRepo>>,
)> {
    let url = database_url.or_else(|| std::env::var("DATABASE_URL").ok());

    let Some(url) = url else {
        tracing::warn!(
            "DATABASE_URL not set — running memory-only (no trace persistence, no profile durability)"
        );
        let mem = Arc::new(MemoryAgentStore::new());
        return Ok((
            mem.clone() as Arc<dyn AgentStore>,
            mem as Arc<dyn ProfileResolver>,
            Arc::new(MemoryPolicyStore::with_policies(fallback_policies)) as Arc<dyn PolicyStore>,
            Arc::new(MemoryTraceStore) as Arc<dyn TraceStore>,
            Arc::new(MemoryKnowledgeStore::new()) as Arc<dyn KnowledgeStore>,
            Arc::new(MemoryApiKeyStore) as Arc<dyn ApiKeyStore>,
            Arc::new(MemorySettingsStore) as Arc<dyn SettingsStore>,
            Arc::new(MemoryUserStore::new()) as Arc<dyn UserStore>,
            Arc::new(MemoryTeamStore::new()) as Arc<dyn TeamStore>,
            None,
            None,
        ));
    };

    migrate_postgres(&url)
        .await
        .map_err(|e| anyhow::anyhow!("migrate: {e}"))?;
    let pool = connect_postgres(&url, 20)
        .await
        .map_err(|e| anyhow::anyhow!("connect Postgres: {e}"))?;
    tracing::info!("Postgres connected and migrated");

    let repo = Arc::new(AgentRepo::new(pool.clone()));
    let adapter = PostgresAgentAdapter::new(repo);
    let policy_repo = Arc::new(PolicyRepo::new(pool.clone()));
    let policy_adapter = PostgresPolicyAdapter::new(policy_repo);
    let trace_adapter = PostgresTraceAdapter::new(Arc::new(TraceRepo::new(pool.clone())));
    let knowledge_adapter =
        PostgresKnowledgeAdapter::new(Arc::new(KnowledgeRepo::new(pool.clone())));
    let dashboard_admin_adapter =
        PostgresDashboardAdminAdapter::new(Arc::new(DashboardAdminRepo::new(pool.clone())));
    let user_repo = Arc::new(UserRepo::new(pool.clone()));
    let user_adapter = PostgresUserAdapter::new(user_repo);

    let team_adapter: Arc<dyn TeamStore> = Arc::new(crate::team::TeamRepoAdapter::new(
        TeamRepo::new(pool.clone()),
    ));

    let (tx, _handle) = spawn_writer(pool.clone(), WriterConfig::default());
    tracing::info!("trace writer spawned");

    let escalation_repo = Arc::new(EscalationRepo::new(pool));

    Ok((
        adapter.clone() as Arc<dyn AgentStore>,
        adapter as Arc<dyn ProfileResolver>,
        policy_adapter as Arc<dyn PolicyStore>,
        trace_adapter as Arc<dyn TraceStore>,
        knowledge_adapter as Arc<dyn KnowledgeStore>,
        dashboard_admin_adapter.clone() as Arc<dyn ApiKeyStore>,
        dashboard_admin_adapter as Arc<dyn SettingsStore>,
        user_adapter as Arc<dyn UserStore>,
        team_adapter,
        Some(tx),
        Some(escalation_repo),
    ))
}

#[cfg(not(feature = "postgres"))]
#[allow(clippy::type_complexity)]
fn build_memory_layer(
    policies: &[Policy],
) -> (
    Arc<dyn AgentStore>,
    Arc<dyn ProfileResolver>,
    Arc<dyn PolicyStore>,
    Arc<dyn TraceStore>,
    Arc<dyn KnowledgeStore>,
    Arc<dyn ApiKeyStore>,
    Arc<dyn SettingsStore>,
    Arc<dyn UserStore>,
    Arc<dyn TeamStore>,
) {
    let mem = Arc::new(MemoryAgentStore::new());
    (
        mem.clone() as Arc<dyn AgentStore>,
        mem as Arc<dyn ProfileResolver>,
        Arc::new(MemoryPolicyStore::with_policies(policies)) as Arc<dyn PolicyStore>,
        Arc::new(MemoryTraceStore) as Arc<dyn TraceStore>,
        Arc::new(MemoryKnowledgeStore::new()) as Arc<dyn KnowledgeStore>,
        Arc::new(MemoryApiKeyStore) as Arc<dyn ApiKeyStore>,
        Arc::new(MemorySettingsStore) as Arc<dyn SettingsStore>,
        Arc::new(MemoryUserStore::new()) as Arc<dyn UserStore>,
        Arc::new(MemoryTeamStore::new()) as Arc<dyn TeamStore>,
    )
}

// -- Trait adapters ---------------------------------------------------------
//
// Both `MemoryAgentStore` (this crate) and `tl_storage::AgentRepo` can
// satisfy the `tl_engine::ProfileResolver` trait. We implement the
// adapter here because `tl-engine` doesn't know about either type and
// `tl-storage` doesn't know about `tl-engine`.

#[async_trait]
impl ProfileResolver for MemoryAgentStore {
    async fn resolve(&self, workspace_id: &str, agent_id: &str) -> Option<Arc<AgentProfile>> {
        AgentStore::get(self, workspace_id, agent_id).await.ok()
    }
}

/// Adapter newtype: wraps `tl_storage::AgentRepo` so we can implement
/// `tl_engine::ProfileResolver` and our own `AgentStore` for it
/// without violating Rust's orphan rule (both the trait and the
/// inner type live in foreign crates).
#[cfg(feature = "postgres")]
pub struct PostgresAgentAdapter(pub Arc<AgentRepo>);

#[cfg(feature = "postgres")]
impl PostgresAgentAdapter {
    pub fn new(repo: Arc<AgentRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ProfileResolver for PostgresAgentAdapter {
    async fn resolve(&self, workspace_id: &str, agent_id: &str) -> Option<Arc<AgentProfile>> {
        self.0.get(workspace_id, agent_id).await.ok()
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl AgentStore for PostgresAgentAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        profile: &AgentProfile,
        source_yaml: &str,
    ) -> Result<(), AgentStoreError> {
        self.0
            .upsert(workspace_id, profile, source_yaml)
            .await
            .map_err(|e| AgentStoreError::Internal(e.to_string()))
    }

    async fn get(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Arc<AgentProfile>, AgentStoreError> {
        self.0
            .get(workspace_id, agent_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => AgentStoreError::NotFound,
                other => AgentStoreError::Internal(other.to_string()),
            })
    }

    async fn delete(&self, workspace_id: &str, agent_id: &str) -> Result<(), AgentStoreError> {
        self.0
            .delete(workspace_id, agent_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => AgentStoreError::NotFound,
                other => AgentStoreError::Internal(other.to_string()),
            })
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError> {
        self.0
            .list(workspace_id)
            .await
            .map_err(|e| AgentStoreError::Internal(e.to_string()))
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresPolicyAdapter(pub Arc<PolicyRepo>);

#[cfg(feature = "postgres")]
impl PostgresPolicyAdapter {
    pub fn new(repo: Arc<PolicyRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl PolicyStore for PostgresPolicyAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .upsert_in(workspace_id, policy, source_yaml)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))?;
        self.get(workspace_id, &policy.id).await
    }

    async fn get(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .get_record_in(workspace_id, policy_id)
            .await
            .map_or_else(
                |e| {
                    Err(match e {
                        tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                        other => PolicyStoreError::Internal(other.to_string()),
                    })
                },
                |row| {
                    Ok(tl_core::PolicyDocument {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        enabled: row.enabled,
                        source_yaml: row.source_yaml,
                    })
                },
            )
    }

    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records_in(workspace_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::PolicySummary {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        action: Some(policy_action(&row.policy.action)),
                        enabled: row.enabled,
                        owner_agent_id: row.owner_agent_id,
                    })
                    .collect()
            })
    }

    async fn list_enabled(&self, workspace_id: &str) -> Result<Vec<Arc<Policy>>, PolicyStoreError> {
        self.0
            .list_enabled_in(workspace_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn set_enabled(
        &self,
        workspace_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .set_enabled_in(workspace_id, policy_id, enabled)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })?;
        self.get(workspace_id, policy_id).await
    }

    async fn delete(&self, workspace_id: &str, policy_id: &str) -> Result<(), PolicyStoreError> {
        self.0
            .delete_in(workspace_id, policy_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })
    }

    async fn list_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records_for_agent(workspace_id, agent_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::PolicySummary {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        action: Some(policy_action(&row.policy.action)),
                        enabled: row.enabled,
                        owner_agent_id: row.owner_agent_id,
                    })
                    .collect()
            })
    }

    async fn delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, PolicyStoreError> {
        self.0
            .soft_delete_for_agent(workspace_id, agent_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresTraceAdapter(pub Arc<TraceRepo>);

#[cfg(feature = "postgres")]
impl PostgresTraceAdapter {
    pub fn new(repo: Arc<TraceRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl TraceStore for PostgresTraceAdapter {
    async fn list_recent(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::TraceSummary>, crate::traces::TraceStoreError> {
        self.0
            .list_recent(workspace_id, limit as i64)
            .await
            .map_err(|e| crate::traces::TraceStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::TraceSummary {
                        trace_id: row.trace_id.to_string(),
                        domain: row.domain,
                        decision: row.decision,
                        elapsed_ms: row.elapsed_ms,
                        payload: row.payload,
                        created_at: row.created_at.to_rfc3339(),
                    })
                    .collect()
            })
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresDashboardAdminAdapter(pub Arc<DashboardAdminRepo>);

#[cfg(feature = "postgres")]
impl PostgresDashboardAdminAdapter {
    pub fn new(repo: Arc<DashboardAdminRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ApiKeyStore for PostgresDashboardAdminAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::DashboardApiKey>, crate::dashboard_admin::DashboardAdminStoreError>
    {
        self.0
            .list_api_keys(workspace_id)
            .await
            .map_err(|e| crate::dashboard_admin::DashboardAdminStoreError::Internal(e.to_string()))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl SettingsStore for PostgresDashboardAdminAdapter {
    async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::WorkspaceSettings, crate::dashboard_admin::DashboardAdminStoreError> {
        self.0
            .get_settings(workspace_id)
            .await
            .map_err(|e| crate::dashboard_admin::DashboardAdminStoreError::Internal(e.to_string()))
            .map(|settings| settings.unwrap_or_else(crate::dashboard_admin::default_settings))
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresKnowledgeAdapter(pub Arc<KnowledgeRepo>);

#[cfg(feature = "postgres")]
impl PostgresKnowledgeAdapter {
    pub fn new(repo: Arc<KnowledgeRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl KnowledgeStore for PostgresKnowledgeAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::KnowledgeSourceDocument>, crate::knowledge_sources::KnowledgeStoreError>
    {
        self.0
            .list(workspace_id)
            .await
            .map_err(|e| crate::knowledge_sources::KnowledgeStoreError::Internal(e.to_string()))?
            .into_iter()
            .map(knowledge_row_to_document)
            .collect()
    }

    async fn create(
        &self,
        workspace_id: &str,
        input: tl_core::CreateKnowledgeSourceRequest,
    ) -> Result<tl_core::KnowledgeSourceDocument, crate::knowledge_sources::KnowledgeStoreError>
    {
        let file = match input.file {
            Some(file) => {
                let data = crate::knowledge_sources::decode_file_data(&file.data_base64)?;
                Some(NewKnowledgeFile {
                    file_name: file.file_name,
                    media_type: file.media_type,
                    data,
                })
            }
            None => None,
        };
        let row = self
            .0
            .create(
                workspace_id,
                NewKnowledgeSource {
                    title: input.title,
                    kind: knowledge_kind_text(input.kind).to_string(),
                    location: input.location,
                    notes: input.notes,
                    file,
                },
            )
            .await
            .map_err(|e| crate::knowledge_sources::KnowledgeStoreError::Internal(e.to_string()))?;
        knowledge_row_to_document(row)
    }

    async fn get_file(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> Result<tl_core::KnowledgeSourceFileResponse, crate::knowledge_sources::KnowledgeStoreError>
    {
        let row = self
            .0
            .get_file(workspace_id, source_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => {
                    crate::knowledge_sources::KnowledgeStoreError::NotFound
                }
                other => crate::knowledge_sources::KnowledgeStoreError::Internal(other.to_string()),
            })?;
        Ok(tl_core::KnowledgeSourceFileResponse {
            file_name: row.file_name,
            media_type: row.media_type,
            byte_size: row.byte_size,
            data_base64: STANDARD.encode(row.data),
        })
    }
}

#[cfg(feature = "postgres")]
fn knowledge_row_to_document(
    row: tl_storage::KnowledgeSourceRow,
) -> Result<tl_core::KnowledgeSourceDocument, crate::knowledge_sources::KnowledgeStoreError> {
    Ok(tl_core::KnowledgeSourceDocument {
        id: row.id,
        title: row.title,
        kind: parse_knowledge_kind(&row.kind)?,
        location: row.location,
        status: parse_knowledge_status(&row.status)?,
        metadata: row.metadata,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
        last_indexed_at: row.last_indexed_at.map(|ts| ts.to_rfc3339()),
    })
}

#[cfg(feature = "postgres")]
fn knowledge_kind_text(kind: tl_core::DashboardKnowledgeSourceKind) -> &'static str {
    match kind {
        tl_core::DashboardKnowledgeSourceKind::Url => "url",
        tl_core::DashboardKnowledgeSourceKind::File => "file",
        tl_core::DashboardKnowledgeSourceKind::Note => "note",
    }
}

#[cfg(feature = "postgres")]
fn parse_knowledge_kind(
    kind: &str,
) -> Result<tl_core::DashboardKnowledgeSourceKind, crate::knowledge_sources::KnowledgeStoreError> {
    match kind {
        "url" => Ok(tl_core::DashboardKnowledgeSourceKind::Url),
        "file" => Ok(tl_core::DashboardKnowledgeSourceKind::File),
        "note" => Ok(tl_core::DashboardKnowledgeSourceKind::Note),
        other => Err(crate::knowledge_sources::KnowledgeStoreError::Internal(
            format!("unknown knowledge source kind `{other}`"),
        )),
    }
}

#[cfg(feature = "postgres")]
fn parse_knowledge_status(
    status: &str,
) -> Result<tl_core::KnowledgeSourceStatus, crate::knowledge_sources::KnowledgeStoreError> {
    match status {
        "draft" => Ok(tl_core::KnowledgeSourceStatus::Draft),
        "indexing" => Ok(tl_core::KnowledgeSourceStatus::Indexing),
        "ready" => Ok(tl_core::KnowledgeSourceStatus::Ready),
        "failed" => Ok(tl_core::KnowledgeSourceStatus::Failed),
        other => Err(crate::knowledge_sources::KnowledgeStoreError::Internal(
            format!("unknown knowledge source status `{other}`"),
        )),
    }
}

#[cfg(feature = "postgres")]
fn policy_action(action: &tl_policy::Action) -> String {
    match action {
        tl_policy::Action::Allow => "allow",
        tl_policy::Action::Block => "block",
        tl_policy::Action::Rewrite => "rewrite",
        tl_policy::Action::Escalate => "escalate",
    }
    .to_string()
}

#[cfg(feature = "postgres")]
pub struct PostgresUserAdapter(pub Arc<UserRepo>);

#[cfg(feature = "postgres")]
impl PostgresUserAdapter {
    pub fn new(repo: Arc<UserRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl UserStore for PostgresUserAdapter {
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<crate::auth_user::UserRecord, UserStoreError> {
        let row = self
            .0
            .create(username, password_hash)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::Conflict => UserStoreError::Conflict,
                other => UserStoreError::Internal(other.to_string()),
            })?;
        Ok(crate::auth_user::UserRecord {
            id: row.id,
            username: row.username,
            password_hash: row.password_hash,
        })
    }

    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<crate::auth_user::UserRecord, UserStoreError> {
        let row = self
            .0
            .find_by_username(username)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => UserStoreError::NotFound,
                other => UserStoreError::Internal(other.to_string()),
            })?;
        Ok(crate::auth_user::UserRecord {
            id: row.id,
            username: row.username,
            password_hash: row.password_hash,
        })
    }

    async fn update_password(
        &self,
        id: uuid::Uuid,
        password_hash: &str,
    ) -> Result<(), UserStoreError> {
        self.0
            .update_password(id, password_hash)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => UserStoreError::NotFound,
                other => UserStoreError::Internal(other.to_string()),
            })
    }
}
