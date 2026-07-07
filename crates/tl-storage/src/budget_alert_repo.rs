//! Budget alert threshold configs + firing log repository.
//!
//! Configs are the user-authored thresholds ("warn at 80% of the
//! weekly cap"); firings are the once-per-window crossings. Dedup is
//! `try_record_firing`: INSERT ... ON CONFLICT DO NOTHING against the
//! UNIQUE `(config_id, principal_id, window_start)` key, returning
//! whether this call won the race — the winner delivers the webhook.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::models::{
    BudgetAlertConfigRecord, BudgetAlertFiringRecord, NewBudgetAlertConfig, NewBudgetAlertFiring,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{budget_alert_configs, budget_alert_firings};
use crate::StorageError;

/// Cap on firing listings, newest first.
const LIST_FIRINGS_LIMIT: i64 = 500;

#[derive(Debug, Clone, PartialEq)]
pub struct StoredBudgetAlertConfig {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub window: String,
    pub principal_id: Option<String>,
    pub threshold_type: String,
    pub threshold_value: i64,
    pub webhook_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewBudgetAlertConfigParams {
    pub name: String,
    pub window: String,
    pub principal_id: Option<String>,
    pub threshold_type: String,
    pub threshold_value: i64,
    pub webhook_url: Option<String>,
    pub enabled: bool,
}

/// Partial update: `None` fields are left unchanged. Nullable columns
/// (`principal_id`, `webhook_url`) cannot be cleared through this
/// shape — recreate the config to widen its scope.
#[derive(Debug, Clone, Default, PartialEq, AsChangeset)]
#[diesel(table_name = budget_alert_configs)]
pub struct UpdateBudgetAlertConfigParams {
    pub name: Option<String>,
    pub window: Option<String>,
    pub principal_id: Option<String>,
    pub threshold_type: Option<String>,
    pub threshold_value: Option<i64>,
    pub webhook_url: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewBudgetAlertFiringParams {
    pub config_id: String,
    pub principal_id: String,
    pub window_start: DateTime<Utc>,
    pub cap_minor: i64,
    pub spent_minor: i64,
    pub currency: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredBudgetAlertFiring {
    pub id: String,
    pub workspace_id: String,
    pub config_id: String,
    pub principal_id: String,
    pub window_start: DateTime<Utc>,
    pub cap_minor: i64,
    pub spent_minor: i64,
    pub currency: String,
    pub payload: serde_json::Value,
    pub fired_at: DateTime<Utc>,
}

pub struct BudgetAlertRepo {
    pool: DbPool,
}

impl BudgetAlertRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_config(
        &self,
        workspace_id: &str,
        params: NewBudgetAlertConfigParams,
    ) -> Result<StoredBudgetAlertConfig, StorageError> {
        let row = NewBudgetAlertConfig {
            id: Uuid::now_v7(),
            workspace_id: workspace_id.to_string(),
            name: params.name,
            window: params.window,
            principal_id: params.principal_id,
            threshold_type: params.threshold_type,
            threshold_value: params.threshold_value,
            webhook_url: params.webhook_url,
            enabled: params.enabled,
        };
        let mut conn = self.connection().await?;
        let record = diesel::insert_into(budget_alert_configs::table)
            .values(&row)
            .returning(BudgetAlertConfigRecord::as_returning())
            .get_result::<BudgetAlertConfigRecord>(&mut conn)
            .await
            .map_err(StorageError::from)?;
        Ok(stored_config(record))
    }

    pub async fn get_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<StoredBudgetAlertConfig, StorageError> {
        let config_id = parse_config_id(config_id)?;
        let mut conn = self.connection().await?;
        let record = budget_alert_configs::table
            .filter(budget_alert_configs::workspace_id.eq(workspace_id))
            .filter(budget_alert_configs::id.eq(config_id))
            .select(BudgetAlertConfigRecord::as_select())
            .first::<BudgetAlertConfigRecord>(&mut conn)
            .await
            .map_err(StorageError::from)?;
        Ok(stored_config(record))
    }

    pub async fn list_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<StoredBudgetAlertConfig>, StorageError> {
        self.list_configs_impl(workspace_id, false).await
    }

    /// The spend-time hook's single indexed lookup.
    pub async fn list_enabled_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<StoredBudgetAlertConfig>, StorageError> {
        self.list_configs_impl(workspace_id, true).await
    }

    async fn list_configs_impl(
        &self,
        workspace_id: &str,
        enabled_only: bool,
    ) -> Result<Vec<StoredBudgetAlertConfig>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = budget_alert_configs::table
            .filter(budget_alert_configs::workspace_id.eq(workspace_id))
            .into_boxed();
        if enabled_only {
            query = query.filter(budget_alert_configs::enabled.eq(true));
        }
        let rows = query
            .order(budget_alert_configs::created_at.asc())
            .select(BudgetAlertConfigRecord::as_select())
            .load::<BudgetAlertConfigRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("budget alert config list: {e}")))?;
        Ok(rows.into_iter().map(stored_config).collect())
    }

    pub async fn update_config(
        &self,
        workspace_id: &str,
        config_id: &str,
        params: UpdateBudgetAlertConfigParams,
    ) -> Result<StoredBudgetAlertConfig, StorageError> {
        let config_id = parse_config_id(config_id)?;
        let mut conn = self.connection().await?;
        let record = diesel::update(
            budget_alert_configs::table
                .filter(budget_alert_configs::workspace_id.eq(workspace_id))
                .filter(budget_alert_configs::id.eq(config_id)),
        )
        .set((&params, budget_alert_configs::updated_at.eq(Utc::now())))
        .returning(BudgetAlertConfigRecord::as_returning())
        .get_result::<BudgetAlertConfigRecord>(&mut conn)
        .await
        .map_err(StorageError::from)?;
        Ok(stored_config(record))
    }

    pub async fn delete_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<(), StorageError> {
        let config_id = parse_config_id(config_id)?;
        let mut conn = self.connection().await?;
        let deleted = diesel::delete(
            budget_alert_configs::table
                .filter(budget_alert_configs::workspace_id.eq(workspace_id))
                .filter(budget_alert_configs::id.eq(config_id)),
        )
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("budget alert config delete: {e}")))?;
        if deleted == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Insert-first dedup gate: `true` when this call recorded the
    /// firing (caller should deliver), `false` when another spend in
    /// the same window already did.
    pub async fn try_record_firing(
        &self,
        workspace_id: &str,
        params: NewBudgetAlertFiringParams,
    ) -> Result<bool, StorageError> {
        let config_id = parse_config_id(&params.config_id)?;
        let row = NewBudgetAlertFiring {
            id: Uuid::now_v7(),
            workspace_id: workspace_id.to_string(),
            config_id,
            principal_id: params.principal_id,
            window_start: params.window_start,
            cap_minor: params.cap_minor,
            spent_minor: params.spent_minor,
            currency: params.currency,
            payload: params.payload,
        };
        let mut conn = self.connection().await?;
        let inserted = diesel::insert_into(budget_alert_firings::table)
            .values(&row)
            .on_conflict((
                budget_alert_firings::config_id,
                budget_alert_firings::principal_id,
                budget_alert_firings::window_start,
            ))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("budget alert firing insert: {e}")))?;
        Ok(inserted > 0)
    }

    /// Firing history, newest first. `config_id = None` lists the
    /// whole workspace.
    pub async fn list_firings(
        &self,
        workspace_id: &str,
        config_id: Option<&str>,
    ) -> Result<Vec<StoredBudgetAlertFiring>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = budget_alert_firings::table
            .filter(budget_alert_firings::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(config_id) = config_id {
            query = query.filter(budget_alert_firings::config_id.eq(parse_config_id(config_id)?));
        }
        let rows = query
            .order((
                budget_alert_firings::fired_at.desc(),
                budget_alert_firings::id.desc(),
            ))
            .limit(LIST_FIRINGS_LIMIT)
            .select(BudgetAlertFiringRecord::as_select())
            .load::<BudgetAlertFiringRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("budget alert firing list: {e}")))?;
        Ok(rows.into_iter().map(stored_firing).collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for BudgetAlertRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BudgetAlertRepo").finish_non_exhaustive()
    }
}

fn stored_config(record: BudgetAlertConfigRecord) -> StoredBudgetAlertConfig {
    StoredBudgetAlertConfig {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        name: record.name,
        window: record.window,
        principal_id: record.principal_id,
        threshold_type: record.threshold_type,
        threshold_value: record.threshold_value,
        webhook_url: record.webhook_url,
        enabled: record.enabled,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn stored_firing(record: BudgetAlertFiringRecord) -> StoredBudgetAlertFiring {
    StoredBudgetAlertFiring {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        config_id: record.config_id.to_string(),
        principal_id: record.principal_id,
        window_start: record.window_start,
        cap_minor: record.cap_minor,
        spent_minor: record.spent_minor,
        currency: record.currency,
        payload: record.payload,
        fired_at: record.fired_at,
    }
}

/// Config ids are UUIDs; a non-UUID path segment can never match a
/// row, so it maps to NotFound rather than a 500.
fn parse_config_id(config_id: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(config_id.trim()).map_err(|_| StorageError::NotFound)
}
