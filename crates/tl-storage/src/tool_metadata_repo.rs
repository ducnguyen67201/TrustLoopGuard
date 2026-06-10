//! Persistent + cached `ToolMetadata` repository.
//!
//! Two layers in front of Postgres:
//! 1. `moka::future::Cache<String, Option<Arc<StoredToolMetadata>>>` —
//!    keyed by `(workspace_id, tool)`, refreshed on `upsert` / `delete`.
//!    Misses are cached as `None`: unregistered tools are the common case
//!    on the per-event hot path and must not become a Postgres round trip
//!    each time. This deliberately diverges from `PolicyRepo`, which
//!    caches hits only.
//! 2. The `tool_metadata` table — source of truth, stores the full
//!    serialized metadata in `spec JSONB` alongside promoted
//!    `side_effect`/`reversible` columns.
//!
//! The repo returns rows regardless of `enabled`; runtime resolution
//! filters disabled tools at the provider seam so the control plane can
//! still read and re-enable them.

use std::sync::Arc;
use std::time::Duration;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use moka::future::Cache;
use tl_core::ToolMetadata;

use crate::models::NewToolMetadata;
use crate::postgres::{DbConnection, DbPool};
use crate::schema::tool_metadata;
use crate::StorageError;

const DEFAULT_CACHE_CAPACITY: u64 = 1_000;
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

/// A stored registry row: the wire `ToolMetadata` plus its registry
/// `enabled` flag. CRUD reads return disabled rows; runtime resolution
/// filters on `enabled` (see the server provider adapter).
#[derive(Debug, Clone, PartialEq)]
pub struct StoredToolMetadata {
    pub metadata: ToolMetadata,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct ToolMetadataRepo {
    pool: DbPool,
    cache: Cache<String, Option<Arc<StoredToolMetadata>>>,
}

impl ToolMetadataRepo {
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

    /// Insert or update a registry row. The promoted `side_effect` column
    /// is derived from the same struct as `spec`, so the two cannot drift.
    /// Refreshes the cache and revives soft-deleted rows on success.
    pub async fn upsert(
        &self,
        workspace_id: &str,
        metadata: &ToolMetadata,
        enabled: bool,
    ) -> Result<(), StorageError> {
        let spec = serde_json::to_value(metadata)
            .map_err(|e| StorageError::Internal(format!("tool metadata serialize: {e}")))?;
        let new_row = NewToolMetadata {
            workspace_id: workspace_id.to_string(),
            tool: metadata.tool.clone(),
            side_effect: side_effect_str(metadata)?,
            reversible: metadata.reversible,
            spec,
            enabled,
        };
        let mut conn = self.connection().await?;

        diesel::insert_into(tool_metadata::table)
            .values(&new_row)
            .on_conflict((tool_metadata::workspace_id, tool_metadata::tool))
            .do_update()
            .set((
                tool_metadata::side_effect.eq(excluded(tool_metadata::side_effect)),
                tool_metadata::reversible.eq(excluded(tool_metadata::reversible)),
                tool_metadata::spec.eq(excluded(tool_metadata::spec)),
                tool_metadata::enabled.eq(excluded(tool_metadata::enabled)),
                tool_metadata::updated_at.eq(now),
                tool_metadata::deleted_at.eq(None::<chrono::DateTime<chrono::Utc>>),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("tool metadata upsert: {e}")))?;

        self.cache
            .insert(
                cache_key(workspace_id, &metadata.tool),
                Some(Arc::new(StoredToolMetadata {
                    metadata: metadata.clone(),
                    enabled,
                })),
            )
            .await;
        Ok(())
    }

    /// Resolve a tool name to its stored metadata. Cache hits return in
    /// microseconds; misses fall through to Postgres and back-fill the
    /// cache — including negative entries, so repeated lookups of
    /// unregistered tools stay off Postgres. `NotFound` for unknown or
    /// soft-deleted tools.
    pub async fn get(
        &self,
        workspace_id: &str,
        tool: &str,
    ) -> Result<Arc<StoredToolMetadata>, StorageError> {
        let key = cache_key(workspace_id, tool);
        if let Some(cached) = self.cache.get(&key).await {
            return cached.ok_or(StorageError::NotFound);
        }
        let mut conn = self.connection().await?;
        let row = tool_metadata::table
            .filter(tool_metadata::workspace_id.eq(workspace_id))
            .filter(tool_metadata::tool.eq(tool))
            .filter(tool_metadata::deleted_at.is_null())
            .select((tool_metadata::spec, tool_metadata::enabled))
            .first::<(serde_json::Value, bool)>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("tool metadata get: {e}")))?;

        match row {
            Some((spec, enabled)) => {
                let stored = Arc::new(StoredToolMetadata {
                    metadata: deserialize_spec(spec)?,
                    enabled,
                });
                self.cache.insert(key, Some(stored.clone())).await;
                Ok(stored)
            }
            None => {
                self.cache.insert(key, None).await;
                Err(StorageError::NotFound)
            }
        }
    }

    /// Soft delete: sets `deleted_at` and caches the absence. The row
    /// stays for audit; future `get` returns `NotFound`.
    pub async fn delete(&self, workspace_id: &str, tool: &str) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows_affected = diesel::update(
            tool_metadata::table
                .filter(tool_metadata::workspace_id.eq(workspace_id))
                .filter(tool_metadata::tool.eq(tool))
                .filter(tool_metadata::deleted_at.is_null()),
        )
        .set(tool_metadata::deleted_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("tool metadata delete: {e}")))?;

        self.cache.insert(cache_key(workspace_id, tool), None).await;

        if rows_affected == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// All non-deleted rows. Bypasses the cache (small admin path —
    /// not the hot path).
    pub async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<Arc<StoredToolMetadata>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = tool_metadata::table
            .filter(tool_metadata::workspace_id.eq(workspace_id))
            .filter(tool_metadata::deleted_at.is_null())
            .select((tool_metadata::spec, tool_metadata::enabled))
            .order(tool_metadata::tool.asc())
            .load::<(serde_json::Value, bool)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("tool metadata list: {e}")))?;
        rows.into_iter()
            .map(|(spec, enabled)| {
                Ok(Arc::new(StoredToolMetadata {
                    metadata: deserialize_spec(spec)?,
                    enabled,
                }))
            })
            .collect()
    }

    /// Approximate cache occupancy. For tests + diagnostics.
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for ToolMetadataRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolMetadataRepo")
            .field("cache_size", &self.cache.entry_count())
            .finish_non_exhaustive()
    }
}

fn cache_key(workspace_id: &str, tool: &str) -> String {
    format!("{workspace_id}\0{tool}")
}

/// The promoted column stores the serde snake_case name of the side
/// effect (e.g. `external_communication`).
fn side_effect_str(metadata: &ToolMetadata) -> Result<String, StorageError> {
    match serde_json::to_value(metadata.side_effect) {
        Ok(serde_json::Value::String(s)) => Ok(s),
        Ok(other) => Err(StorageError::Internal(format!(
            "side effect serialized to non-string: {other}"
        ))),
        Err(e) => Err(StorageError::Internal(format!(
            "side effect serialize: {e}"
        ))),
    }
}

fn deserialize_spec(spec: serde_json::Value) -> Result<ToolMetadata, StorageError> {
    serde_json::from_value(spec)
        .map_err(|e| StorageError::Internal(format!("tool metadata deserialize: {e}")))
}
