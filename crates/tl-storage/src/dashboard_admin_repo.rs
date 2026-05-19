//! Dashboard runtime-admin repository.

use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde_json::Value;
use tl_core::{DashboardApiKey, WorkspaceSettings};
use uuid::Uuid;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{workspace_api_keys, workspace_settings};
use crate::StorageError;

#[derive(Clone)]
pub struct DashboardAdminRepo {
    pool: DbPool,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_api_keys)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ApiKeyRecord {
    id: String,
    name: String,
    key_prefix: String,
    status: String,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_api_keys)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ApiKeyAuthRecord {
    pub id: String,
    pub workspace_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = workspace_api_keys)]
struct NewApiKeyRecord<'a> {
    id: &'a str,
    workspace_id: &'a str,
    name: &'a str,
    key_prefix: &'a str,
    key_hash: &'a str,
    created_by_user_id: Option<Uuid>,
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
}

impl DashboardAdminRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_api_keys(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<DashboardApiKey>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = workspace_api_keys::table
            .filter(workspace_api_keys::workspace_id.eq(workspace_id))
            .order(workspace_api_keys::created_at.desc())
            .select(ApiKeyRecord::as_select())
            .load::<ApiKeyRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list api keys: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|row| DashboardApiKey {
                id: row.id,
                name: row.name,
                prefix: row.key_prefix,
                status: row.status,
                created_at: row.created_at.to_rfc3339(),
                last_used_at: row.last_used_at.map(|value| value.to_rfc3339()),
                created_by: row.created_by_user_id.map(|value| value.to_string()),
            })
            .collect())
    }

    pub async fn create_api_key(
        &self,
        id: &str,
        workspace_id: &str,
        name: &str,
        key_prefix: &str,
        key_hash: &str,
        created_by_user_id: Option<Uuid>,
    ) -> Result<DashboardApiKey, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::insert_into(workspace_api_keys::table)
            .values(NewApiKeyRecord {
                id,
                workspace_id,
                name,
                key_prefix,
                key_hash,
                created_by_user_id,
            })
            .returning(ApiKeyRecord::as_returning())
            .get_result::<ApiKeyRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("create api key: {e}")))?;

        Ok(DashboardApiKey {
            id: row.id,
            name: row.name,
            prefix: row.key_prefix,
            status: row.status,
            created_at: row.created_at.to_rfc3339(),
            last_used_at: row.last_used_at.map(|value| value.to_rfc3339()),
            created_by: row.created_by_user_id.map(|value| value.to_string()),
        })
    }

    pub async fn batch_revoke_api_keys(
        &self,
        workspace_id: &str,
        api_key_ids: &[String],
    ) -> Result<Vec<DashboardApiKey>, StorageError> {
        let mut ids = Vec::new();
        for id in api_key_ids {
            if !ids.iter().any(|existing: &String| existing == id) {
                ids.push(id.clone());
            }
        }
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            let existing = workspace_api_keys::table
                .filter(workspace_api_keys::workspace_id.eq(workspace_id))
                .filter(workspace_api_keys::id.eq_any(&ids))
                .select(workspace_api_keys::id)
                .load::<String>(conn)
                .await?;
            if existing.len() != ids.len() {
                return Err(StorageError::NotFound);
            }

            diesel::update(
                workspace_api_keys::table
                    .filter(workspace_api_keys::workspace_id.eq(workspace_id))
                    .filter(workspace_api_keys::id.eq_any(&ids)),
            )
            .set((
                workspace_api_keys::status.eq(sql::<Text>("'revoked'::api_key_status")),
                workspace_api_keys::revoked_at.eq(diesel::dsl::now),
            ))
            .execute(conn)
            .await?;

            let rows = workspace_api_keys::table
                .filter(workspace_api_keys::workspace_id.eq(workspace_id))
                .filter(workspace_api_keys::id.eq_any(&ids))
                .order(workspace_api_keys::created_at.desc())
                .select(ApiKeyRecord::as_select())
                .load::<ApiKeyRecord>(conn)
                .await?;

            Ok(rows.into_iter().map(api_key_to_wire).collect())
        })
        .await
    }

    pub async fn verify_api_key_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<ApiKeyAuthRecord>, StorageError> {
        let mut conn = self.connection().await?;
        let row = workspace_api_keys::table
            .filter(workspace_api_keys::key_hash.eq(key_hash))
            .filter(workspace_api_keys::status.eq(sql::<Text>("'active'::api_key_status")))
            .filter(workspace_api_keys::revoked_at.is_null())
            .select(ApiKeyAuthRecord::as_select())
            .first::<ApiKeyAuthRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("verify api key: {e}")))?;

        if let Some(row) = row.as_ref() {
            diesel::update(workspace_api_keys::table.filter(workspace_api_keys::id.eq(&row.id)))
                .set(workspace_api_keys::last_used_at.eq(diesel::dsl::now))
                .execute(&mut conn)
                .await
                .map_err(|e| StorageError::Internal(format!("mark api key used: {e}")))?;
        }

        Ok(row)
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

        Ok(row.map(|row| WorkspaceSettings {
            default_action: row.default_action,
            escalation_webhook_url: row.escalation_webhook_url,
            telemetry_enabled: row.telemetry_enabled,
            retention_days: row.retention_days,
            data_handling_mode: tl_core::DataHandlingMode::RawAllowed,
            config: row.config,
            updated_at: Some(row.updated_at.to_rfc3339()),
        }))
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

fn api_key_to_wire(row: ApiKeyRecord) -> DashboardApiKey {
    DashboardApiKey {
        id: row.id,
        name: row.name,
        prefix: row.key_prefix,
        status: row.status,
        created_at: row.created_at.to_rfc3339(),
        last_used_at: row.last_used_at.map(|value| value.to_rfc3339()),
        created_by: row.created_by_user_id.map(|value| value.to_string()),
    }
}
