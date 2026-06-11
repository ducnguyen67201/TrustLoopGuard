//! Dashboard runtime-admin repository.

mod api_keys;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::Value;
use tl_core::{DataHandlingMode, EnforcementMode, WorkspaceSettings};

use crate::postgres::{DbConnection, DbPool};
use crate::schema::workspace_settings;
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

        row.map(|row| {
            let data_handling_mode = parse_data_handling_mode(&row.data_handling_mode)?;
            let flow_checker_mode =
                parse_enforcement_mode("flow_checker_mode", &row.flow_checker_mode)?;
            let memory_checker_mode =
                parse_enforcement_mode("memory_checker_mode", &row.memory_checker_mode)?;
            let param_checker_mode =
                parse_enforcement_mode("param_checker_mode", &row.param_checker_mode)?;
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
        })
        .transpose()
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
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
