//! Persistent + cached `SourceLabelPolicy` repository.
//!
//! Two layers in front of Postgres:
//! 1. `moka::future::Cache<String, Arc<Vec<StoredSourceLabelPolicy>>>` —
//!    keyed by `workspace_id`, invalidated on `upsert` / `delete`. This
//!    deliberately diverges from `ToolMetadataRepo`'s per-row cache: the
//!    runtime read is list-shaped (all origins for a workspace, at most
//!    one row per `Origin` variant), so caching the whole list avoids
//!    per-origin point lookups, and an empty vec doubles as the negative
//!    entry for workspaces with no policies.
//! 2. The `source_label_policy` table — source of truth, stores the full
//!    serialized policy in `spec JSONB` alongside the promoted `origin`
//!    key column.
//!
//! The repo returns rows regardless of `enabled`; runtime resolution
//! filters disabled policies at the provider seam so the control plane
//! can still read and re-enable them.

use std::sync::Arc;
use std::time::Duration;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use moka::future::Cache;
use tl_core::{Origin, SourceLabelPolicy};

use crate::models::NewSourceLabelPolicy;
use crate::postgres::{DbConnection, DbPool};
use crate::schema::source_label_policy;
use crate::StorageError;

const DEFAULT_CACHE_CAPACITY: u64 = 1_000;
/// Upper bound on cache staleness. A policy changed on another instance
/// can stay invisible to resolution for up to this long; operators
/// gating live traffic with `enabled`/`deleted_at` must account for the
/// window.
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

/// A stored policy row: the wire `SourceLabelPolicy` plus its registry
/// `enabled` flag. CRUD reads return disabled rows; runtime resolution
/// filters on `enabled` (see the server provider adapter).
#[derive(Debug, Clone, PartialEq)]
pub struct StoredSourceLabelPolicy {
    pub policy: SourceLabelPolicy,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct SourceLabelPolicyRepo {
    pool: DbPool,
    cache: Cache<String, Arc<Vec<StoredSourceLabelPolicy>>>,
}

impl SourceLabelPolicyRepo {
    /// Build with default cache settings (1K capacity, 60s TTL).
    pub fn new(pool: DbPool) -> Self {
        Self::with_cache(pool, DEFAULT_CACHE_CAPACITY, DEFAULT_CACHE_TTL)
    }

    /// Build with explicit cache capacity and TTL. Capacity 0 disables
    /// the cache (every read hits Postgres). Capacity counts entries,
    /// not bytes: each value is at most one small enum-only policy per
    /// `Origin` variant (≤9 rows). Add a moka `weigher` if the policy
    /// type ever grows variable-length fields.
    pub fn with_cache(pool: DbPool, capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .build();
        Self { pool, cache }
    }

    /// Insert or update a policy row. Invalidates the workspace cache
    /// entry and revives soft-deleted rows on success. The promoted
    /// `origin` key column is derived from the same struct as `spec`,
    /// so the two cannot drift through this repo; rows written outside
    /// it must keep `spec.origin` equal to the column.
    pub async fn upsert(
        &self,
        workspace_id: &str,
        policy: &SourceLabelPolicy,
        enabled: bool,
    ) -> Result<(), StorageError> {
        let spec = serde_json::to_value(policy)
            .map_err(|e| StorageError::Internal(format!("source label policy serialize: {e}")))?;
        let new_row = NewSourceLabelPolicy {
            workspace_id: workspace_id.to_string(),
            origin: origin_str(policy.origin)?,
            spec,
            enabled,
        };
        let mut conn = self.connection().await?;

        diesel::insert_into(source_label_policy::table)
            .values(&new_row)
            .on_conflict((
                source_label_policy::workspace_id,
                source_label_policy::origin,
            ))
            .do_update()
            .set((
                source_label_policy::spec.eq(excluded(source_label_policy::spec)),
                source_label_policy::enabled.eq(excluded(source_label_policy::enabled)),
                source_label_policy::updated_at.eq(now),
                source_label_policy::deleted_at.eq(None::<chrono::DateTime<chrono::Utc>>),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("source label policy upsert: {e}")))?;

        self.cache.invalidate(workspace_id).await;
        Ok(())
    }

    /// Resolve one origin's policy. Admin path: reads through Postgres
    /// (no per-row cache). `NotFound` for unknown or soft-deleted rows.
    pub async fn get(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<StoredSourceLabelPolicy, StorageError> {
        let mut conn = self.connection().await?;
        let row = source_label_policy::table
            .filter(source_label_policy::workspace_id.eq(workspace_id))
            .filter(source_label_policy::origin.eq(origin_str(origin)?))
            .filter(source_label_policy::deleted_at.is_null())
            .select((source_label_policy::spec, source_label_policy::enabled))
            .first::<(serde_json::Value, bool)>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("source label policy get: {e}")))?;

        match row {
            Some((spec, enabled)) => Ok(StoredSourceLabelPolicy {
                policy: deserialize_spec(spec)?,
                enabled,
            }),
            None => Err(StorageError::NotFound),
        }
    }

    /// Soft delete: sets `deleted_at` and invalidates the workspace
    /// cache. The row stays for audit; future `get` returns `NotFound`.
    pub async fn delete(&self, workspace_id: &str, origin: Origin) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows_affected = diesel::update(
            source_label_policy::table
                .filter(source_label_policy::workspace_id.eq(workspace_id))
                .filter(source_label_policy::origin.eq(origin_str(origin)?))
                .filter(source_label_policy::deleted_at.is_null()),
        )
        .set(source_label_policy::deleted_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("source label policy delete: {e}")))?;

        if rows_affected == 0 {
            return Err(StorageError::NotFound);
        }

        self.cache.invalidate(workspace_id).await;
        Ok(())
    }

    /// All non-deleted rows for the workspace — the per-event hot path.
    /// Cache hits return in microseconds; misses load from Postgres and
    /// back-fill the cache, including the empty list, so workspaces
    /// without policies stay off Postgres. `try_get_with` serializes
    /// concurrent misses per key, so a stampede resolves with one
    /// Postgres read; a write racing the load can still re-cache the
    /// pre-write snapshot for up to the TTL — the 60s staleness bound
    /// is the backstop, same as the tool-metadata cache.
    pub async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Arc<Vec<StoredSourceLabelPolicy>>, StorageError> {
        self.cache
            .try_get_with(workspace_id.to_string(), self.load_active(workspace_id))
            .await
            .map_err(|e: Arc<StorageError>| e.as_ref().clone())
    }

    async fn load_active(
        &self,
        workspace_id: &str,
    ) -> Result<Arc<Vec<StoredSourceLabelPolicy>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = source_label_policy::table
            .filter(source_label_policy::workspace_id.eq(workspace_id))
            .filter(source_label_policy::deleted_at.is_null())
            .select((source_label_policy::spec, source_label_policy::enabled))
            .order(source_label_policy::origin.asc())
            .load::<(serde_json::Value, bool)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("source label policy list: {e}")))?;

        rows.into_iter()
            .map(|(spec, enabled)| {
                Ok(StoredSourceLabelPolicy {
                    policy: deserialize_spec(spec)?,
                    enabled,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()
            .map(Arc::new)
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

impl std::fmt::Debug for SourceLabelPolicyRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceLabelPolicyRepo")
            .field("cache_size", &self.cache.entry_count())
            .finish_non_exhaustive()
    }
}

/// The promoted key column stores the serde snake_case name of the
/// origin (e.g. `web`).
fn origin_str(origin: Origin) -> Result<String, StorageError> {
    match serde_json::to_value(origin) {
        Ok(serde_json::Value::String(s)) => Ok(s),
        Ok(other) => Err(StorageError::Internal(format!(
            "origin serialized to non-string: {other}"
        ))),
        Err(e) => Err(StorageError::Internal(format!("origin serialize: {e}"))),
    }
}

fn deserialize_spec(spec: serde_json::Value) -> Result<SourceLabelPolicy, StorageError> {
    serde_json::from_value(spec)
        .map_err(|e| StorageError::Internal(format!("source label policy deserialize: {e}")))
}
