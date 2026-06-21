use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::global_feature_flags;
use crate::StorageError;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = global_feature_flags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GlobalFeatureFlagRow {
    pub key: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

#[derive(Clone)]
pub struct GlobalFeatureFlagRepo {
    pool: DbPool,
}

impl GlobalFeatureFlagRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> Result<Option<GlobalFeatureFlagRow>, StorageError> {
        let mut conn = self.connection().await?;
        global_feature_flags::table
            .filter(global_feature_flags::key.eq(key))
            .select(GlobalFeatureFlagRow::as_select())
            .first::<GlobalFeatureFlagRow>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("global feature flag get: {e}")))
    }

    pub async fn is_enabled(&self, key: &str, default: bool) -> Result<bool, StorageError> {
        Ok(self
            .get(key)
            .await?
            .map(|flag| flag.enabled)
            .unwrap_or(default))
    }

    pub async fn set_enabled(
        &self,
        key: &str,
        enabled: bool,
        updated_by: Option<&str>,
    ) -> Result<GlobalFeatureFlagRow, StorageError> {
        let now = Utc::now();
        let mut conn = self.connection().await?;
        diesel::insert_into(global_feature_flags::table)
            .values((
                global_feature_flags::key.eq(key),
                global_feature_flags::enabled.eq(enabled),
                global_feature_flags::config.eq(serde_json::json!({})),
                global_feature_flags::updated_at.eq(now),
                global_feature_flags::updated_by.eq(updated_by),
            ))
            .on_conflict(global_feature_flags::key)
            .do_update()
            .set((
                global_feature_flags::enabled.eq(enabled),
                global_feature_flags::updated_at.eq(now),
                global_feature_flags::updated_by.eq(updated_by),
            ))
            .returning(GlobalFeatureFlagRow::as_returning())
            .get_result::<GlobalFeatureFlagRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("global feature flag set: {e}")))
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for GlobalFeatureFlagRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalFeatureFlagRepo")
            .finish_non_exhaustive()
    }
}
