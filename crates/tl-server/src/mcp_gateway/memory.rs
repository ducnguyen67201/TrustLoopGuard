use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{
    McpGatewayAuthKind, McpGatewayCatalogStatus, McpGatewayConnection, McpGatewayCredentialStatus,
    McpGatewaySyncStatus, McpGatewayTool, SideEffectClass,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{
    CatalogToolInput, CredentialPatch, EntitledMcpTool, McpConnectionPatch, McpConnectionSecret,
    McpGatewayStore, McpGatewayStoreError, NewMcpConnection,
};

#[derive(Default)]
pub struct MemoryMcpGatewayStore {
    inner: RwLock<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    connections: HashMap<(String, Uuid), (McpGatewayConnection, Option<String>)>,
    tools: HashMap<(String, Uuid), McpGatewayTool>,
    assignments: HashMap<(String, Uuid), BTreeSet<Uuid>>,
}

fn credential_status(
    kind: McpGatewayAuthKind,
    secret: &Option<String>,
) -> McpGatewayCredentialStatus {
    match (kind, secret.is_some()) {
        (McpGatewayAuthKind::None, _) => McpGatewayCredentialStatus::NotRequired,
        (McpGatewayAuthKind::StaticBearer, true) => McpGatewayCredentialStatus::Configured,
        (McpGatewayAuthKind::StaticBearer, false) => McpGatewayCredentialStatus::Missing,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

#[async_trait]
impl McpGatewayStore for MemoryMcpGatewayStore {
    async fn list_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayConnection>, McpGatewayStoreError> {
        let state = self.inner.read().await;
        let mut values = state
            .connections
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, (value, _))| value.clone())
            .collect::<Vec<_>>();
        values.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(values)
    }

    async fn create_connection(
        &self,
        input: NewMcpConnection,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        let mut state = self.inner.write().await;
        if state
            .connections
            .contains_key(&(input.workspace_id.clone(), input.id))
        {
            return Err(McpGatewayStoreError::Conflict(
                "connection already exists".into(),
            ));
        }
        let timestamp = now();
        let value = McpGatewayConnection {
            id: input.id.to_string(),
            display_name: input.display_name,
            server_slug: input.server_slug,
            endpoint_url: input.endpoint_url,
            auth_kind: input.auth_kind,
            credential_status: credential_status(input.auth_kind, &input.encrypted_credential),
            enabled: input.enabled,
            last_sync_status: McpGatewaySyncStatus::Never,
            last_sync_error: None,
            last_synced_at: None,
            tool_count: 0,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        state.connections.insert(
            (input.workspace_id, input.id),
            (value.clone(), input.encrypted_credential),
        );
        Ok(value)
    }

    async fn get_connection_secret(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<McpConnectionSecret, McpGatewayStoreError> {
        let state = self.inner.read().await;
        let (connection, encrypted_credential) = state
            .connections
            .get(&(workspace_id.to_string(), connection_id))
            .ok_or(McpGatewayStoreError::NotFound)?;
        Ok(McpConnectionSecret {
            connection: connection.clone(),
            encrypted_credential: encrypted_credential.clone(),
        })
    }

    async fn update_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        patch: McpConnectionPatch,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        let mut state = self.inner.write().await;
        let invalidate_catalog = patch.invalidate_catalog;
        let output = {
            let (connection, secret) = state
                .connections
                .get_mut(&(workspace_id.to_string(), connection_id))
                .ok_or(McpGatewayStoreError::NotFound)?;
            if let Some(value) = patch.display_name {
                connection.display_name = value;
            }
            if let Some(value) = patch.endpoint_url {
                connection.endpoint_url = value;
            }
            if let Some(value) = patch.auth_kind {
                connection.auth_kind = value;
            }
            match patch.credential {
                CredentialPatch::Preserve => {}
                CredentialPatch::Clear => *secret = None,
                CredentialPatch::Replace(value) => *secret = Some(value),
            }
            if connection.auth_kind == McpGatewayAuthKind::None {
                *secret = None;
            }
            if let Some(value) = patch.enabled {
                connection.enabled = value;
            }
            connection.credential_status = credential_status(connection.auth_kind, secret);
            connection.updated_at = now();
            if invalidate_catalog {
                connection.last_sync_status = McpGatewaySyncStatus::Never;
                connection.last_sync_error = None;
                connection.last_synced_at = None;
            }
            connection.clone()
        };
        if invalidate_catalog {
            for ((workspace, _), tool) in state.tools.iter_mut() {
                if workspace == workspace_id && tool.connection_id == connection_id.to_string() {
                    tool.catalog_status = McpGatewayCatalogStatus::SchemaChanged;
                }
            }
        }
        Ok(output)
    }

    async fn record_sync_failure(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        safe_error: &str,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        let mut state = self.inner.write().await;
        let (connection, _) = state
            .connections
            .get_mut(&(workspace_id.to_string(), connection_id))
            .ok_or(McpGatewayStoreError::NotFound)?;
        connection.last_sync_status = McpGatewaySyncStatus::Failed;
        connection.last_sync_error = Some(safe_error.to_string());
        connection.updated_at = now();
        Ok(connection.clone())
    }
    async fn delete_connection(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
    ) -> Result<(), McpGatewayStoreError> {
        let mut state = self.inner.write().await;
        state
            .connections
            .remove(&(workspace_id.to_string(), connection_id))
            .ok_or(McpGatewayStoreError::NotFound)?;
        let ids = state
            .tools
            .iter()
            .filter(|((workspace, _), tool)| {
                workspace == workspace_id && tool.connection_id == connection_id.to_string()
            })
            .map(|((_, id), _)| *id)
            .collect::<Vec<_>>();
        for id in ids {
            state.tools.remove(&(workspace_id.to_string(), id));
            state.assignments.remove(&(workspace_id.to_string(), id));
        }
        Ok(())
    }

    async fn replace_catalog_snapshot(
        &self,
        workspace_id: &str,
        connection_id: Uuid,
        tools: Vec<CatalogToolInput>,
    ) -> Result<McpGatewayConnection, McpGatewayStoreError> {
        let mut state = self.inner.write().await;
        if !state
            .connections
            .contains_key(&(workspace_id.to_string(), connection_id))
        {
            return Err(McpGatewayStoreError::NotFound);
        }
        for ((workspace, _), tool) in state.tools.iter_mut() {
            if workspace == workspace_id && tool.connection_id == connection_id.to_string() {
                tool.catalog_status = McpGatewayCatalogStatus::Missing;
            }
        }
        for input in tools {
            let existing = state
                .tools
                .iter()
                .find(|((workspace, _), tool)| {
                    workspace == workspace_id
                        && tool.connection_id == connection_id.to_string()
                        && tool.upstream_name == input.upstream_name
                })
                .map(|((_, id), _)| *id)
                .unwrap_or_else(Uuid::new_v4);
            let assigned = state
                .assignments
                .get(&(workspace_id.to_string(), existing))
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|id| id.to_string())
                .collect();
            let timestamp = now();
            let created_at = state
                .tools
                .get(&(workspace_id.to_string(), existing))
                .map(|tool| tool.created_at.clone())
                .unwrap_or_else(|| timestamp.clone());
            let connection_name = state
                .connections
                .get(&(workspace_id.to_string(), connection_id))
                .map(|(value, _)| value.display_name.clone())
                .unwrap_or_default();
            state.tools.insert(
                (workspace_id.to_string(), existing),
                McpGatewayTool {
                    id: existing.to_string(),
                    connection_id: connection_id.to_string(),
                    connection_name,
                    upstream_name: input.upstream_name,
                    public_name: input.public_name,
                    title: input.title,
                    description: input.description,
                    input_schema: input.input_schema,
                    output_schema: input.output_schema,
                    annotations: input.annotations,
                    schema_hash: input.schema_hash,
                    side_effect: SideEffectClass::ApiMutation,
                    catalog_status: McpGatewayCatalogStatus::Active,
                    assigned_user_ids: assigned,
                    created_at,
                    updated_at: timestamp,
                },
            );
        }
        let count = state
            .tools
            .values()
            .filter(|tool| tool.connection_id == connection_id.to_string())
            .count() as u32;
        let (connection, _) = state
            .connections
            .get_mut(&(workspace_id.to_string(), connection_id))
            .expect("checked");
        connection.last_sync_status = McpGatewaySyncStatus::Succeeded;
        connection.last_sync_error = None;
        connection.last_synced_at = Some(now());
        connection.tool_count = count;
        connection.updated_at = now();
        Ok(connection.clone())
    }

    async fn list_tools(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<McpGatewayTool>, McpGatewayStoreError> {
        let state = self.inner.read().await;
        let mut values = state
            .tools
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, tool)| {
                let mut tool = tool.clone();
                tool.assigned_user_ids = state
                    .assignments
                    .get(&(
                        workspace_id.to_string(),
                        Uuid::parse_str(&tool.id).unwrap_or_default(),
                    ))
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect();
                tool
            })
            .collect::<Vec<_>>();
        values.sort_by(|a, b| a.public_name.cmp(&b.public_name));
        Ok(values)
    }
    async fn update_tool_side_effect(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        side_effect: SideEffectClass,
    ) -> Result<McpGatewayTool, McpGatewayStoreError> {
        let mut state = self.inner.write().await;
        let tool = state
            .tools
            .get_mut(&(workspace_id.to_string(), tool_id))
            .ok_or(McpGatewayStoreError::NotFound)?;
        tool.side_effect = side_effect;
        tool.updated_at = now();
        Ok(tool.clone())
    }
    async fn mark_tool_schema_changed(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
    ) -> Result<(), McpGatewayStoreError> {
        let mut state = self.inner.write().await;
        let tool = state
            .tools
            .get_mut(&(workspace_id.to_string(), tool_id))
            .ok_or(McpGatewayStoreError::NotFound)?;
        tool.catalog_status = McpGatewayCatalogStatus::SchemaChanged;
        Ok(())
    }
    async fn resolve_entitled_tool(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        public_name: &str,
    ) -> Result<EntitledMcpTool, McpGatewayStoreError> {
        let state = self.inner.read().await;
        let tool = state
            .tools
            .iter()
            .find(|((workspace, id), tool)| {
                workspace == workspace_id
                    && tool.public_name == public_name
                    && tool.catalog_status == McpGatewayCatalogStatus::Active
                    && state
                        .assignments
                        .get(&(workspace.clone(), *id))
                        .is_some_and(|users| users.contains(&user_id))
            })
            .map(|(_, tool)| tool.clone())
            .ok_or(McpGatewayStoreError::NotFound)?;
        let (connection, secret) = state
            .connections
            .get(&(
                workspace_id.to_string(),
                Uuid::parse_str(&tool.connection_id)
                    .map_err(|_| McpGatewayStoreError::Internal("invalid connection id".into()))?,
            ))
            .filter(|(connection, _)| connection.enabled)
            .ok_or(McpGatewayStoreError::NotFound)?;
        let mut tool = tool;
        tool.assigned_user_ids = vec![user_id.to_string()];
        Ok(EntitledMcpTool {
            tool,
            endpoint_url: connection.endpoint_url.clone(),
            auth_kind: connection.auth_kind,
            encrypted_credential: secret.clone(),
            connection_updated_at: DateTime::parse_from_rfc3339(&connection.updated_at)
                .map_err(|error| McpGatewayStoreError::Internal(error.to_string()))?
                .with_timezone(&Utc),
        })
    }
    async fn list_entitled_tools(
        &self,
        workspace_id: &str,
        user_id: Uuid,
        after_public_name: Option<&str>,
        limit: u32,
    ) -> Result<Vec<EntitledMcpTool>, McpGatewayStoreError> {
        let state = self.inner.read().await;
        let mut names = state
            .tools
            .values()
            .filter(|tool| {
                after_public_name
                    .map(|cursor| tool.public_name.as_str() > cursor)
                    .unwrap_or(true)
            })
            .map(|tool| tool.public_name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        drop(state);
        let mut out = Vec::new();
        for name in names {
            if let Ok(tool) = self
                .resolve_entitled_tool(workspace_id, user_id, &name)
                .await
            {
                out.push(tool);
                if out.len() >= limit as usize {
                    break;
                }
            }
        }
        Ok(out)
    }
    async fn replace_assignments(
        &self,
        workspace_id: &str,
        tool_id: Uuid,
        user_ids: Vec<Uuid>,
        _created_by: Option<Uuid>,
    ) -> Result<Vec<Uuid>, McpGatewayStoreError> {
        let unique = user_ids.into_iter().collect::<BTreeSet<_>>();
        if unique.len() > 500 {
            return Err(McpGatewayStoreError::Conflict(
                "at most 500 assignments are allowed".into(),
            ));
        }
        let mut state = self.inner.write().await;
        if !state
            .tools
            .contains_key(&(workspace_id.to_string(), tool_id))
        {
            return Err(McpGatewayStoreError::NotFound);
        }
        state
            .assignments
            .insert((workspace_id.to_string(), tool_id), unique.clone());
        let output = unique.into_iter().collect::<Vec<_>>();
        if let Some(tool) = state.tools.get_mut(&(workspace_id.to_string(), tool_id)) {
            tool.assigned_user_ids = output.iter().map(ToString::to_string).collect();
        }
        Ok(output)
    }
}
