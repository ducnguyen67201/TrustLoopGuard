//! Dashboard runtime-admin repository.

mod api_keys;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde_json::Value;
use tl_core::{
    DataHandlingMode, EnforcementMode, EnvironmentCheckerModes, UpdateWorkspaceSettingsRequest,
    WorkspaceSettings,
};

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{environment_checker_modes, workspace_environments, workspace_settings};
use crate::StorageError;

pub use api_keys::ApiKeyAuthRecord;

#[derive(Clone)]
pub struct DashboardAdminRepo {
    pool: DbPool,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_settings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct SettingsRecord {
    default_action: String,
    escalation_webhook_url: Option<String>,
    telemetry_enabled: bool,
    retention_days: String,
    config: Value,
    updated_at: DateTime<Utc>,
    data_handling_mode: String,
    flow_checker_mode: String,
    memory_checker_mode: String,
    param_checker_mode: String,
    approval_checker_mode: String,
}

impl DashboardAdminRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn get_settings(
        &self,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceSettings>, StorageError> {
        let mut conn = self.connection().await?;
        let row = workspace_settings::table
            .filter(workspace_settings::workspace_id.eq(workspace_id))
            .select(SettingsRecord::as_select())
            .first::<SettingsRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("get workspace settings: {e}")))?;

        row.map(settings_from_record).transpose()
    }

    /// Apply a partial settings update atomically: the current row is
    /// read under `FOR UPDATE` and the merged row written in the same
    /// transaction, so concurrent PATCHes serialize instead of losing
    /// updates. `defaults` seeds the merge when no row exists yet.
    pub async fn update_settings(
        &self,
        workspace_id: &str,
        update: &UpdateWorkspaceSettingsRequest,
        defaults: WorkspaceSettings,
    ) -> Result<WorkspaceSettings, StorageError> {
        let now = Utc::now();
        let mut conn = self.connection().await?;
        let merged = conn
            .transaction::<_, StorageError, _>(async |conn| {
                let row = workspace_settings::table
                    .filter(workspace_settings::workspace_id.eq(workspace_id))
                    .select(SettingsRecord::as_select())
                    .for_update()
                    .first::<SettingsRecord>(conn)
                    .await
                    .optional()
                    .map_err(|e| StorageError::Internal(format!("get workspace settings: {e}")))?;
                let current = match row {
                    Some(row) => settings_from_record(row)?,
                    None => defaults,
                };
                let merged = update.apply_to(&current);

                let record = SettingsWriteRecord {
                    workspace_id: workspace_id.to_string(),
                    default_action: merged.default_action.clone(),
                    escalation_webhook_url: merged.escalation_webhook_url.clone(),
                    telemetry_enabled: merged.telemetry_enabled,
                    retention_days: merged.retention_days.clone(),
                    config: merged.config.clone(),
                    updated_at: now,
                    data_handling_mode: mode_to_db(
                        merged.data_handling_mode,
                        "data_handling_mode",
                    )?,
                    flow_checker_mode: mode_to_db(merged.flow_checker_mode, "flow_checker_mode")?,
                    memory_checker_mode: mode_to_db(
                        merged.memory_checker_mode,
                        "memory_checker_mode",
                    )?,
                    param_checker_mode: mode_to_db(
                        merged.param_checker_mode,
                        "param_checker_mode",
                    )?,
                    approval_checker_mode: mode_to_db(
                        merged.approval_checker_mode,
                        "approval_checker_mode",
                    )?,
                };
                diesel::insert_into(workspace_settings::table)
                    .values(&record)
                    .on_conflict(workspace_settings::workspace_id)
                    .do_update()
                    .set(&record)
                    .execute(conn)
                    .await
                    .map_err(|e| StorageError::Internal(format!("put workspace settings: {e}")))?;
                Ok(merged)
            })
            .await?;

        let mut persisted = merged;
        persisted.updated_at = Some(now.to_rfc3339());
        Ok(persisted)
    }

    pub async fn get_environment_checker_modes(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Option<EnvironmentCheckerModes>, StorageError> {
        let mut conn = self.connection().await?;
        let row = environment_checker_modes::table
            .filter(environment_checker_modes::workspace_id.eq(workspace_id))
            .filter(environment_checker_modes::environment_id.eq(environment_id))
            .select(EnvironmentCheckerModesRecord::as_select())
            .first::<EnvironmentCheckerModesRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("get environment checker modes: {e}")))?;

        row.map(environment_checker_modes_from_record).transpose()
    }

    /// UPSERT per-environment checker-mode overrides. Returns
    /// [`StorageError::NotFound`] when the environment does not exist
    /// (or is soft-deleted) in the workspace.
    pub async fn put_environment_checker_modes(
        &self,
        workspace_id: &str,
        environment_id: &str,
        modes: &EnvironmentCheckerModes,
    ) -> Result<EnvironmentCheckerModes, StorageError> {
        let now = Utc::now();
        let record = EnvironmentCheckerModesWriteRecord {
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            flow_checker_mode: optional_mode_to_db(modes.flow_checker_mode, "flow_checker_mode")?,
            memory_checker_mode: optional_mode_to_db(
                modes.memory_checker_mode,
                "memory_checker_mode",
            )?,
            param_checker_mode: optional_mode_to_db(
                modes.param_checker_mode,
                "param_checker_mode",
            )?,
            approval_checker_mode: optional_mode_to_db(
                modes.approval_checker_mode,
                "approval_checker_mode",
            )?,
            updated_at: now,
        };

        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            // Lock the environment row so a concurrent soft-delete cannot
            // slip between the existence check and the upsert.
            let environment_exists = workspace_environments::table
                .filter(workspace_environments::workspace_id.eq(workspace_id))
                .filter(workspace_environments::id.eq(environment_id))
                .filter(workspace_environments::deleted_at.is_null())
                .select(workspace_environments::id)
                .for_update()
                .first::<String>(conn)
                .await
                .optional()
                .map_err(|e| StorageError::Internal(format!("resolve environment: {e}")))?
                .is_some();
            if !environment_exists {
                return Err(StorageError::NotFound);
            }

            diesel::insert_into(environment_checker_modes::table)
                .values(&record)
                .on_conflict((
                    environment_checker_modes::workspace_id,
                    environment_checker_modes::environment_id,
                ))
                .do_update()
                .set(&record)
                .execute(conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("put environment checker modes: {e}"))
                })?;
            Ok(())
        })
        .await?;

        let mut persisted = modes.clone();
        persisted.updated_at = Some(now.to_rfc3339());
        Ok(persisted)
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = workspace_settings)]
#[diesel(treat_none_as_null = true)]
struct SettingsWriteRecord {
    workspace_id: String,
    default_action: String,
    escalation_webhook_url: Option<String>,
    telemetry_enabled: bool,
    retention_days: String,
    config: Value,
    updated_at: DateTime<Utc>,
    data_handling_mode: String,
    flow_checker_mode: String,
    memory_checker_mode: String,
    param_checker_mode: String,
    approval_checker_mode: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = environment_checker_modes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct EnvironmentCheckerModesRecord {
    flow_checker_mode: Option<String>,
    memory_checker_mode: Option<String>,
    param_checker_mode: Option<String>,
    approval_checker_mode: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = environment_checker_modes)]
#[diesel(treat_none_as_null = true)]
struct EnvironmentCheckerModesWriteRecord {
    workspace_id: String,
    environment_id: String,
    flow_checker_mode: Option<String>,
    memory_checker_mode: Option<String>,
    param_checker_mode: Option<String>,
    approval_checker_mode: Option<String>,
    updated_at: DateTime<Utc>,
}

fn settings_from_record(row: SettingsRecord) -> Result<WorkspaceSettings, StorageError> {
    let data_handling_mode = parse_data_handling_mode(&row.data_handling_mode)?;
    let flow_checker_mode = parse_enforcement_mode("flow_checker_mode", &row.flow_checker_mode)?;
    let memory_checker_mode =
        parse_enforcement_mode("memory_checker_mode", &row.memory_checker_mode)?;
    let param_checker_mode = parse_enforcement_mode("param_checker_mode", &row.param_checker_mode)?;
    let approval_checker_mode =
        parse_enforcement_mode("approval_checker_mode", &row.approval_checker_mode)?;
    Ok(WorkspaceSettings {
        default_action: row.default_action,
        escalation_webhook_url: row.escalation_webhook_url,
        telemetry_enabled: row.telemetry_enabled,
        retention_days: row.retention_days,
        data_handling_mode,
        flow_checker_mode,
        memory_checker_mode,
        param_checker_mode,
        approval_checker_mode,
        config: row.config,
        updated_at: Some(row.updated_at.to_rfc3339()),
    })
}

fn environment_checker_modes_from_record(
    row: EnvironmentCheckerModesRecord,
) -> Result<EnvironmentCheckerModes, StorageError> {
    Ok(EnvironmentCheckerModes {
        flow_checker_mode: parse_optional_mode("flow_checker_mode", row.flow_checker_mode)?,
        memory_checker_mode: parse_optional_mode("memory_checker_mode", row.memory_checker_mode)?,
        param_checker_mode: parse_optional_mode("param_checker_mode", row.param_checker_mode)?,
        approval_checker_mode: parse_optional_mode(
            "approval_checker_mode",
            row.approval_checker_mode,
        )?,
        updated_at: Some(row.updated_at.to_rfc3339()),
    })
}

fn parse_optional_mode(
    column: &str,
    raw: Option<String>,
) -> Result<Option<EnforcementMode>, StorageError> {
    raw.map(|raw| parse_enforcement_mode(column, &raw))
        .transpose()
}

/// Serialize a wire-enum value to its snake_case DB string via serde so
/// the DB mapping can never drift from the wire mapping. The read path
/// (`parse_enforcement_mode`) round-trips through serde the same way,
/// and the CHECK constraints on the mode columns pin the accepted
/// values, so a serde-representation change fails loudly at write time
/// instead of corrupting rows.
fn mode_to_db<T: serde::Serialize>(value: T, column: &str) -> Result<String, StorageError> {
    match serde_json::to_value(value) {
        Ok(Value::String(s)) => Ok(s),
        other => Err(StorageError::Internal(format!(
            "{column} did not serialize to a string: {other:?}"
        ))),
    }
}

fn optional_mode_to_db(
    value: Option<EnforcementMode>,
    column: &str,
) -> Result<Option<String>, StorageError> {
    value.map(|mode| mode_to_db(mode, column)).transpose()
}

fn parse_data_handling_mode(raw: &str) -> Result<DataHandlingMode, StorageError> {
    serde_json::from_value::<DataHandlingMode>(Value::String(raw.to_string())).map_err(|e| {
        StorageError::Internal(format!(
            "workspace_settings.data_handling_mode is invalid: {e}"
        ))
    })
}

fn parse_enforcement_mode(column: &str, raw: &str) -> Result<EnforcementMode, StorageError> {
    serde_json::from_value::<EnforcementMode>(Value::String(raw.to_string()))
        .map_err(|e| StorageError::Internal(format!("workspace_settings.{column} is invalid: {e}")))
}
