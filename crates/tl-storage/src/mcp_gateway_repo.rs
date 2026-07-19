mod assignments;
mod connections;
mod tools;

use chrono::{DateTime, Utc};
use tl_core::{
    McpGatewayAuthKind, McpGatewayConnection, McpGatewayCredentialStatus, McpGatewaySyncStatus,
    McpGatewayTool, SideEffectClass,
};
use uuid::Uuid;

use crate::models::{McpServerConnectionRecord, McpToolRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::StorageError;

#[derive(Clone)]
pub struct McpGatewayRepo {
    pool: DbPool,
}

#[derive(Debug, Clone)]
pub struct NewMcpConnection {
    pub workspace_id: String,
    pub id: Uuid,
    pub display_name: String,
    pub server_slug: String,
    pub endpoint_url: String,
    pub auth_kind: McpGatewayAuthKind,
    pub encrypted_credential: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub enum CredentialPatch {
    #[default]
    Preserve,
    Clear,
    Replace(String),
}

#[derive(Debug, Clone, Default)]
pub struct McpConnectionPatch {
    pub display_name: Option<String>,
    pub endpoint_url: Option<String>,
    pub auth_kind: Option<McpGatewayAuthKind>,
    pub credential: CredentialPatch,
    pub enabled: Option<bool>,
    pub invalidate_catalog: bool,
}

#[derive(Debug, Clone)]
pub struct CatalogToolInput {
    pub upstream_name: String,
    pub public_name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub annotations: serde_json::Value,
    pub schema_hash: String,
}

#[derive(Debug, Clone)]
pub struct McpConnectionSecret {
    pub connection: McpGatewayConnection,
    pub encrypted_credential: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EntitledMcpTool {
    pub tool: McpGatewayTool,
    pub endpoint_url: String,
    pub auth_kind: McpGatewayAuthKind,
    pub encrypted_credential: Option<String>,
    pub connection_updated_at: DateTime<Utc>,
}

impl McpGatewayRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|error| StorageError::Internal(format!("db pool: {error}")))
    }
}

fn connection_record_to_wire(
    row: &McpServerConnectionRecord,
    tool_count: u32,
) -> Result<McpGatewayConnection, StorageError> {
    let auth_kind = parse_auth_kind(&row.auth_kind)?;
    let credential_status = match auth_kind {
        McpGatewayAuthKind::None => McpGatewayCredentialStatus::NotRequired,
        McpGatewayAuthKind::StaticBearer if row.encrypted_credential.is_some() => {
            McpGatewayCredentialStatus::Configured
        }
        McpGatewayAuthKind::StaticBearer => McpGatewayCredentialStatus::Missing,
    };
    Ok(McpGatewayConnection {
        id: row.id.to_string(),
        display_name: row.display_name.clone(),
        server_slug: row.server_slug.clone(),
        endpoint_url: row.endpoint_url.clone(),
        auth_kind,
        credential_status,
        enabled: row.enabled,
        last_sync_status: match row.last_sync_status.as_str() {
            "never" => McpGatewaySyncStatus::Never,
            "succeeded" => McpGatewaySyncStatus::Succeeded,
            "failed" => McpGatewaySyncStatus::Failed,
            other => {
                return Err(StorageError::Internal(format!(
                    "invalid MCP sync status: {other}"
                )))
            }
        },
        last_sync_error: row.last_sync_error.clone(),
        last_synced_at: row.last_synced_at.map(|time| time.to_rfc3339()),
        tool_count,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}

fn tool_record_to_wire(
    row: McpToolRecord,
    connection_name: String,
    assigned_user_ids: Vec<String>,
) -> Result<McpGatewayTool, StorageError> {
    let side_effect: SideEffectClass =
        serde_json::from_value(serde_json::Value::String(row.side_effect.clone()))
            .map_err(|error| StorageError::Internal(format!("invalid MCP side effect: {error}")))?;
    let catalog_status = match row.catalog_status.as_str() {
        "active" => tl_core::McpGatewayCatalogStatus::Active,
        "schema_changed" => tl_core::McpGatewayCatalogStatus::SchemaChanged,
        "missing" => tl_core::McpGatewayCatalogStatus::Missing,
        other => {
            return Err(StorageError::Internal(format!(
                "invalid MCP catalog status: {other}"
            )))
        }
    };
    Ok(McpGatewayTool {
        id: row.id.to_string(),
        connection_id: row.connection_id.to_string(),
        connection_name,
        upstream_name: row.upstream_name,
        public_name: row.public_name,
        title: row.title,
        description: row.description,
        input_schema: row.input_schema,
        output_schema: row.output_schema,
        annotations: row.annotations,
        schema_hash: row.schema_hash,
        side_effect,
        catalog_status,
        assigned_user_ids,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}

fn auth_kind_text(kind: McpGatewayAuthKind) -> &'static str {
    match kind {
        McpGatewayAuthKind::None => "none",
        McpGatewayAuthKind::StaticBearer => "static_bearer",
    }
}

fn parse_auth_kind(value: &str) -> Result<McpGatewayAuthKind, StorageError> {
    match value {
        "none" => Ok(McpGatewayAuthKind::None),
        "static_bearer" => Ok(McpGatewayAuthKind::StaticBearer),
        other => Err(StorageError::Internal(format!(
            "invalid MCP auth kind: {other}"
        ))),
    }
}

fn side_effect_text(value: SideEffectClass) -> Result<String, StorageError> {
    serde_json::to_value(value)
        .and_then(serde_json::from_value)
        .map_err(|error| StorageError::Internal(format!("MCP side effect: {error}")))
}
