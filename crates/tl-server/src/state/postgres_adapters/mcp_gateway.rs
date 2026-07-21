use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{McpGatewayConnection, McpGatewayTool, SideEffectClass};
use tl_storage::{
    CatalogToolInput as StorageCatalogToolInput, CredentialPatch as StorageCredentialPatch,
    McpConnectionPatch as StorageConnectionPatch, McpGatewayRepo,
    NewMcpConnection as StorageNewMcpConnection, StorageError,
};
use uuid::Uuid;

use crate::mcp_gateway::{
    CatalogToolInput, CredentialPatch, EntitledMcpTool, McpConnectionPatch, McpConnectionSecret,
    McpGatewayStore, McpGatewayStoreError, NewMcpConnection,
};

pub struct PostgresMcpGatewayAdapter {
    repo: Arc<McpGatewayRepo>,
}
impl PostgresMcpGatewayAdapter {
    pub fn new(repo: Arc<McpGatewayRepo>) -> Self {
        Self { repo }
    }
}

fn error(error: StorageError) -> McpGatewayStoreError {
    match error {
        StorageError::NotFound => McpGatewayStoreError::NotFound,
        StorageError::Conflict => {
            McpGatewayStoreError::Conflict("request conflicts with current gateway state".into())
        }
        other => McpGatewayStoreError::Internal(other.to_string()),
    }
}

#[async_trait]
impl McpGatewayStore for PostgresMcpGatewayAdapter {
    async fn list_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayConnection>, McpGatewayStoreError> {
        self.repo
            .list_connections(workspace_id)
            .await
            .map_err(error)
    }
    async fn create_connection(
        &self,
        input: NewMcpConnection,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        self.repo
            .create_connection(StorageNewMcpConnection {
                workspace_id: input.workspace_id,
                id: input.id,
                display_name: input.display_name,
                server_slug: input.server_slug,
                endpoint_url: input.endpoint_url,
                auth_kind: input.auth_kind,
                encrypted_credential: input.encrypted_credential,
                enabled: input.enabled,
            })
            .await
            .map_err(error)
    }
    async fn get_connection_secret(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<McpConnectionSecret, McpGatewayStoreError> {
        self.repo
            .get_connection_secret(workspace_id, connection_id)
            .await
            .map(|value| McpConnectionSecret {
                connection: value.connection,
                encrypted_credential: value.encrypted_credential,
            })
            .map_err(error)
    }
    async fn update_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        patch: McpConnectionPatch,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        let credential = match patch.credential {
            CredentialPatch::Preserve => StorageCredentialPatch::Preserve,
            CredentialPatch::Clear => StorageCredentialPatch::Clear,
            CredentialPatch::Replace(value) => StorageCredentialPatch::Replace(value),
        };
        self.repo
            .update_connection(
                workspace_id,
                connection_id,
                StorageConnectionPatch {
                    display_name: patch.display_name,
                    endpoint_url: patch.endpoint_url,
                    auth_kind: patch.auth_kind,
                    credential,
                    enabled: patch.enabled,
                    invalidate_catalog: patch.invalidate_catalog,
                },
            )
            .await
            .map_err(error)
    }
    async fn record_sync_failure(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        safe_error: &str,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        self.repo
            .record_sync_failure(workspace_id, connection_id, safe_error)
            .await
            .map_err(error)
    }
    async fn delete_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<(), McpGatewayStoreError> {
        self.repo
            .delete_connection(workspace_id, connection_id)
            .await
            .map_err(error)
    }
    async fn replace_catalog_snapshot(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        tools: Vec<CatalogToolInput>,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        self.repo
            .replace_catalog_snapshot(
                workspace_id,
                connection_id,
                tools
                    .into_iter()
                    .map(|tool| StorageCatalogToolInput {
                        upstream_name: tool.upstream_name,
                        public_name: tool.public_name,
                        title: tool.title,
                        description: tool.description,
                        input_schema: tool.input_schema,
                        output_schema: tool.output_schema,
                        annotations: tool.annotations,
                        schema_hash: tool.schema_hash,
                    })
                    .collect(),
            )
            .await
            .map_err(error)
    }
    async fn list_tools(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayTool>, McpGatewayStoreError> {
        self.repo.list_tools(workspace_id).await.map_err(error)
    }
    async fn update_tool_side_effect(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        side_effect: SideEffectClass,
    ) -> Result<McpGatewayTool, McpGatewayStoreError> {
        self.repo
            .update_tool_side_effect(workspace_id, tool_id, side_effect)
            .await
            .map_err(error)
    }
    async fn mark_tool_schema_changed(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
    ) -> Result<(), McpGatewayStoreError> {
        self.repo
            .mark_tool_schema_changed(workspace_id, tool_id)
            .await
            .map_err(error)
    }
    async fn resolve_entitled_tool(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        agent_id: &str,
        public_name: &str,
    ) -> Result<EntitledMcpTool, McpGatewayStoreError> {
        self.repo
            .resolve_entitled_tool(workspace_id, user_id, agent_id, public_name)
            .await
            .map(|value| EntitledMcpTool {
                tool: value.tool,
                endpoint_url: value.endpoint_url,
                auth_kind: value.auth_kind,
                encrypted_credential: value.encrypted_credential,
                connection_updated_at: value.connection_updated_at,
            })
            .map_err(error)
    }
    async fn list_entitled_tools(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        agent_id: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<EntitledMcpTool>, McpGatewayStoreError> {
        self.repo
            .list_entitled_tools(workspace_id, user_id, agent_id, cursor, limit)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|value| EntitledMcpTool {
                        tool: value.tool,
                        endpoint_url: value.endpoint_url,
                        auth_kind: value.auth_kind,
                        encrypted_credential: value.encrypted_credential,
                        connection_updated_at: value.connection_updated_at,
                    })
                    .collect()
            })
            .map_err(error)
    }
    async fn replace_agent_assignments(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        agent_id: &str,
        user_ids: Vec<Uuid>,
        created_by: Option<Uuid>,
    ) -> Result<Vec<Uuid>, McpGatewayStoreError> {
        self.repo
            .replace_agent_assignments(workspace_id, tool_id, agent_id, user_ids, created_by)
            .await
            .map_err(error)
    }
}
