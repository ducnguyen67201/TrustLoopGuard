//! Persistent + cached `Policy` repository.
//!
//! Callers parse and validate YAML through `tl-policy`, then pass both
//! the typed policy and source YAML here. The database stores both forms:
//! YAML for authoring/audit and JSONB for fast reads and runtime loading.

mod environment_deployments;
mod mutations;
mod records;
mod versions;

use std::sync::Arc;
use std::time::Duration;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use moka::future::Cache;
use tl_policy::Policy;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::policies;
use crate::StorageError;

const DEFAULT_CACHE_CAPACITY: u64 = 1_000;
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct PolicyRepo {
    pub(super) pool: DbPool,
    pub(super) cache: Cache<String, Arc<Policy>>,
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

    /// Resolve a policy by id. Disabled policies are still retrievable
    /// for admin/editor views; soft-deleted rows return `NotFound`.
    pub async fn get(&self, policy_id: &str) -> Result<Arc<Policy>, StorageError> {
        self.get_in(tl_core::DEFAULT_WORKSPACE_ID, policy_id).await
    }

    pub async fn get_in(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<Arc<Policy>, StorageError> {
        let key = cache_key(workspace_id, policy_id);
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        let mut conn = self.connection().await?;
        let row = policies::table
            .filter(policies::workspace_id.eq(workspace_id))
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
                self.cache.insert(key, arc.clone()).await;
                Ok(arc)
            }
            None => Err(StorageError::NotFound),
        }
    }

    /// Approximate cache occupancy. For tests + diagnostics.
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }
}

pub(super) fn cache_key(workspace_id: &str, policy_id: &str) -> String {
    format!("{workspace_id}\0{policy_id}")
}

impl PolicyRepo {
    pub(super) async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
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
