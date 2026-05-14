//! Persistent + cached `AgentProfile` repository.
//!
//! Two layers in front of Postgres:
//! 1. `moka::future::Cache<String, Arc<AgentProfile>>` — keyed by
//!    `agent_id`, refreshed on `upsert` / invalidated on `delete`.
//! 2. The `agents` table — source of truth, stores `profile_yaml`
//!    alongside the materialised `parsed_profile JSONB`.
//!
//! Callers parse YAML themselves (`tl_policy::load_agent_str`) and
//! pass the parsed `AgentProfile` along with its source `yaml`.
//! Keeping the YAML parser out of `tl-storage` means this crate
//! doesn't depend on `tl-policy` and stays a pure persistence layer.

use std::sync::Arc;
use std::time::Duration;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use moka::future::Cache;
use tl_core::AgentProfile;

use crate::models::NewAgent;
use crate::postgres::{DbConnection, DbPool};
use crate::schema::agents;
use crate::StorageError;

const DEFAULT_CACHE_CAPACITY: u64 = 1_000;
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct AgentRepo {
    pool: DbPool,
    cache: Cache<String, Arc<AgentProfile>>,
}

impl AgentRepo {
    /// Build with default cache settings (1K capacity, 60s TTL).
    pub fn new(pool: DbPool) -> Self {
        Self::with_cache(pool, DEFAULT_CACHE_CAPACITY, DEFAULT_CACHE_TTL)
    }

    /// Build with explicit cache capacity and TTL. Capacity 0 disables
    /// the cache (every read hits Postgres).
    pub fn with_cache(pool: DbPool, capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .build();
        Self { pool, cache }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Insert or update a profile. Stores both the source YAML (for
    /// audit / re-parse) and the parsed JSONB (for fast queries).
    /// Refreshes the cache on success.
    pub async fn upsert(
        &self,
        profile: &AgentProfile,
        source_yaml: &str,
    ) -> Result<(), StorageError> {
        let parsed_profile = serde_json::to_value(profile)
            .map_err(|e| StorageError::Internal(format!("profile serialize: {e}")))?;
        let new_agent = NewAgent {
            id: profile.agent_id.clone(),
            profile_yaml: source_yaml.to_string(),
            parsed_profile,
        };
        let mut conn = self.connection().await?;

        diesel::insert_into(agents::table)
            .values(&new_agent)
            .on_conflict(agents::id)
            .do_update()
            .set((
                agents::profile_yaml.eq(excluded(agents::profile_yaml)),
                agents::parsed_profile.eq(excluded(agents::parsed_profile)),
                agents::updated_at.eq(now),
                agents::deleted_at.eq(None::<chrono::DateTime<chrono::Utc>>),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("agent upsert: {e}")))?;

        // Refresh the cache so the next read sees the new value.
        self.cache
            .insert(profile.agent_id.clone(), Arc::new(profile.clone()))
            .await;
        Ok(())
    }

    /// Resolve an `agent_id` to its profile. Cache hits return in
    /// microseconds; misses fall through to Postgres and back-fill
    /// the cache. `NotFound` for unknown or soft-deleted ids.
    pub async fn get(&self, agent_id: &str) -> Result<Arc<AgentProfile>, StorageError> {
        if let Some(cached) = self.cache.get(agent_id).await {
            return Ok(cached);
        }
        let mut conn = self.connection().await?;
        let row = agents::table
            .filter(agents::id.eq(agent_id))
            .filter(agents::deleted_at.is_null())
            .select(agents::parsed_profile)
            .first::<serde_json::Value>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("agent get: {e}")))?;

        match row {
            Some(value) => {
                let profile: AgentProfile = serde_json::from_value(value)
                    .map_err(|e| StorageError::Internal(format!("profile deserialize: {e}")))?;
                let arc = Arc::new(profile);
                self.cache.insert(agent_id.to_string(), arc.clone()).await;
                Ok(arc)
            }
            None => Err(StorageError::NotFound),
        }
    }

    /// Soft delete: sets `deleted_at` and clears the cache. The row
    /// stays for audit; future `get` returns `NotFound`.
    pub async fn delete(&self, agent_id: &str) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows_affected = diesel::update(
            agents::table
                .filter(agents::id.eq(agent_id))
                .filter(agents::deleted_at.is_null()),
        )
        .set(agents::deleted_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("agent delete: {e}")))?;

        self.cache.invalidate(agent_id).await;

        if rows_affected == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// All non-deleted profiles. Bypasses the cache (small admin path —
    /// not the hot path).
    pub async fn list(&self) -> Result<Vec<Arc<AgentProfile>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = agents::table
            .filter(agents::deleted_at.is_null())
            .select(agents::parsed_profile)
            .order(agents::id.asc())
            .load::<serde_json::Value>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("agent list: {e}")))?;
        rows.into_iter()
            .map(|value| {
                serde_json::from_value(value)
                    .map(Arc::new)
                    .map_err(|e| StorageError::Internal(format!("profile deserialize: {e}")))
            })
            .collect()
    }

    /// Approximate cache occupancy. For tests + diagnostics.
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Soft-delete an agent and every policy that lists it as
    /// `owner_agent_id`, atomically. Both writes run inside a single
    /// Postgres transaction — if either fails (including the
    /// `agent_id not found` branch), nothing is committed.
    ///
    /// Returns the policy ids that were soft-deleted so callers can
    /// invalidate any per-policy caches outside the transaction.
    ///
    /// Caches are invalidated only on commit — see the end of this fn.
    pub async fn cascade_soft_delete(
        &self,
        policy_repo: &crate::policy_repo::PolicyRepo,
        agent_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        use diesel_async::AsyncConnection;

        let mut conn = self.connection().await?;
        let now_ts = chrono::Utc::now();

        // Whole cascade lives in one transaction. diesel-async 0.9
        // takes an `AsyncFnOnce` callback that yields a Result, where
        // any Err rolls the transaction back. We signal "agent not
        // found" with the dedicated RollbackTransaction sentinel and
        // translate it to StorageError::NotFound below.
        let result: Result<Vec<String>, diesel::result::Error> = conn
            .transaction::<Vec<String>, diesel::result::Error, _>(async |tx_conn| {
                // 1. Soft-delete owned policies. RETURNING tells us
                // which rows actually got marked so we can invalidate
                // their caches downstream.
                let deleted_policy_ids: Vec<String> = diesel::update(
                    crate::schema::policies::table
                        .filter(crate::schema::policies::owner_agent_id.eq(agent_id))
                        .filter(crate::schema::policies::deleted_at.is_null()),
                )
                .set(crate::schema::policies::deleted_at.eq(Some(now_ts)))
                .returning(crate::schema::policies::id)
                .get_results(tx_conn)
                .await?;

                // 2. Soft-delete the agent. NOTE: an already-deleted
                // or non-existent agent shows up as rows_affected
                // == 0; rolling back via RollbackTransaction maps
                // cleanly to StorageError::NotFound after unwind.
                let rows_affected = diesel::update(
                    agents::table
                        .filter(agents::id.eq(agent_id))
                        .filter(agents::deleted_at.is_null()),
                )
                .set(agents::deleted_at.eq(Some(now_ts)))
                .execute(tx_conn)
                .await?;

                if rows_affected == 0 {
                    return Err(diesel::result::Error::RollbackTransaction);
                }
                Ok(deleted_policy_ids)
            })
            .await;

        let deleted_policy_ids = match result {
            Ok(ids) => ids,
            Err(diesel::result::Error::RollbackTransaction) => {
                return Err(StorageError::NotFound);
            }
            Err(e) => {
                return Err(StorageError::Internal(format!(
                    "agent cascade-delete transaction: {e}"
                )));
            }
        };

        // Only invalidate caches after the commit succeeds. Doing this
        // earlier risks a stale-cache window if the transaction rolled
        // back at the last moment.
        for id in &deleted_policy_ids {
            policy_repo.invalidate_cache(id).await;
        }
        self.cache.invalidate(agent_id).await;

        Ok(deleted_policy_ids)
    }
}

impl AgentRepo {
    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for AgentRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRepo")
            .field("cache_size", &self.cache.entry_count())
            .finish_non_exhaustive()
    }
}
