use std::collections::HashMap;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{McpGatewayConnection, McpGatewayTool, McpGatewayToolAssignment, SideEffectClass};
use uuid::Uuid;

use super::assignments::assignment_views_for;
use super::connections::tool_count_for;
use super::{
    connection_record_to_wire, parse_auth_kind, side_effect_text, tool_record_to_wire,
    CatalogToolInput, EntitledMcpTool, McpGatewayRepo,
};
use crate::models::{McpServerConnectionRecord, McpToolRecord, NewMcpTool};
use crate::schema::{mcp_agent_tool_assignments, mcp_server_connections, mcp_tools};
use crate::StorageError;

impl McpGatewayRepo {
    pub async fn replace_catalog_snapshot(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        tools: Vec<CatalogToolInput>,
    ) -> Result<McpGatewayConnection, StorageError> {
        let mut conn = self.connection().await?;
        let workspace = workspace_id.to_string();
        let connection = conn
            .transaction::<McpServerConnectionRecord, StorageError, _>(async move |conn| {
                let _connection = mcp_server_connections::table
                    .filter(mcp_server_connections::workspace_id.eq(&workspace))
                    .filter(mcp_server_connections::id.eq(connection_id))
                    .select(McpServerConnectionRecord::as_select())
                    .first::<McpServerConnectionRecord>(&mut *conn)
                    .await?;

                diesel::update(
                    mcp_tools::table
                        .filter(mcp_tools::workspace_id.eq(&workspace))
                        .filter(mcp_tools::connection_id.eq(connection_id)),
                )
                .set((
                    mcp_tools::catalog_status.eq("missing"),
                    mcp_tools::updated_at.eq(now),
                ))
                .execute(&mut *conn)
                .await?;

                for tool in tools {
                    let row = NewMcpTool {
                        workspace_id: workspace.clone(),
                        id: Uuid::new_v4(),
                        connection_id,
                        upstream_name: tool.upstream_name,
                        public_name: tool.public_name,
                        title: tool.title,
                        description: tool.description,
                        input_schema: tool.input_schema,
                        output_schema: tool.output_schema,
                        annotations: tool.annotations,
                        schema_hash: tool.schema_hash,
                        side_effect: "api_mutation".to_string(),
                        catalog_status: "active".to_string(),
                    };
                    diesel::insert_into(mcp_tools::table)
                        .values(&row)
                        .on_conflict((
                            mcp_tools::workspace_id,
                            mcp_tools::connection_id,
                            mcp_tools::upstream_name,
                        ))
                        .do_update()
                        .set((
                            mcp_tools::public_name.eq(excluded(mcp_tools::public_name)),
                            mcp_tools::title.eq(excluded(mcp_tools::title)),
                            mcp_tools::description.eq(excluded(mcp_tools::description)),
                            mcp_tools::input_schema.eq(excluded(mcp_tools::input_schema)),
                            mcp_tools::output_schema.eq(excluded(mcp_tools::output_schema)),
                            mcp_tools::annotations.eq(excluded(mcp_tools::annotations)),
                            mcp_tools::schema_hash.eq(excluded(mcp_tools::schema_hash)),
                            mcp_tools::catalog_status.eq("active"),
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
                    mcp_server_connections::last_sync_status.eq("succeeded"),
                    mcp_server_connections::last_sync_error.eq::<Option<String>>(None),
                    mcp_server_connections::last_synced_at.eq(now),
                    mcp_server_connections::updated_at.eq(now),
                ))
                .returning(McpServerConnectionRecord::as_returning())
                .get_result::<McpServerConnectionRecord>(&mut *conn)
                .await
                .map_err(Into::into)
            })
            .await?;
        let count = tool_count_for(&mut conn, workspace_id, connection_id).await?;
        connection_record_to_wire(&connection, count)
    }

    pub async fn list_tools(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayTool>, StorageError> {
        let mut conn = self.connection().await?;
        let connections = mcp_server_connections::table
            .filter(mcp_server_connections::workspace_id.eq(workspace_id))
            .select((
                mcp_server_connections::id,
                mcp_server_connections::display_name,
            ))
            .load::<(Uuid, String)>(&mut conn)
            .await?
            .into_iter()
            .collect::<HashMap<_, _>>();
        let rows = mcp_tools::table
            .filter(mcp_tools::workspace_id.eq(workspace_id))
            .order(mcp_tools::public_name.asc())
            .select(McpToolRecord::as_select())
            .load::<McpToolRecord>(&mut conn)
            .await?;
        let mut output = Vec::with_capacity(rows.len());
        for row in rows {
            let connection_name = connections
                .get(&row.connection_id)
                .cloned()
                .ok_or_else(|| StorageError::Internal("MCP connection missing".to_string()))?;
            let (assigned, agent_assignments, unbound) =
                assignment_views_for(&mut conn, workspace_id, row.id).await?;
            output.push(tool_record_to_wire(
                row,
                connection_name,
                assigned,
                agent_assignments,
                unbound,
            )?);
        }
        Ok(output)
    }

    pub async fn update_tool_side_effect(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        side_effect: SideEffectClass,
    ) -> Result<McpGatewayTool, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::update(
            mcp_tools::table
                .filter(mcp_tools::workspace_id.eq(workspace_id))
                .filter(mcp_tools::id.eq(tool_id)),
        )
        .set((
            mcp_tools::side_effect.eq(side_effect_text(side_effect)?),
            mcp_tools::updated_at.eq(now),
        ))
        .returning(McpToolRecord::as_returning())
        .get_result::<McpToolRecord>(&mut conn)
        .await?;
        let connection_name = mcp_server_connections::table
            .filter(mcp_server_connections::workspace_id.eq(workspace_id))
            .filter(mcp_server_connections::id.eq(row.connection_id))
            .select(mcp_server_connections::display_name)
            .first::<String>(&mut conn)
            .await?;
        let (assigned, agent_assignments, unbound) =
            assignment_views_for(&mut conn, workspace_id, row.id).await?;
        tool_record_to_wire(row, connection_name, assigned, agent_assignments, unbound)
    }

    pub async fn mark_tool_schema_changed(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let changed = diesel::update(
            mcp_tools::table
                .filter(mcp_tools::workspace_id.eq(workspace_id))
                .filter(mcp_tools::id.eq(tool_id)),
        )
        .set((
            mcp_tools::catalog_status.eq("schema_changed"),
            mcp_tools::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    pub async fn resolve_entitled_tool(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        agent_id: &str,
        public_name: &str,
    ) -> Result<EntitledMcpTool, StorageError> {
        let mut conn = self.connection().await?;
        let (tool, connection) = mcp_agent_tool_assignments::table
            .inner_join(
                mcp_tools::table.on(mcp_agent_tool_assignments::workspace_id
                    .eq(mcp_tools::workspace_id)
                    .and(mcp_agent_tool_assignments::tool_id.eq(mcp_tools::id))),
            )
            .inner_join(
                mcp_server_connections::table.on(mcp_tools::workspace_id
                    .eq(mcp_server_connections::workspace_id)
                    .and(mcp_tools::connection_id.eq(mcp_server_connections::id))),
            )
            .filter(mcp_agent_tool_assignments::workspace_id.eq(workspace_id))
            .filter(mcp_agent_tool_assignments::user_id.eq(user_id))
            .filter(mcp_agent_tool_assignments::agent_id.eq(agent_id))
            .filter(mcp_tools::catalog_status.eq("active"))
            .filter(mcp_server_connections::enabled.eq(true))
            .filter(mcp_tools::public_name.eq(public_name))
            .select((
                McpToolRecord::as_select(),
                McpServerConnectionRecord::as_select(),
            ))
            .first::<(McpToolRecord, McpServerConnectionRecord)>(&mut conn)
            .await?;
        entitled_from_records(tool, connection, user_id, agent_id)
    }

    pub async fn list_entitled_tools(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        agent_id: &str,
        after_public_name: Option<&str>,
        limit: u32,
    ) -> Result<Vec<EntitledMcpTool>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = mcp_agent_tool_assignments::table
            .inner_join(
                mcp_tools::table.on(mcp_agent_tool_assignments::workspace_id
                    .eq(mcp_tools::workspace_id)
                    .and(mcp_agent_tool_assignments::tool_id.eq(mcp_tools::id))),
            )
            .inner_join(
                mcp_server_connections::table.on(mcp_tools::workspace_id
                    .eq(mcp_server_connections::workspace_id)
                    .and(mcp_tools::connection_id.eq(mcp_server_connections::id))),
            )
            .filter(mcp_agent_tool_assignments::workspace_id.eq(workspace_id))
            .filter(mcp_agent_tool_assignments::user_id.eq(user_id))
            .filter(mcp_agent_tool_assignments::agent_id.eq(agent_id))
            .filter(mcp_tools::catalog_status.eq("active"))
            .filter(mcp_server_connections::enabled.eq(true))
            .into_boxed();
        if let Some(cursor) = after_public_name {
            query = query.filter(mcp_tools::public_name.gt(cursor));
        }
        let rows = query
            .order(mcp_tools::public_name.asc())
            .limit(i64::from(limit))
            .select((
                McpToolRecord::as_select(),
                McpServerConnectionRecord::as_select(),
            ))
            .load::<(McpToolRecord, McpServerConnectionRecord)>(&mut conn)
            .await?;
        rows.into_iter()
            .map(|(tool, connection)| entitled_from_records(tool, connection, user_id, agent_id))
            .collect()
    }
}

fn entitled_from_records(
    tool: McpToolRecord,
    connection: McpServerConnectionRecord,
    user_id: Uuid,
    agent_id: &str,
) -> Result<EntitledMcpTool, StorageError> {
    let endpoint_url = connection.endpoint_url.clone();
    let auth_kind = parse_auth_kind(&connection.auth_kind)?;
    let encrypted_credential = connection.encrypted_credential.clone();
    let connection_updated_at = connection.updated_at;
    Ok(EntitledMcpTool {
        tool: tool_record_to_wire(
            tool,
            connection.display_name,
            vec![user_id.to_string()],
            vec![McpGatewayToolAssignment {
                user_id: user_id.to_string(),
                agent_id: agent_id.to_string(),
            }],
            Vec::new(),
        )?,
        endpoint_url,
        auth_kind,
        encrypted_credential,
        connection_updated_at,
    })
}
