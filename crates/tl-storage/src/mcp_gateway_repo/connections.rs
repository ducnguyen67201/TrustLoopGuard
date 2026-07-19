use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use tl_core::McpGatewayConnection;
use uuid::Uuid;

use super::{
    auth_kind_text, connection_record_to_wire, CredentialPatch, McpConnectionPatch,
    McpConnectionSecret, McpGatewayRepo, NewMcpConnection,
};
use crate::models::{McpServerConnectionRecord, NewMcpServerConnection};
use crate::schema::{mcp_server_connections, mcp_tools};
use crate::StorageError;

impl McpGatewayRepo {
    pub async fn list_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayConnection>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = mcp_server_connections::table
            .filter(mcp_server_connections::workspace_id.eq(workspace_id))
            .order(mcp_server_connections::created_at.desc())
            .select(McpServerConnectionRecord::as_select())
            .load::<McpServerConnectionRecord>(&mut conn)
            .await?;
        let mut output = Vec::with_capacity(rows.len());
        for row in rows {
            let count = tool_count_for(&mut conn, workspace_id, row.id).await?;
            output.push(connection_record_to_wire(&row, count)?);
        }
        Ok(output)
    }

    pub async fn create_connection(
        &self,
        input: NewMcpConnection,
    ) -> Result<McpGatewayConnection, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::insert_into(mcp_server_connections::table)
            .values(NewMcpServerConnection {
                workspace_id: input.workspace_id,
                id: input.id,
                display_name: input.display_name,
                server_slug: input.server_slug,
                endpoint_url: input.endpoint_url,
                auth_kind: auth_kind_text(input.auth_kind).to_string(),
                encrypted_credential: input.encrypted_credential,
                enabled: input.enabled,
            })
            .returning(McpServerConnectionRecord::as_returning())
            .get_result::<McpServerConnectionRecord>(&mut conn)
            .await?;
        connection_record_to_wire(&row, 0)
    }

    pub async fn get_connection_secret(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<McpConnectionSecret, StorageError> {
        let mut conn = self.connection().await?;
        let row = mcp_server_connections::table
            .filter(mcp_server_connections::workspace_id.eq(workspace_id))
            .filter(mcp_server_connections::id.eq(connection_id))
            .select(McpServerConnectionRecord::as_select())
            .first::<McpServerConnectionRecord>(&mut conn)
            .await?;
        let count = tool_count_for(&mut conn, workspace_id, connection_id).await?;
        Ok(McpConnectionSecret {
            connection: connection_record_to_wire(&row, count)?,
            encrypted_credential: row.encrypted_credential,
        })
    }

    pub async fn update_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        patch: McpConnectionPatch,
    ) -> Result<McpGatewayConnection, StorageError> {
        let mut conn = self.connection().await?;
        let workspace = workspace_id.to_string();
        let row = conn
            .transaction::<McpServerConnectionRecord, StorageError, _>(async move |conn| {
                let mut current = mcp_server_connections::table
                    .filter(mcp_server_connections::workspace_id.eq(&workspace))
                    .filter(mcp_server_connections::id.eq(connection_id))
                    .select(McpServerConnectionRecord::as_select())
                    .first::<McpServerConnectionRecord>(&mut *conn)
                    .await?;

                if let Some(display_name) = patch.display_name {
                    current.display_name = display_name;
                }
                if let Some(endpoint_url) = patch.endpoint_url {
                    current.endpoint_url = endpoint_url;
                }
                if let Some(auth_kind) = patch.auth_kind {
                    current.auth_kind = auth_kind_text(auth_kind).to_string();
                }
                match patch.credential {
                    CredentialPatch::Preserve => {}
                    CredentialPatch::Clear => current.encrypted_credential = None,
                    CredentialPatch::Replace(value) => current.encrypted_credential = Some(value),
                }
                if current.auth_kind == "none" {
                    current.encrypted_credential = None;
                }
                if let Some(enabled) = patch.enabled {
                    current.enabled = enabled;
                }
                if patch.invalidate_catalog {
                    current.last_sync_status = "never".to_string();
                    current.last_sync_error = None;
                    current.last_synced_at = None;
                    diesel::update(
                        mcp_tools::table
                            .filter(mcp_tools::workspace_id.eq(&workspace))
                            .filter(mcp_tools::connection_id.eq(connection_id)),
                    )
                    .set((
                        mcp_tools::catalog_status.eq("schema_changed"),
                        mcp_tools::updated_at.eq(now),
                    ))
                    .execute(&mut *conn)
                    .await?;
                }

                diesel::update(
                    mcp_server_connections::table
                        .filter(mcp_server_connections::workspace_id.eq(&workspace))
                        .filter(mcp_server_connections::id.eq(connection_id)),
                )
                .set((
                    mcp_server_connections::display_name.eq(current.display_name),
                    mcp_server_connections::endpoint_url.eq(current.endpoint_url),
                    mcp_server_connections::auth_kind.eq(current.auth_kind),
                    mcp_server_connections::encrypted_credential.eq(current.encrypted_credential),
                    mcp_server_connections::enabled.eq(current.enabled),
                    mcp_server_connections::last_sync_status.eq(current.last_sync_status),
                    mcp_server_connections::last_sync_error.eq(current.last_sync_error),
                    mcp_server_connections::last_synced_at.eq(current.last_synced_at),
                    mcp_server_connections::updated_at.eq(now),
                ))
                .returning(McpServerConnectionRecord::as_returning())
                .get_result::<McpServerConnectionRecord>(&mut *conn)
                .await
                .map_err(Into::into)
            })
            .await?;
        let count = tool_count_for(&mut conn, workspace_id, connection_id).await?;
        connection_record_to_wire(&row, count)
    }

    pub async fn record_sync_failure(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        safe_error: &str,
    ) -> Result<McpGatewayConnection, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::update(
            mcp_server_connections::table
                .filter(mcp_server_connections::workspace_id.eq(workspace_id))
                .filter(mcp_server_connections::id.eq(connection_id)),
        )
        .set((
            mcp_server_connections::last_sync_status.eq("failed"),
            mcp_server_connections::last_sync_error.eq(Some(safe_error)),
            mcp_server_connections::updated_at.eq(now),
        ))
        .returning(McpServerConnectionRecord::as_returning())
        .get_result::<McpServerConnectionRecord>(&mut conn)
        .await?;
        let count = tool_count_for(&mut conn, workspace_id, connection_id).await?;
        connection_record_to_wire(&row, count)
    }

    pub async fn delete_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let deleted = diesel::delete(
            mcp_server_connections::table
                .filter(mcp_server_connections::workspace_id.eq(workspace_id))
                .filter(mcp_server_connections::id.eq(connection_id)),
        )
        .execute(&mut conn)
        .await?;
        if deleted == 1 {
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }
}

pub(super) async fn tool_count_for(
    conn: &mut AsyncPgConnection,
    workspace_id: &str,
    connection_id: Uuid,
) -> Result<u32, StorageError> {
    let count = mcp_tools::table
        .filter(mcp_tools::workspace_id.eq(workspace_id))
        .filter(mcp_tools::connection_id.eq(connection_id))
        .count()
        .get_result::<i64>(conn)
        .await?;
    u32::try_from(count)
        .map_err(|_| StorageError::Internal("MCP tool count exceeds u32".to_string()))
}
