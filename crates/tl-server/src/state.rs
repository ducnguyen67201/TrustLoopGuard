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
use crate::escalation::{spawn_escalation_worker, EscalationConfig, EscalationPayload};
#[cfg(feature = "postgres")]
use crate::policies::PolicyStoreError;
use crate::policies::{MemoryPolicyStore, PolicyStore};

#[cfg(feature = "postgres")]
use {
    tl_storage::{
        connect_postgres, migrate_postgres, spawn_writer, AgentRepo, EscalationRepo, PolicyRepo,
        TraceWrite, UserRepo, WriterConfig,
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
    /// Backing store for username/password accounts. Memory-only when
    /// the server runs without Postgres.
    pub user_store: Arc<dyn UserStore>,
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
        user_store: Arc::new(MemoryUserStore::new()),
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
    let (agent_store, profile_resolver, policy_store, user_store, trace_tx, escalation_repo) =
        build_postgres_layer(opts.database_url, &policies).await?;

    #[cfg(not(feature = "postgres"))]
    let (agent_store, profile_resolver, policy_store, user_store) = build_memory_layer(&policies);

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
        user_store,
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
async fn build_postgres_layer(
    database_url: Option<String>,
    fallback_policies: &[Policy],
) -> Result<(
    Arc<dyn AgentStore>,
    Arc<dyn ProfileResolver>,
    Arc<dyn PolicyStore>,
    Arc<dyn UserStore>,
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
            Arc::new(MemoryUserStore::new()) as Arc<dyn UserStore>,
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
    let user_repo = Arc::new(UserRepo::new(pool.clone()));
    let user_adapter = PostgresUserAdapter::new(user_repo);

    let (tx, _handle) = spawn_writer(pool.clone(), WriterConfig::default());
    tracing::info!("trace writer spawned");

    let escalation_repo = Arc::new(EscalationRepo::new(pool));

    Ok((
        adapter.clone() as Arc<dyn AgentStore>,
        adapter as Arc<dyn ProfileResolver>,
        policy_adapter as Arc<dyn PolicyStore>,
        user_adapter as Arc<dyn UserStore>,
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
    Arc<dyn UserStore>,
) {
    let mem = Arc::new(MemoryAgentStore::new());
    (
        mem.clone() as Arc<dyn AgentStore>,
        mem as Arc<dyn ProfileResolver>,
        Arc::new(MemoryPolicyStore::with_policies(policies)) as Arc<dyn PolicyStore>,
        Arc::new(MemoryUserStore::new()) as Arc<dyn UserStore>,
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
    async fn resolve(&self, agent_id: &str) -> Option<Arc<AgentProfile>> {
        AgentStore::get(self, agent_id).await.ok()
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
    async fn resolve(&self, agent_id: &str) -> Option<Arc<AgentProfile>> {
        self.0.get(agent_id).await.ok()
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl AgentStore for PostgresAgentAdapter {
    async fn upsert(
        &self,
        profile: &AgentProfile,
        source_yaml: &str,
    ) -> Result<(), AgentStoreError> {
        self.0
            .upsert(profile, source_yaml)
            .await
            .map_err(|e| AgentStoreError::Internal(e.to_string()))
    }

    async fn get(&self, agent_id: &str) -> Result<Arc<AgentProfile>, AgentStoreError> {
        self.0.get(agent_id).await.map_err(|e| match e {
            tl_storage::StorageError::NotFound => AgentStoreError::NotFound,
            other => AgentStoreError::Internal(other.to_string()),
        })
    }

    async fn delete(&self, agent_id: &str) -> Result<(), AgentStoreError> {
        self.0.delete(agent_id).await.map_err(|e| match e {
            tl_storage::StorageError::NotFound => AgentStoreError::NotFound,
            other => AgentStoreError::Internal(other.to_string()),
        })
    }

    async fn list(&self) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError> {
        self.0
            .list()
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
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .upsert(policy, source_yaml)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))?;
        self.get(&policy.id).await
    }

    async fn get(&self, policy_id: &str) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0.get_record(policy_id).await.map_or_else(
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

    async fn list(&self) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records()
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::PolicySummary {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        enabled: row.enabled,
                    })
                    .collect()
            })
    }

    async fn list_enabled(&self) -> Result<Vec<Arc<Policy>>, PolicyStoreError> {
        self.0
            .list_enabled()
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn set_enabled(
        &self,
        policy_id: &str,
        enabled: bool,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .set_enabled(policy_id, enabled)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })?;
        self.get(policy_id).await
    }

    async fn delete(&self, policy_id: &str) -> Result<(), PolicyStoreError> {
        self.0.delete(policy_id).await.map_err(|e| match e {
            tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
            other => PolicyStoreError::Internal(other.to_string()),
        })
    }

    async fn list_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records_for_agent(agent_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::PolicySummary {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        enabled: row.enabled,
                    })
                    .collect()
            })
    }

    async fn delete_for_agent(&self, agent_id: &str) -> Result<Vec<String>, PolicyStoreError> {
        self.0
            .soft_delete_for_agent(agent_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }
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
}
