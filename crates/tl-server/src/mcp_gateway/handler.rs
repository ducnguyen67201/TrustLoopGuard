use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ErrorData as McpError, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use tl_core::{
    Action, ApprovalStatus, AuthorizationClaim, AuthorizationEffect, AuthorizationFinding,
    CompleteAuthorizationLeaseRequest, EventKind, GuardEvent, LeaseStatus, Principal, RunStatus,
    Severity, SideEffectClass, ToolIdentity,
};
use uuid::Uuid;

use crate::auth::McpAccessContext;
use crate::services::event_service::{
    execute_event_submission_as_principal, execute_event_submission_with_context,
    EventSubmissionContext, EventSubmissionResult,
};
use crate::AppState;

use super::governance::{
    extract_result_policy_text, governed_input_schema, managed_description,
    split_governance_arguments, GovernanceContext, GovernedArguments, GovernedResult,
};
use super::runs::{create_hosted_mcp_run, finish_hosted_mcp_run, HostedMcpRun};
use super::service::require_signed_member_feature;
use super::upstream::{prepare_catalog_upstream, prepare_tool_upstream, schema_hash};
use super::{EntitledMcpTool, McpGatewayStore, McpGatewayStoreError};

const PAGE_SIZE: u32 = 100;
const APPROVAL_WAIT_SECONDS: u64 = 60;

#[derive(Clone)]
pub struct HostedMcpHandler {
    app: AppState,
    store: Arc<dyn McpGatewayStore>,
    seal_key: [u8; 32],
}

#[derive(Debug)]
struct CallFailure {
    message: String,
    run_status: RunStatus,
}

impl CallFailure {
    fn expected(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            run_status: RunStatus::Completed,
        }
    }

    fn failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            run_status: RunStatus::Failed,
        }
    }

    fn canceled(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            run_status: RunStatus::Canceled,
        }
    }
}

struct UpstreamFailure {
    failure: CallFailure,
    ambiguous: bool,
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
        self.app
            .environment_store
            .default_environment_id(workspace_id)
            .await
            .map_err(|error| {
                tracing::error!(workspace_id, error = %error, "MCP default environment lookup failed");
                McpError::internal_error("MCP runtime is unavailable", None)
            })
    }
}

impl ServerHandler for HostedMcpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Tools are assigned to your signed user-and-agent identity. Every call must include the required __trustloop governance context and is evaluated before execution and again before result disclosure.",
        )
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let access = self.access(&context).await?;
        let cursor = request.and_then(|request| request.cursor);
        if cursor.as_deref() == Some("") {
            return Err(McpError::invalid_params("Invalid tools/list cursor", None));
        }
        let mut rows = self
            .store
            .list_entitled_tools(
                &access.workspace_id,
                access.user_id,
                &access.agent_id,
                cursor.as_deref(),
                PAGE_SIZE + 1,
            )
            .await
            .map_err(store_mcp_error)?;
        let next_cursor = if rows.len() > PAGE_SIZE as usize {
            let last = rows[PAGE_SIZE as usize - 1].tool.public_name.clone();
            rows.truncate(PAGE_SIZE as usize);
            Some(last)
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
        let invocation_id = Uuid::now_v7().to_string();
        let run = create_hosted_mcp_run(
            &self.app,
            &access.workspace_id,
            &environment_id,
            &access.agent_id,
            &access.client_id,
            &invocation_id,
            &bounded_label(&request.name),
        )
        .await;
        let result = self
            .call_governed_inner(
                &access,
                &environment_id,
                &run,
                &invocation_id,
                request,
                request_context,
            )
            .await;
        let status = result
            .as_ref()
            .map(|_| RunStatus::Completed)
            .unwrap_or_else(|failure| failure.run_status);
        finish_hosted_mcp_run(
            &self.app,
            &access.workspace_id,
            &environment_id,
            run.run_id.as_deref(),
            status,
        )
        .await;
        result.map_err(|failure| failure.message)
    }

    async fn call_governed_inner(
        &self,
        access: &McpAccessContext,
        environment_id: &str,
        run: &HostedMcpRun,
        invocation_id: &str,
        request: CallToolRequestParams,
        request_context: &RequestContext<RoleServer>,
    ) -> Result<CallToolResult, CallFailure> {
        let entitled = match self
            .store
            .resolve_entitled_tool(
                &access.workspace_id,
                access.user_id,
                &access.agent_id,
                &request.name,
            )
            .await
        {
            Ok(entitled) => entitled,
            Err(McpGatewayStoreError::NotFound) => {
                let event = build_unresolved_event(
                    access,
                    environment_id,
                    run,
                    invocation_id,
                    &request.name,
                );
                self.record_forced_finding(
                    access,
                    environment_id,
                    event,
                    "mcp_entitlement",
                    AuthorizationEffect::Deny,
                    "the signed user and agent are not assigned this hosted MCP tool",
                )
                .await?;
                return Err(CallFailure::expected("This tool is not assigned to you"));
            }
            Err(error) => {
                tracing::error!(
                    workspace_id = access.workspace_id,
                    agent_id = access.agent_id,
                    error = %error,
                    "MCP entitlement resolution failed"
                );
                return Err(CallFailure::failed("MCP runtime is unavailable"));
            }
        };

        let arguments = request.arguments.unwrap_or_default();
        let governed = match split_governance_arguments(&entitled.tool.input_schema, &arguments) {
            Ok(governed) => governed,
            Err(reason) => {
                let event = build_managed_failure_event(
                    access,
                    environment_id,
                    run,
                    invocation_id,
                    &entitled,
                    "invalid_governance_context",
                    SideEffectClass::Read,
                );
                self.record_forced_finding(
                    access,
                    environment_id,
                    event,
                    "mcp_governance_context",
                    AuthorizationEffect::Deny,
                    &reason,
                )
                .await?;
                return Err(CallFailure::expected(reason));
            }
        };

        self.verify_live_catalog(access, &entitled).await?;
        let preflight_event = build_preflight_event(
            access,
            environment_id,
            run,
            invocation_id,
            &entitled,
            &governed,
            None,
        );
        let preflight = self
            .evaluate_checkpoint(access, environment_id, preflight_event, request_context)
            .await?;
        let preflight_lease = preflight
            .authorization
            .lease
            .as_ref()
            .map(|lease| lease.id.clone());
        match preflight.authorization.effect {
            AuthorizationEffect::Permit => {}
            AuthorizationEffect::Transform => {
                complete_lease(
                    &self.app,
                    &access.workspace_id,
                    environment_id,
                    preflight_lease.as_deref(),
                    LeaseStatus::Canceled,
                    serde_json::json!({
                        "upstream_success": false,
                        "result_released": false,
                        "transformed": false,
                        "ambiguous": false,
                    }),
                )
                .await?;
                return Err(CallFailure::expected(
                    "Runtime policy requested a transformation; transformed MCP calls are not executable",
                ));
            }
            AuthorizationEffect::Deny => {
                return Err(CallFailure::expected(
                    "Runtime policy blocked this tool call",
                ));
            }
            AuthorizationEffect::Defer => {
                return Err(CallFailure::expected(
                    "Runtime policy deferred this tool call",
                ));
            }
            AuthorizationEffect::RequireApproval => {
                return Err(CallFailure::expected("Approval is required"));
            }
        }

        let upstream_result = match self
            .execute_upstream(access, &entitled, governed.upstream.clone())
            .await
        {
            Ok(result) => result,
            Err(error) => {
                complete_lease(
                    &self.app,
                    &access.workspace_id,
                    environment_id,
                    preflight_lease.as_deref(),
                    LeaseStatus::Canceled,
                    serde_json::json!({
                        "upstream_success": false,
                        "result_released": false,
                        "transformed": false,
                        "ambiguous": error.ambiguous,
                    }),
                )
                .await?;
                return Err(error.failure);
            }
        };
        let withheld_outcome = serde_json::json!({
            "upstream_success": upstream_result.is_error != Some(true),
            "upstream_returned": true,
            "result_released": false,
            "transformed": false,
            "ambiguous": false,
        });

        let governed_result = match extract_result_policy_text(
            &upstream_result,
            entitled.tool.output_schema.as_ref(),
        ) {
            Ok(result) => result,
            Err(reason) => {
                let event = build_result_validation_event(
                    access,
                    environment_id,
                    run,
                    &entitled,
                    &governed.context,
                );
                let audit = self
                    .record_forced_finding(
                        access,
                        environment_id,
                        event,
                        "mcp_result_validation",
                        AuthorizationEffect::Defer,
                        &reason,
                    )
                    .await;
                let reconciliation = complete_lease(
                    &self.app,
                    &access.workspace_id,
                    environment_id,
                    preflight_lease.as_deref(),
                    LeaseStatus::Consumed,
                    withheld_outcome.clone(),
                )
                .await;
                if audit.is_err() || reconciliation.is_err() {
                    return Err(CallFailure::failed(
                        "The upstream tool ran, but its result was withheld because policy auditing failed; do not retry automatically",
                    ));
                }
                return Err(CallFailure::expected(format!(
                    "The upstream tool ran, but its result was withheld: {reason}; do not retry automatically"
                )));
            }
        };

        let disclosure_event = build_disclosure_event(
            access,
            environment_id,
            run,
            &entitled,
            &governed.context,
            &governed_result,
            None,
        );
        let disclosure = match self
            .evaluate_checkpoint(access, environment_id, disclosure_event, request_context)
            .await
        {
            Ok(disclosure) => disclosure,
            Err(failure) => {
                let _ = complete_lease(
                    &self.app,
                    &access.workspace_id,
                    environment_id,
                    preflight_lease.as_deref(),
                    LeaseStatus::Consumed,
                    withheld_outcome,
                )
                .await;
                return Err(CallFailure {
                    message: format!(
                        "The upstream tool ran, but its result was withheld: {}; do not retry automatically",
                        failure.message
                    ),
                    run_status: failure.run_status,
                });
            }
        };
        let disclosure_lease = disclosure
            .authorization
            .lease
            .as_ref()
            .map(|lease| lease.id.clone());
        let upstream_success = upstream_result.is_error != Some(true);

        let (released_result, transformed) = match disclosure.authorization.effect {
            AuthorizationEffect::Permit => (Some(upstream_result), false),
            AuthorizationEffect::Transform
                if governed_result.text_only && entitled.tool.output_schema.is_none() =>
            {
                match disclosure.decision.safe_output.as_deref() {
                    Some(safe_output) => (
                        Some(CallToolResult::success(vec![ContentBlock::text(
                            safe_output,
                        )])),
                        true,
                    ),
                    None => (None, false),
                }
            }
            AuthorizationEffect::Transform
            | AuthorizationEffect::Deny
            | AuthorizationEffect::Defer
            | AuthorizationEffect::RequireApproval => (None, false),
        };
        let result_released = released_result.is_some();
        let disclosure_lease_status = if result_released {
            LeaseStatus::Consumed
        } else {
            LeaseStatus::Canceled
        };
        complete_lease(
            &self.app,
            &access.workspace_id,
            environment_id,
            disclosure_lease.as_deref(),
            disclosure_lease_status,
            serde_json::json!({
                "upstream_success": upstream_success,
                "result_released": result_released,
                "transformed": transformed,
                "ambiguous": false,
            }),
        )
        .await
        .map_err(|_| {
            CallFailure::failed(
                "The upstream tool ran, but its result was withheld because authorization reconciliation failed; do not retry automatically",
            )
        })?;
        complete_lease(
            &self.app,
            &access.workspace_id,
            environment_id,
            preflight_lease.as_deref(),
            LeaseStatus::Consumed,
            serde_json::json!({
                "upstream_success": upstream_success,
                "upstream_returned": true,
                "result_released": result_released,
                "transformed": transformed,
                "ambiguous": false,
            }),
        )
        .await
        .map_err(|_| {
            CallFailure::failed(
                "The upstream tool ran, but its result was withheld because execution reconciliation failed; do not retry automatically",
            )
        })?;

        released_result.ok_or_else(|| {
            let reason = match disclosure.authorization.effect {
                AuthorizationEffect::Deny => "runtime policy blocked disclosure",
                AuthorizationEffect::Defer => "runtime policy deferred disclosure",
                AuthorizationEffect::Transform => {
                    "the requested result transformation could not be safely applied"
                }
                AuthorizationEffect::RequireApproval => "approval was not completed",
                AuthorizationEffect::Permit => "result release failed",
            };
            CallFailure::expected(format!(
                "The upstream tool ran, but its result was withheld because {reason}; do not retry automatically"
            ))
        })
    }

    async fn evaluate_checkpoint(
        &self,
        access: &McpAccessContext,
        environment_id: &str,
        event: GuardEvent,
        request_context: &RequestContext<RoleServer>,
    ) -> Result<EventSubmissionResult, CallFailure> {
        let mut result = execute_event_submission_as_principal(
            &self.app,
            &access.workspace_id,
            environment_id,
            event.clone(),
            Instant::now(),
            &access.agent_id,
        )
        .await
        .map_err(|_| CallFailure::failed("Runtime policy evaluation failed"))?;
        if result.authorization.effect != AuthorizationEffect::RequireApproval {
            return Ok(result);
        }

        let approval =
            result.authorization.approval.clone().ok_or_else(|| {
                CallFailure::failed("Approval is required but could not be created")
            })?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(APPROVAL_WAIT_SECONDS);
        let grant_id = loop {
            tokio::select! {
                _ = request_context.ct.cancelled() => {
                    return Err(CallFailure::canceled("Tool call was canceled while waiting for approval"));
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CallFailure::expected(
                    "Approval timed out before the tool call could continue",
                ));
            }
            let current = self
                .app
                .authorization_store
                .get_approval(&access.workspace_id, environment_id, &approval.id)
                .await
                .map_err(|_| CallFailure::failed("Approval state could not be read"))?;
            match current.status {
                ApprovalStatus::Approved => {
                    break current
                        .grant_id
                        .ok_or_else(|| CallFailure::failed("Approved request has no grant"))?;
                }
                ApprovalStatus::Pending => continue,
                ApprovalStatus::Denied | ApprovalStatus::Canceled | ApprovalStatus::Expired => {
                    return Err(CallFailure::expected("The tool call was not approved"));
                }
            }
        };
        let resumed = resume_authorized_event(
            event,
            AuthorizationClaim {
                grant_id,
                attempt_id: Uuid::new_v4().to_string(),
            },
        );
        result = execute_event_submission_as_principal(
            &self.app,
            &access.workspace_id,
            environment_id,
            resumed,
            Instant::now(),
            &access.agent_id,
        )
        .await
        .map_err(|_| CallFailure::failed("Runtime policy evaluation failed"))?;
        if !matches!(
            result.authorization.effect,
            AuthorizationEffect::Permit | AuthorizationEffect::Transform
        ) || result.authorization.lease.is_none()
        {
            return Err(CallFailure::expected(
                "The approved tool call was not authorized at execution time",
            ));
        }
        Ok(result)
    }

    async fn record_forced_finding(
        &self,
        access: &McpAccessContext,
        environment_id: &str,
        event: GuardEvent,
        source: &str,
        effect: AuthorizationEffect,
        reason: &str,
    ) -> Result<EventSubmissionResult, CallFailure> {
        let severity = match effect {
            AuthorizationEffect::Deny => Severity::High,
            _ => Severity::Medium,
        };
        execute_event_submission_with_context(
            &self.app,
            &access.workspace_id,
            environment_id,
            event,
            Instant::now(),
            EventSubmissionContext {
                authorization_principal_id: Some(access.agent_id.clone()),
                additional_findings: vec![AuthorizationFinding {
                    id: format!("gateway:{}", Uuid::now_v7()),
                    source: source.to_string(),
                    effect,
                    reason: reason.to_string(),
                    severity,
                    policy_id: None,
                    requirement_id: None,
                    remediation: None,
                    evidence: serde_json::Value::Null,
                }],
            },
        )
        .await
        .map_err(|_| CallFailure::failed("Runtime policy auditing failed"))
    }

    async fn verify_live_catalog(
        &self,
        access: &McpAccessContext,
        entitled: &EntitledMcpTool,
    ) -> Result<(), CallFailure> {
        let bearer = decrypt_bearer(entitled, &self.seal_key).map_err(CallFailure::failed)?;
        let prepared = prepare_catalog_upstream(&entitled.endpoint_url, bearer.as_deref())
            .await
            .map_err(|_| CallFailure::failed("The upstream tool server is unavailable"))?;
        let live_tools = prepared.list_tools().await;
        prepared.close().await;
        let live_tool = live_tools
            .map_err(|_| CallFailure::failed("The upstream tool server is unavailable"))?
            .into_iter()
            .find(|tool| tool.name == entitled.tool.upstream_name)
            .ok_or_else(|| CallFailure::expected("The tool is no longer available upstream"))?;
        if schema_hash(&serde_json::Value::Object(
            (*live_tool.input_schema).clone(),
        )) != entitled.tool.schema_hash
        {
            let tool_id = Uuid::parse_str(&entitled.tool.id)
                .map_err(|_| CallFailure::failed("The tool catalog is invalid"))?;
            let _ = self
                .store
                .mark_tool_schema_changed(&access.workspace_id, tool_id)
                .await;
            return Err(CallFailure::expected(
                "The tool schema changed and requires an administrator to synchronize it",
            ));
        }
        Ok(())
    }

    async fn execute_upstream(
        &self,
        access: &McpAccessContext,
        original: &EntitledMcpTool,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, UpstreamFailure> {
        let current = self
            .store
            .resolve_entitled_tool(
                &access.workspace_id,
                access.user_id,
                &access.agent_id,
                &original.tool.public_name,
            )
            .await
            .map_err(|error| {
                authority_recheck_failure(error, "Tool access was revoked before execution")
            })?;
        require_same_authority(original, &current).map_err(|message| UpstreamFailure {
            failure: CallFailure::expected(message),
            ambiguous: false,
        })?;
        self.verify_live_catalog(access, &current)
            .await
            .map_err(|failure| UpstreamFailure {
                failure,
                ambiguous: false,
            })?;
        let bearer =
            decrypt_bearer(&current, &self.seal_key).map_err(|message| UpstreamFailure {
                failure: CallFailure::failed(message),
                ambiguous: false,
            })?;
        let prepared = prepare_tool_upstream(&current.endpoint_url, bearer.as_deref())
            .await
            .map_err(|_| UpstreamFailure {
                failure: CallFailure::failed("The upstream tool server is unavailable"),
                ambiguous: false,
            })?;
        let final_authority = self
            .store
            .resolve_entitled_tool(
                &access.workspace_id,
                access.user_id,
                &access.agent_id,
                &current.tool.public_name,
            )
            .await
            .map_err(|error| {
                authority_recheck_failure(error, "Tool access was revoked before execution")
            })?;
        require_same_authority(&current, &final_authority).map_err(|message| UpstreamFailure {
            failure: CallFailure::expected(message),
            ambiguous: false,
        })?;
        let call = CallToolRequestParams::new(current.tool.upstream_name.clone())
            .with_arguments(arguments);
        let result = prepared.call_tool(call).await;
        prepared.close().await;
        result.map_err(|_| UpstreamFailure {
            failure: CallFailure::failed(
                "The upstream tool call failed; its outcome may be ambiguous and clients must not retry automatically",
            ),
            ambiguous: true,
        })
    }
}

pub(super) fn resume_authorized_event(
    mut event: GuardEvent,
    authorization: AuthorizationClaim,
) -> GuardEvent {
    event.action.authorization = Some(authorization);
    event
}

fn build_preflight_event(
    access: &McpAccessContext,
    environment_id: &str,
    run: &HostedMcpRun,
    invocation_id: &str,
    entitled: &EntitledMcpTool,
    governed: &GovernedArguments,
    authorization: Option<AuthorizationClaim>,
) -> GuardEvent {
    build_tool_event(
        access,
        environment_id,
        run,
        invocation_id.to_string(),
        format!(
            "mcp:{}:{}",
            urlencoding::encode(&entitled.tool.connection_id),
            urlencoding::encode(&entitled.tool.upstream_name)
        ),
        governed
            .context
            .policy_parameters(governed.upstream.clone()),
        entitled.tool.side_effect,
        ToolIdentity {
            server_id: entitled.tool.connection_id.clone(),
            tool_name: entitled.tool.upstream_name.clone(),
            schema_hash: entitled.tool.schema_hash.clone(),
        },
        authorization,
        "mcp_preflight",
        Some(entitled),
    )
}

fn build_disclosure_event(
    access: &McpAccessContext,
    environment_id: &str,
    run: &HostedMcpRun,
    entitled: &EntitledMcpTool,
    context: &GovernanceContext,
    result: &GovernedResult,
    authorization: Option<AuthorizationClaim>,
) -> GuardEvent {
    let mut parameters = context
        .policy_parameters(serde_json::Map::new())
        .as_object()
        .cloned()
        .expect("policy parameters are an object");
    parameters
        .get_mut("__trustloop")
        .and_then(serde_json::Value::as_object_mut)
        .expect("governance metadata is an object")
        .insert(
            "policy_text".into(),
            serde_json::Value::String(result.policy_text.clone()),
        );
    parameters.insert("result_digest".into(), serde_json::json!(result.digest));
    parameters.insert(
        "result_bytes".into(),
        serde_json::json!(result.result_bytes),
    );
    parameters.insert(
        "content_types".into(),
        serde_json::json!(result.content_types),
    );
    let output_schema_hash = entitled
        .tool
        .output_schema
        .as_ref()
        .map(schema_hash)
        .unwrap_or_else(|| "sha256:v1:no-output-schema".into());
    build_tool_event(
        access,
        environment_id,
        run,
        Uuid::now_v7().to_string(),
        format!(
            "mcp_result:{}:{}",
            urlencoding::encode(&entitled.tool.connection_id),
            urlencoding::encode(&entitled.tool.upstream_name)
        ),
        serde_json::Value::Object(parameters),
        SideEffectClass::ExternalCommunication,
        ToolIdentity {
            server_id: entitled.tool.connection_id.clone(),
            tool_name: format!("{}.result", entitled.tool.upstream_name),
            schema_hash: output_schema_hash,
        },
        authorization,
        "mcp_result_disclosure",
        Some(entitled),
    )
}

fn build_result_validation_event(
    access: &McpAccessContext,
    environment_id: &str,
    run: &HostedMcpRun,
    entitled: &EntitledMcpTool,
    context: &GovernanceContext,
) -> GuardEvent {
    let mut parameters = context
        .policy_parameters(serde_json::Map::new())
        .as_object()
        .cloned()
        .expect("policy parameters are an object");
    parameters.remove("__trustloop");
    parameters.insert(
        "validation".into(),
        serde_json::Value::String("uninspectable_result".into()),
    );
    build_tool_event(
        access,
        environment_id,
        run,
        Uuid::now_v7().to_string(),
        format!(
            "mcp_result:{}:{}",
            urlencoding::encode(&entitled.tool.connection_id),
            urlencoding::encode(&entitled.tool.upstream_name)
        ),
        serde_json::Value::Object(parameters),
        SideEffectClass::ExternalCommunication,
        ToolIdentity {
            server_id: entitled.tool.connection_id.clone(),
            tool_name: format!("{}.result", entitled.tool.upstream_name),
            schema_hash: entitled
                .tool
                .output_schema
                .as_ref()
                .map(schema_hash)
                .unwrap_or_else(|| "sha256:v1:no-output-schema".into()),
        },
        None,
        "mcp_result_validation",
        Some(entitled),
    )
}

fn build_managed_failure_event(
    access: &McpAccessContext,
    environment_id: &str,
    run: &HostedMcpRun,
    invocation_id: &str,
    entitled: &EntitledMcpTool,
    validation: &str,
    side_effect: SideEffectClass,
) -> GuardEvent {
    build_tool_event(
        access,
        environment_id,
        run,
        invocation_id.to_string(),
        format!(
            "mcp:{}:{}",
            urlencoding::encode(&entitled.tool.connection_id),
            urlencoding::encode(&entitled.tool.upstream_name)
        ),
        serde_json::json!({"validation": validation}),
        side_effect,
        ToolIdentity {
            server_id: entitled.tool.connection_id.clone(),
            tool_name: entitled.tool.upstream_name.clone(),
            schema_hash: entitled.tool.schema_hash.clone(),
        },
        None,
        "mcp_preflight_validation",
        Some(entitled),
    )
}

fn build_unresolved_event(
    access: &McpAccessContext,
    environment_id: &str,
    run: &HostedMcpRun,
    invocation_id: &str,
    requested_name: &str,
) -> GuardEvent {
    build_tool_event(
        access,
        environment_id,
        run,
        invocation_id.to_string(),
        "mcp:unresolved".into(),
        serde_json::json!({"requested_public_name": bounded_label(requested_name)}),
        SideEffectClass::Read,
        ToolIdentity {
            server_id: "hosted-mcp-gateway".into(),
            tool_name: "unresolved".into(),
            schema_hash: "sha256:v1:unresolved".into(),
        },
        None,
        "mcp_entitlement",
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_tool_event(
    access: &McpAccessContext,
    environment_id: &str,
    run: &HostedMcpRun,
    invocation_id: String,
    operation: String,
    parameters: serde_json::Value,
    side_effect: SideEffectClass,
    tool_identity: ToolIdentity,
    authorization: Option<AuthorizationClaim>,
    checkpoint: &str,
    entitled: Option<&EntitledMcpTool>,
) -> GuardEvent {
    let mut context = serde_json::Map::new();
    context.insert(
        "integration_mode".into(),
        serde_json::Value::String("hosted_mcp".into()),
    );
    context.insert(
        "gateway_phase".into(),
        serde_json::Value::String(checkpoint.into()),
    );
    context.insert(
        "oauth_client_id".into(),
        serde_json::Value::String(access.client_id.clone()),
    );
    if let Some(entitled) = entitled {
        context.insert(
            "connection_id".into(),
            serde_json::Value::String(entitled.tool.connection_id.clone()),
        );
        context.insert(
            "public_tool_name".into(),
            serde_json::Value::String(entitled.tool.public_name.clone()),
        );
    }
    GuardEvent {
        kind: EventKind::ToolCallProposed,
        principal: Principal {
            workspace_id: access.workspace_id.clone(),
            environment_id: environment_id.to_string(),
            agent_id: access.agent_id.clone(),
            user_id: Some(access.user_id.to_string()),
            session_id: Some(access.client_id.clone()),
            task_id: None,
            run_id: run.run_id.clone(),
            run_event_id: run.run_event_id.clone(),
        },
        action: Action {
            operation,
            parameters,
            side_effect: Some(side_effect),
            invocation_id: Some(invocation_id),
            tool_identity: Some(tool_identity),
            authorization,
        },
        sources: Vec::new(),
        provenance: Default::default(),
        resolution: None,
        label_resolution: None,
        checks: Vec::new(),
        signals: Vec::new(),
        context: serde_json::Value::Object(context),
    }
}

fn decrypt_bearer(entitled: &EntitledMcpTool, key: &[u8; 32]) -> Result<Option<String>, String> {
    match entitled.encrypted_credential.as_deref() {
        Some(value) => crate::gateway::unseal_provider_key(value, key)
            .map(Some)
            .map_err(|_| "The upstream credential is unavailable".into()),
        None => Ok(None),
    }
}

pub(super) fn require_same_authority(
    original: &EntitledMcpTool,
    current: &EntitledMcpTool,
) -> Result<(), String> {
    if original.tool.id != current.tool.id
        || original.tool.schema_hash != current.tool.schema_hash
        || original.tool.upstream_name != current.tool.upstream_name
        || original.tool.side_effect != current.tool.side_effect
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
    environment_id: &str,
    lease_id: Option<&str>,
    status: LeaseStatus,
    outcome: serde_json::Value,
) -> Result<(), CallFailure> {
    let Some(lease_id) = lease_id else {
        return Ok(());
    };
    app.authorization_store
        .complete_lease(
            workspace_id,
            environment_id,
            lease_id,
            CompleteAuthorizationLeaseRequest { status, outcome },
        )
        .await
        .map(|_| ())
        .map_err(|error| {
            tracing::error!(
                workspace_id,
                environment_id,
                lease_id,
                error = %error,
                "MCP authorization lease completion failed"
            );
            CallFailure::failed(
                "The tool ran, but execution reconciliation failed; do not retry automatically",
            )
        })
}

fn wire_tool(tool: tl_core::McpGatewayTool) -> Result<Tool, McpError> {
    let input = governed_input_schema(&tool.input_schema)
        .and_then(|value| {
            value
                .as_object()
                .cloned()
                .ok_or_else(|| "Stored MCP tool schema is invalid".to_string())
        })
        .map_err(|message| McpError::internal_error(message, None))?;
    let output = tool
        .output_schema
        .and_then(|value| value.as_object().cloned())
        .map(Arc::new);
    let annotations = serde_json::from_value::<ToolAnnotations>(tool.annotations).ok();
    let description = managed_description(tool.description.as_deref());
    let mut value = Tool::new_with_raw(
        Cow::Owned(tool.public_name),
        Some(Cow::Owned(description)),
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

fn authority_recheck_failure(error: McpGatewayStoreError, message: &str) -> UpstreamFailure {
    match error {
        McpGatewayStoreError::NotFound => UpstreamFailure {
            failure: CallFailure::expected(message),
            ambiguous: false,
        },
        error => {
            tracing::error!(error = %error, "MCP authority recheck failed");
            UpstreamFailure {
                failure: CallFailure::failed("MCP runtime is unavailable"),
                ambiguous: false,
            }
        }
    }
}

fn bounded_label(value: &str) -> String {
    value.chars().take(200).collect()
}
