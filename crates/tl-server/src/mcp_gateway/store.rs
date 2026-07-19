use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{McpGatewayAuthKind, McpGatewayConnection, McpGatewayTool, SideEffectClass};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum McpGatewayStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal: {0}")]
    Internal(String),
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

#[async_trait]
pub trait McpGatewayStore: Send + Sync {
    async fn list_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayConnection>, McpGatewayStoreError>;
    async fn create_connection(
        &self,
        input: NewMcpConnection,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError>;
    async fn get_connection_secret(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<McpConnectionSecret, McpGatewayStoreError>;
    async fn update_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        patch: McpConnectionPatch,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError>;
    async fn record_sync_failure(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        safe_error: &str,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError>;
    async fn delete_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<(), McpGatewayStoreError>;
    async fn replace_catalog_snapshot(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        tools: Vec<CatalogToolInput>,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError>;
    async fn list_tools(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayTool>, McpGatewayStoreError>;
    async fn update_tool_side_effect(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        side_effect: SideEffectClass,
    ) -> Result<McpGatewayTool, McpGatewayStoreError>;
    async fn mark_tool_schema_changed(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
    ) -> Result<(), McpGatewayStoreError>;
    async fn resolve_entitled_tool(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        public_name: &str,
    ) -> Result<EntitledMcpTool, McpGatewayStoreError>;
    async fn list_entitled_tools(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        after_public_name: Option<&str>,
        limit: u32,
    ) -> Result<Vec<EntitledMcpTool>, McpGatewayStoreError>;
    async fn replace_assignments(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        user_ids: Vec<Uuid>,
        created_by: Option<Uuid>,
    ) -> Result<Vec<Uuid>, McpGatewayStoreError>;
}
