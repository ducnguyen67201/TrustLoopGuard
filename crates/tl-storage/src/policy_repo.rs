//! Persistent + cached `Policy` repository.
//!
//! Callers parse and validate YAML through `tl-policy`, then pass both
//! the typed policy and source YAML here. The database stores both forms:
//! YAML for authoring/audit and JSONB for fast reads and runtime loading.

use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use sqlx::postgres::PgPool;
use sqlx::types::Json;
use tl_policy::Policy;

use crate::StorageError;

const DEFAULT_CACHE_CAPACITY: u64 = 1_000;
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct PolicyRepo {
    pool: PgPool,
    cache: Cache<String, Arc<Policy>>,
}

impl PolicyRepo {
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

    /// Insert or update a policy. Upsert resurrects soft-deleted rows
    /// and makes them enabled, which matches an author intentionally
    /// publishing the policy again.
    pub async fn upsert(&self, policy: &Policy, source_yaml: &str) -> Result<(), StorageError> {
        let payload = Json(
            serde_json::to_value(policy)
                .map_err(|e| StorageError::Internal(format!("policy serialize: {e}")))?,
        );
        sqlx::query(
            r#"
            INSERT INTO "Policy" (id, policy_yaml, parsed_policy, enabled, created_at, updated_at)
            VALUES ($1, $2, $3, TRUE, NOW(), NOW())
            ON CONFLICT (id) DO UPDATE
                SET policy_yaml   = EXCLUDED.policy_yaml,
                    parsed_policy = EXCLUDED.parsed_policy,
                    enabled       = TRUE,
                    updated_at    = NOW(),
                    deleted_at    = NULL
            "#,
        )
        .bind(&policy.id)
        .bind(source_yaml)
        .bind(payload)
        .execute(&self.pool)
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

        let row: Option<(Json<Policy>,)> = sqlx::query_as(
            r#"SELECT parsed_policy FROM "Policy" WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(policy_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("policy get: {e}")))?;

        match row {
            Some((Json(policy),)) => {
                let arc = Arc::new(policy);
                self.cache.insert(policy_id.to_string(), arc.clone()).await;
                Ok(arc)
            }
            None => Err(StorageError::NotFound),
        }
    }

    /// All non-deleted policies. Bypasses the cache because this is an
    /// admin/editor path, not the hot path.
    pub async fn list(&self) -> Result<Vec<Arc<Policy>>, StorageError> {
        let rows: Vec<(Json<Policy>,)> = sqlx::query_as(
            r#"SELECT parsed_policy FROM "Policy" WHERE deleted_at IS NULL ORDER BY id"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("policy list: {e}")))?;
        Ok(rows.into_iter().map(|(Json(p),)| Arc::new(p)).collect())
    }

    /// Runtime policy set: active, enabled policies only.
    pub async fn list_enabled(&self) -> Result<Vec<Arc<Policy>>, StorageError> {
        let rows: Vec<(Json<Policy>,)> = sqlx::query_as(
            r#"
            SELECT parsed_policy
            FROM "Policy"
            WHERE deleted_at IS NULL AND enabled = TRUE
            ORDER BY id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("enabled policy list: {e}")))?;
        Ok(rows.into_iter().map(|(Json(p),)| Arc::new(p)).collect())
    }

    pub async fn set_enabled(&self, policy_id: &str, enabled: bool) -> Result<(), StorageError> {
        let result = sqlx::query(
            r#"
            UPDATE "Policy"
            SET enabled = $2, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(policy_id)
        .bind(enabled)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("policy set enabled: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Soft delete: sets `deleted_at` and clears the cache.
    pub async fn delete(&self, policy_id: &str) -> Result<(), StorageError> {
        let result = sqlx::query(
            r#"UPDATE "Policy" SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(policy_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("policy delete: {e}")))?;

        self.cache.invalidate(policy_id).await;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Approximate cache occupancy. For tests + diagnostics.
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl std::fmt::Debug for PolicyRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyRepo")
            .field("cache_size", &self.cache.entry_count())
            .finish_non_exhaustive()
    }
}
