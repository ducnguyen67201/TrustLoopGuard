//! Persistent + cached `AgentProfile` repository.
//!
//! Two layers in front of Postgres:
//! 1. `moka::future::Cache<String, Arc<AgentProfile>>` — keyed by
//!    `agent_id`, refreshed on `upsert` / invalidated on `delete`.
//! 2. The `"Agent"` table — source of truth, stores `profile_yaml`
//!    alongside the materialised `parsed_profile JSONB`.
//!
//! Callers parse YAML themselves (`tl_policy::load_agent_str`) and
//! pass the parsed `AgentProfile` along with its source `yaml`.
//! Keeping the YAML parser out of `tl-storage` means this crate
//! doesn't depend on `tl-policy` and stays a pure persistence layer.

use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use sqlx::postgres::PgPool;
use sqlx::types::Json;
use tl_core::AgentProfile;

use crate::StorageError;

const DEFAULT_CACHE_CAPACITY: u64 = 1_000;
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct AgentRepo {
    pool: PgPool,
    cache: Cache<String, Arc<AgentProfile>>,
}

impl AgentRepo {
    /// Build with default cache settings (1K capacity, 60s TTL).
    pub fn new(pool: PgPool) -> Self {
        Self::with_cache(pool, DEFAULT_CACHE_CAPACITY, DEFAULT_CACHE_TTL)
    }

    /// Build with explicit cache capacity and TTL. Capacity 0 disables
    /// the cache (every read hits Postgres).
    pub fn with_cache(pool: PgPool, capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .build();
        Self { pool, cache }
    }

    pub fn pool(&self) -> &PgPool {
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
        let payload = Json(serde_json::to_value(profile).map_err(|e| {
            StorageError::Internal(format!("profile serialize: {e}"))
        })?);
        sqlx::query(
            r#"
            INSERT INTO "Agent" (id, profile_yaml, parsed_profile, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (id) DO UPDATE
                SET profile_yaml   = EXCLUDED.profile_yaml,
                    parsed_profile = EXCLUDED.parsed_profile,
                    updated_at     = NOW(),
                    deleted_at     = NULL
            "#,
        )
        .bind(&profile.agent_id)
        .bind(source_yaml)
        .bind(payload)
        .execute(&self.pool)
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
        let row: Option<(Json<AgentProfile>,)> = sqlx::query_as(
            r#"SELECT parsed_profile FROM "Agent" WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("agent get: {e}")))?;

        match row {
            Some((Json(profile),)) => {
                let arc = Arc::new(profile);
                self.cache
                    .insert(agent_id.to_string(), arc.clone())
                    .await;
                Ok(arc)
            }
            None => Err(StorageError::NotFound),
        }
    }

    /// Soft delete: sets `deleted_at` and clears the cache. The row
    /// stays for audit; future `get` returns `NotFound`.
    pub async fn delete(&self, agent_id: &str) -> Result<(), StorageError> {
        let result = sqlx::query(
            r#"UPDATE "Agent" SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(agent_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("agent delete: {e}")))?;

        self.cache.invalidate(agent_id).await;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// All non-deleted profiles. Bypasses the cache (small admin path —
    /// not the hot path).
    pub async fn list(&self) -> Result<Vec<Arc<AgentProfile>>, StorageError> {
        let rows: Vec<(Json<AgentProfile>,)> = sqlx::query_as(
            r#"SELECT parsed_profile FROM "Agent" WHERE deleted_at IS NULL ORDER BY id"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("agent list: {e}")))?;
        Ok(rows.into_iter().map(|(Json(p),)| Arc::new(p)).collect())
    }

    /// Approximate cache occupancy. For tests + diagnostics.
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl std::fmt::Debug for AgentRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRepo")
            .field("cache_size", &self.cache.entry_count())
            .finish_non_exhaustive()
    }
}
