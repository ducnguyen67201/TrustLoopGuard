use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ErrorData as McpError, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use serde::{Deserialize, Serialize};
use tl_core::{
    Action, ApprovalStatus, AuthorizationClaim, AuthorizationEffect,
    CompleteAuthorizationLeaseRequest, EventKind, GuardEvent, LeaseStatus, Principal, ToolIdentity,
};
use uuid::Uuid;

use crate::auth::McpAccessContext;
use crate::AppState;

use super::service::require_signed_member_feature;
use super::upstream::{prepare_upstream, schema_hash};
use super::{EntitledMcpTool, McpGatewayStore, McpGatewayStoreError};

const PAGE_SIZE: u32 = 100;
const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
const MAX_RESULT_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct HostedMcpHandler {
    app: AppState,
    store: Arc<dyn McpGatewayStore>,
    seal_key: [u8; 32],
}

impl HostedMcpHandler {
    pub fn new(app: AppState, store: Arc<dyn McpGatewayStore>, seal_key: [u8; 32]) -> Self {
        Self {
            app,
            store,
            seal_key,
        }
    }

    async fn access(
        &self,
        request: &RequestContext<RoleServer>,
    ) -> Result<McpAccessContext, McpError> {
        let context = request
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<McpAccessContext>())
            .cloned()
            .ok_or_else(|| McpError::invalid_request("MCP identity is required", None))?;
        require_signed_member_feature(&self.app, &context)
            .await
            .map_err(|_| McpError::invalid_request("MCP workspace access is unavailable", None))?;
        Ok(context)
    }

    async fn default_environment(&self, workspace_id: &str) -> Result<String, McpError> {
        self.app.environment_store.default_environment_id(workspace_id).await.map_err(|error| { tracing::error!(workspace_id, error = %error, "MCP default environment lookup failed"); McpError::internal_error("MCP runtime is unavailable", None) })
    }
}

impl ServerHandler for HostedMcpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Tools are assigned by your workspace and every call is evaluated by runtime policy.",
        )
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let access = self.access(&context).await?;
        let cursor = request
            .and_then(|request| request.cursor)
            .map(decode_cursor)
            .transpose()?;
        let mut rows = self
            .store
            .list_entitled_tools(
                &access.workspace_id,
                access.user_id,
                cursor.as_deref(),
                PAGE_SIZE + 1,
            )
            .await
            .map_err(store_mcp_error)?;
        let next_cursor = if rows.len() > PAGE_SIZE as usize {
            let last = rows[PAGE_SIZE as usize - 1].tool.public_name.clone();
            rows.truncate(PAGE_SIZE as usize);
            Some(encode_cursor(&last))
        } else {
            None
        };
        let tools = rows
            .into_iter()
            .map(|row| wire_tool(row.tool))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ListToolsResult {
            meta: None,
            next_cursor,
            tools,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let access = self.access(&context).await?;
        Ok(self
            .call_governed(access, request, &context)
            .await
            .unwrap_or_else(tool_error))
    }
}

impl HostedMcpHandler {
    async fn call_governed(
        &self,
        access: McpAccessContext,
        request: CallToolRequestParams,
        request_context: &RequestContext<RoleServer>,
    ) -> Result<CallToolResult, String> {
        let environment_id = self
            .default_environment(&access.workspace_id)
            .await
            .map_err(|_| "MCP runtime is unavailable".to_string())?;
        let entitled = self
            .store
            .resolve_entitled_tool(&access.workspace_id, access.user_id, &request.name)
            .await
            .map_err(|_| "This tool is not assigned to you".to_string())?;
        let arguments = request.arguments.clone().unwrap_or_default();
        validate_arguments(&entitled.tool.input_schema, &arguments)?;
        let bearer = decrypt_bearer(&entitled, &self.seal_key)?;
        let prepared = prepare_upstream(&entitled.endpoint_url, bearer.as_deref())
            .await
            .map_err(|_| "The upstream tool server is unavailable".to_string())?;
        let live_tools = prepared
            .list_tools()
            .await
            .map_err(|_| "The upstream tool server is unavailable".to_string())?;
        let live_tool = live_tools
            .into_iter()
            .find(|tool| tool.name == entitled.tool.upstream_name)
            .ok_or_else(|| "The tool is no longer available upstream".to_string())?;
        if schema_hash(&serde_json::Value::Object(
            (*live_tool.input_schema).clone(),
        )) != entitled.tool.schema_hash
        {
            prepared.close().await;
            let tool_id = Uuid::parse_str(&entitled.tool.id)
                .map_err(|_| "The tool catalog is invalid".to_string())?;
            let _ = self
                .store
                .mark_tool_schema_changed(&access.workspace_id, tool_id)
                .await;
            return Err(
                "The tool schema changed and requires an administrator to synchronize it".into(),
            );
        }
        let event = build_event(&access, &environment_id, &entitled, &arguments, None);
        let principal_id = format!("mcp:user:{}", access.user_id);
        let mut decision = crate::services::event_service::execute_event_submission_as_principal(
            &self.app,
            &access.workspace_id,
            &environment_id,
            event.clone(),
            Instant::now(),
            &principal_id,
        )
        .await
        .map_err(|_| "Runtime policy evaluation failed".to_string())?;

        if decision.decision.effect == AuthorizationEffect::RequireApproval {
            prepared.close().await;
            let approval = decision
                .decision
                .approval
                .clone()
                .ok_or_else(|| "Approval is required but could not be created".to_string())?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
            let grant_id = loop {
                tokio::select! {
                    _ = request_context.ct.cancelled() => return Err("Tool call was canceled while waiting for approval".into()),
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err("Approval timed out before the tool was called".into());
                }
                let current = self
                    .app
                    .authorization_store
                    .get_approval(&access.workspace_id, &environment_id, &approval.id)
                    .await
                    .map_err(|_| "Approval state could not be read".to_string())?;
                match current.status {
                    ApprovalStatus::Approved => {
                        break current
                            .grant_id
                            .ok_or_else(|| "Approved request has no grant".to_string())?
                    }
                    ApprovalStatus::Pending => continue,
                    ApprovalStatus::Denied | ApprovalStatus::Canceled | ApprovalStatus::Expired => {
                        return Err("The tool call was not approved".into())
                    }
                }
            };
            let attempt_id = Uuid::new_v4().to_string();
            let resumed = build_event(
                &access,
                &environment_id,
                &entitled,
                &arguments,
                Some(AuthorizationClaim {
                    grant_id,
                    attempt_id,
                }),
            );
            decision = crate::services::event_service::execute_event_submission_as_principal(
                &self.app,
                &access.workspace_id,
                &environment_id,
                resumed,
                Instant::now(),
                &principal_id,
            )
            .await
            .map_err(|_| "Runtime policy evaluation failed".to_string())?;
            if decision.decision.effect != AuthorizationEffect::Permit
                || decision.decision.lease.is_none()
            {
                return Err("The approved tool call was not permitted at execution time".into());
            }
            return self
                .execute_permitted(
                    &access,
                    &entitled,
                    arguments,
                    decision
                        .decision
                        .lease
                        .as_ref()
                        .map(|lease| lease.id.as_str()),
                )
                .await;
        }
        if decision.decision.effect != AuthorizationEffect::Permit {
            prepared.close().await;
            return Err(match decision.decision.effect { AuthorizationEffect::Deny => "Runtime policy blocked this tool call", AuthorizationEffect::Transform => "Runtime policy requested a transformation; transformed MCP calls are not executable", AuthorizationEffect::Defer => "Runtime policy deferred this tool call", AuthorizationEffect::RequireApproval => "Approval is required", AuthorizationEffect::Permit => unreachable!() }.into());
        }
        self.execute_with_prepared(
            &access,
            &entitled,
            arguments,
            prepared,
            live_tool,
            decision
                .decision
                .lease
                .as_ref()
                .map(|lease| lease.id.as_str()),
        )
        .await
    }

    async fn execute_permitted(
        &self,
        access: &McpAccessContext,
        original: &EntitledMcpTool,
        arguments: serde_json::Map<String, serde_json::Value>,
        lease_id: Option<&str>,
    ) -> Result<CallToolResult, String> {
        let current = self
            .store
            .resolve_entitled_tool(
                &access.workspace_id,
                access.user_id,
                &original.tool.public_name,
            )
            .await
            .map_err(|_| "Tool access was revoked before execution".to_string())?;
        require_same_authority(original, &current)?;
        let bearer = decrypt_bearer(&current, &self.seal_key)?;
        let prepared = prepare_upstream(&current.endpoint_url, bearer.as_deref())
            .await
            .map_err(|_| "The upstream tool server is unavailable".to_string())?;
        let live = prepared
            .list_tools()
            .await
            .map_err(|_| "The upstream tool server is unavailable".to_string())?
            .into_iter()
            .find(|tool| tool.name == current.tool.upstream_name)
            .ok_or_else(|| "The tool is no longer available upstream".to_string())?;
        if schema_hash(&serde_json::Value::Object((*live.input_schema).clone()))
            != current.tool.schema_hash
        {
            prepared.close().await;
            let _ = self
                .store
                .mark_tool_schema_changed(
                    &access.workspace_id,
                    Uuid::parse_str(&current.tool.id).unwrap_or_default(),
                )
                .await;
            return Err("The tool schema changed before execution".into());
        }
        self.execute_with_prepared(access, &current, arguments, prepared, live, lease_id)
            .await
    }

    async fn execute_with_prepared(
        &self,
        access: &McpAccessContext,
        entitled: &EntitledMcpTool,
        arguments: serde_json::Map<String, serde_json::Value>,
        prepared: super::upstream::PreparedUpstream,
        _live: Tool,
        lease_id: Option<&str>,
    ) -> Result<CallToolResult, String> {
        let current = self
            .store
            .resolve_entitled_tool(
                &access.workspace_id,
                access.user_id,
                &entitled.tool.public_name,
            )
            .await
            .map_err(|_| "Tool access was revoked before execution".to_string())?;
        require_same_authority(entitled, &current)?;
        let call = CallToolRequestParams::new(entitled.tool.upstream_name.clone())
            .with_arguments(arguments);
        let result = prepared.call_tool(call).await;
        prepared.close().await;
        let mut result = match result {
            Ok(value) => value,
            Err(_) => {
                complete_lease(
                    &self.app,
                    &access.workspace_id,
                    lease_id,
                    LeaseStatus::Canceled,
                    serde_json::json!({"success": false, "ambiguous": true}),
                )
                .await?;
                return Err("The upstream tool call failed; its outcome may be ambiguous".into());
            }
        };
        if let (Some(schema), Some(structured)) = (
            entitled.tool.output_schema.as_ref(),
            result.structured_content.as_ref(),
        ) {
            let validator = jsonschema::JSONSchema::compile(schema)
                .map_err(|_| "The pinned output schema is invalid".to_string())?;
            if !validator.is_valid(structured) {
                result = tool_error(
                    "The upstream result did not match the approved output schema".into(),
                );
            }
        }
        complete_lease(
            &self.app,
            &access.workspace_id,
            lease_id,
            LeaseStatus::Consumed,
            serde_json::json!({"success": result.is_error != Some(true)}),
        )
        .await?;
        if serde_json::to_vec(&result)
            .map_err(|_| "The upstream result could not be serialized".to_string())?
            .len()
            > MAX_RESULT_BYTES
        {
            return Err("The upstream tool result exceeded the 1 MiB response limit".into());
        }
        Ok(result)
    }
}

fn build_event(
    access: &McpAccessContext,
    environment_id: &str,
    entitled: &EntitledMcpTool,
    arguments: &serde_json::Map<String, serde_json::Value>,
    authorization: Option<AuthorizationClaim>,
) -> GuardEvent {
    GuardEvent {
        kind: EventKind::ToolCallProposed,
        principal: Principal {
            workspace_id: access.workspace_id.clone(),
            environment_id: environment_id.to_string(),
            agent_id: "hosted-mcp".into(),
            user_id: Some(access.user_id.to_string()),
            session_id: Some(access.client_id.clone()),
            task_id: None,
            run_id: None,
            run_event_id: None,
        },
        action: Action {
            operation: format!(
                "mcp:{}:{}",
                urlencoding::encode(&entitled.tool.connection_id),
                urlencoding::encode(&entitled.tool.upstream_name)
            ),
            parameters: serde_json::Value::Object(arguments.clone()),
            side_effect: Some(entitled.tool.side_effect),
            invocation_id: Some(Uuid::new_v4().to_string()),
            tool_identity: Some(ToolIdentity {
                server_id: entitled.tool.connection_id.clone(),
                tool_name: entitled.tool.upstream_name.clone(),
                schema_hash: entitled.tool.schema_hash.clone(),
            }),
            authorization,
        },
        sources: Vec::new(),
        provenance: Default::default(),
        resolution: None,
        label_resolution: None,
        checks: Vec::new(),
        signals: Vec::new(),
        context: serde_json::json!({"collector":"hosted_mcp_gateway","oauth_client_id":access.client_id,"connection_id":entitled.tool.connection_id,"public_tool_name":entitled.tool.public_name}),
    }
}

fn validate_arguments(
    schema: &serde_json::Value,
    arguments: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let value = serde_json::Value::Object(arguments.clone());
    if serde_json::to_vec(&value)
        .map_err(|_| "Tool arguments are invalid".to_string())?
        .len()
        > MAX_ARGUMENT_BYTES
    {
        return Err("Tool arguments exceed 64 KiB".into());
    }
    let validator = jsonschema::JSONSchema::compile(schema)
        .map_err(|_| "The pinned input schema is invalid".to_string())?;
    if !validator.is_valid(&value) {
        return Err("Tool arguments do not match the assigned schema".into());
    }
    Ok(())
}
fn decrypt_bearer(entitled: &EntitledMcpTool, key: &[u8; 32]) -> Result<Option<String>, String> {
    match entitled.encrypted_credential.as_deref() {
        Some(value) => crate::gateway::unseal_provider_key(value, key)
            .map(Some)
            .map_err(|_| "The upstream credential is unavailable".into()),
        None => Ok(None),
    }
}
fn require_same_authority(
    original: &EntitledMcpTool,
    current: &EntitledMcpTool,
) -> Result<(), String> {
    if original.tool.id != current.tool.id
        || original.tool.schema_hash != current.tool.schema_hash
        || original.tool.upstream_name != current.tool.upstream_name
        || original.endpoint_url != current.endpoint_url
        || original.connection_updated_at != current.connection_updated_at
    {
        Err("Tool access changed before execution".into())
    } else {
        Ok(())
    }
}
async fn complete_lease(
    app: &AppState,
    workspace_id: &str,
    lease_id: Option<&str>,
    status: LeaseStatus,
    outcome: serde_json::Value,
) -> Result<(), String> {
    let Some(lease_id) = lease_id else {
        return Ok(());
    };
    let environment_id = app
        .environment_store
        .default_environment_id(workspace_id)
        .await
        .map_err(|_| "Authorization lease environment could not be resolved".to_string())?;
    app.authorization_store.complete_lease(workspace_id, &environment_id, lease_id, CompleteAuthorizationLeaseRequest { status, outcome }).await.map(|_| ()).map_err(|error| { tracing::error!(workspace_id, lease_id, error = %error, "MCP authorization lease completion failed"); "The tool ran, but execution reconciliation failed; do not retry automatically".into() })
}

fn wire_tool(tool: tl_core::McpGatewayTool) -> Result<Tool, McpError> {
    let input = tool
        .input_schema
        .as_object()
        .cloned()
        .ok_or_else(|| McpError::internal_error("Stored MCP tool schema is invalid", None))?;
    let output = tool
        .output_schema
        .and_then(|value| value.as_object().cloned())
        .map(Arc::new);
    let annotations = serde_json::from_value::<ToolAnnotations>(tool.annotations).ok();
    let mut value = Tool::new_with_raw(
        Cow::Owned(tool.public_name),
        tool.description.map(Cow::Owned),
        Arc::new(input),
    );
    if let Some(title) = tool.title {
        value = value.with_title(title);
    }
    if let Some(output) = output {
        value = value.with_raw_output_schema(output);
    }
    if let Some(annotations) = annotations {
        value = value.with_annotations(annotations);
    }
    Ok(value)
}
fn store_mcp_error(error: McpGatewayStoreError) -> McpError {
    tracing::error!(error = %error, "MCP entitlement listing failed");
    McpError::internal_error("MCP tool catalog is unavailable", None)
}
fn tool_error(message: String) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

#[derive(Serialize, Deserialize)]
struct CursorV1 {
    v: u8,
    after: String,
}
fn encode_cursor(after: &str) -> String {
    URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&CursorV1 {
            v: 1,
            after: after.to_string(),
        })
        .expect("cursor serializes"),
    )
}
fn decode_cursor(value: String) -> Result<String, McpError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| McpError::invalid_params("Invalid tools/list cursor", None))?;
    let cursor: CursorV1 = serde_json::from_slice(&bytes)
        .map_err(|_| McpError::invalid_params("Invalid tools/list cursor", None))?;
    if cursor.v != 1 || cursor.after.is_empty() {
        return Err(McpError::invalid_params("Invalid tools/list cursor", None));
    }
    Ok(cursor.after)
}
