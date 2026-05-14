//! Persistent + cached `Policy` repository.
//!
//! Callers parse and validate YAML through `tl-policy`, then pass both
//! the typed policy and source YAML here. The database stores both forms:
//! YAML for authoring/audit and JSONB for fast reads and runtime loading.

use std::sync::Arc;
use std::time::Duration;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use moka::future::Cache;
use tl_policy::Policy;

use crate::models::{NewPolicy, PolicyRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::policies;
use crate::StorageError;

const DEFAULT_CACHE_CAPACITY: u64 = 1_000;
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct PolicyRepo {
    pool: DbPool,
    cache: Cache<String, Arc<Policy>>,
}

#[derive(Debug, Clone)]
pub struct PolicyRow {
    pub policy: Policy,
    pub source_yaml: String,
    pub enabled: bool,
    pub owner_agent_id: Option<String>,
}

impl PolicyRepo {
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

    /// Insert or update a policy. Upsert resurrects soft-deleted rows
    /// and makes them enabled, which matches an author intentionally
    /// publishing the policy again.
    pub async fn upsert(&self, policy: &Policy, source_yaml: &str) -> Result<(), StorageError> {
        let parsed_policy = serde_json::to_value(policy)
            .map_err(|e| StorageError::Internal(format!("policy serialize: {e}")))?;
        let new_policy = NewPolicy {
            id: policy.id.clone(),
            policy_yaml: source_yaml.to_string(),
            parsed_policy,
            owner_agent_id: policy.owner_agent_id.clone(),
        };
        let mut conn = self.connection().await?;

        diesel::insert_into(policies::table)
            .values(&new_policy)
            .on_conflict(policies::id)
            .do_update()
            .set((
                policies::policy_yaml.eq(excluded(policies::policy_yaml)),
                policies::parsed_policy.eq(excluded(policies::parsed_policy)),
                policies::enabled.eq(true),
                policies::updated_at.eq(now),
                policies::deleted_at.eq(None::<chrono::DateTime<chrono::Utc>>),
                policies::owner_agent_id.eq(excluded(policies::owner_agent_id)),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy upsert: {e}")))?;

        self.cache
            .insert(policy.id.clone(), Arc::new(policy.clone()))
            .await;
        Ok(())
    }

    /// Resolve a policy by id. Disabled policies are still retrievable
    /// for admin/editor views; soft-deleted rows return `NotFound`.
    pub async fn get(&self, policy_id: &str) -> Result<Arc<Policy>, StorageError> {
        if let Some(cached) = self.cache.get(policy_id).await {
            return Ok(cached);
        }

        let mut conn = self.connection().await?;
        let row = policies::table
            .filter(policies::id.eq(policy_id))
            .filter(policies::deleted_at.is_null())
            .select(policies::parsed_policy)
            .first::<serde_json::Value>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("policy get: {e}")))?;

        match row {
            Some(value) => {
                let policy: Policy = serde_json::from_value(value)
                    .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))?;
                let arc = Arc::new(policy);
                self.cache.insert(policy_id.to_string(), arc.clone()).await;
                Ok(arc)
            }
            None => Err(StorageError::NotFound),
        }
    }

    /// Full authoring record for API/editor views.
    pub async fn get_record(&self, policy_id: &str) -> Result<PolicyRow, StorageError> {
        let mut conn = self.connection().await?;
        let row = policies::table
            .filter(policies::id.eq(policy_id))
            .filter(policies::deleted_at.is_null())
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
            ))
            .first::<PolicyRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("policy record get: {e}")))?;

        match row {
            Some(record) => Ok(PolicyRow {
                policy: serde_json::from_value(record.parsed_policy)
                    .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))?,
                source_yaml: record.policy_yaml,
                enabled: record.enabled,
                owner_agent_id: record.owner_agent_id,
            }),
            None => Err(StorageError::NotFound),
        }
    }

    /// All non-deleted policies. Bypasses the cache because this is an
    /// admin/editor path, not the hot path.
    pub async fn list(&self) -> Result<Vec<Arc<Policy>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::deleted_at.is_null())
            .select(policies::parsed_policy)
            .order(policies::id.asc())
            .load::<serde_json::Value>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy list: {e}")))?;
        rows.into_iter()
            .map(|value| {
                serde_json::from_value(value)
                    .map(Arc::new)
                    .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))
            })
            .collect()
    }

    /// All non-deleted authoring records. Bypasses the cache.
    pub async fn list_records(&self) -> Result<Vec<PolicyRow>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::deleted_at.is_null())
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
            ))
            .order(policies::id.asc())
            .load::<PolicyRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy record list: {e}")))?;
        rows.into_iter()
            .map(|record| {
                Ok(PolicyRow {
                    policy: serde_json::from_value(record.parsed_policy)
                        .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))?,
                    source_yaml: record.policy_yaml,
                    enabled: record.enabled,
                    owner_agent_id: record.owner_agent_id,
                })
            })
            .collect()
    }

    /// All non-deleted policies owned by a given agent. Bypasses the
    /// cache — admin-list path, not the hot policy-resolution path.
    pub async fn list_records_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<PolicyRow>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::deleted_at.is_null())
            .filter(policies::owner_agent_id.eq(agent_id))
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
            ))
            .order(policies::id.asc())
            .load::<PolicyRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy by-agent list: {e}")))?;
        rows.into_iter()
            .map(|record| {
                Ok(PolicyRow {
                    policy: serde_json::from_value(record.parsed_policy)
                        .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))?,
                    source_yaml: record.policy_yaml,
                    enabled: record.enabled,
                    owner_agent_id: record.owner_agent_id,
                })
            })
            .collect()
    }

    /// Soft-delete every active policy owned by `agent_id`. Returns the
    /// list of policy ids that were marked deleted so callers can also
    /// invalidate their caches. Used by the cascade-delete handler in
    /// the server when an agent is soft-deleted.
    pub async fn soft_delete_for_agent(&self, agent_id: &str) -> Result<Vec<String>, StorageError> {
        let mut conn = self.connection().await?;
        let now_ts = chrono::Utc::now();
        let deleted_ids: Vec<String> = diesel::update(
            policies::table
                .filter(policies::owner_agent_id.eq(agent_id))
                .filter(policies::deleted_at.is_null()),
        )
        .set(policies::deleted_at.eq(Some(now_ts)))
        .returning(policies::id)
        .get_results(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("policy cascade delete: {e}")))?;

        for id in &deleted_ids {
            self.cache.invalidate(id).await;
        }
        Ok(deleted_ids)
    }

    /// Runtime policy set: active, enabled policies only.
    pub async fn list_enabled(&self) -> Result<Vec<Arc<Policy>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::deleted_at.is_null())
            .filter(policies::enabled.eq(true))
            .select(policies::parsed_policy)
            .order(policies::id.asc())
            .load::<serde_json::Value>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("enabled policy list: {e}")))?;
        rows.into_iter()
            .map(|value| {
                serde_json::from_value(value)
                    .map(Arc::new)
                    .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))
            })
            .collect()
    }

    pub async fn set_enabled(&self, policy_id: &str, enabled: bool) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows_affected = diesel::update(
            policies::table
                .filter(policies::id.eq(policy_id))
                .filter(policies::deleted_at.is_null()),
        )
        .set((policies::enabled.eq(enabled), policies::updated_at.eq(now)))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("policy set enabled: {e}")))?;

        if rows_affected == 0 {
            return Err(StorageError::NotFound);
        }
        self.cache.invalidate(policy_id).await;
        Ok(())
    }

    /// Soft delete: sets `deleted_at` and clears the cache.
    pub async fn delete(&self, policy_id: &str) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows_affected = diesel::update(
            policies::table
                .filter(policies::id.eq(policy_id))
                .filter(policies::deleted_at.is_null()),
        )
        .set(policies::deleted_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("policy delete: {e}")))?;

        self.cache.invalidate(policy_id).await;

        if rows_affected == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Approximate cache occupancy. For tests + diagnostics.
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Invalidate a single cached policy. Used by the transactional
    /// cascade-delete path so AgentRepo can purge stale entries after a
    /// successful commit without owning PolicyRepo's cache directly.
    pub async fn invalidate_cache(&self, policy_id: &str) {
        self.cache.invalidate(policy_id).await;
    }
}

impl PolicyRepo {
    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for PolicyRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyRepo")
            .field("cache_size", &self.cache.entry_count())
            .finish_non_exhaustive()
    }
}
