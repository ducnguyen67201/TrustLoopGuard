use serde_json::json;
use tl_core::{
    CreateRunEventRequest, CreateRunRequest, RunEventKind, RunKind, RunStatus, UpdateRunRequest,
};

use crate::AppState;

#[derive(Debug, Clone, Default)]
pub(super) struct HostedMcpRun {
    pub run_id: Option<String>,
    pub run_event_id: Option<String>,
}

pub(super) async fn create_hosted_mcp_run(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    agent_id: &str,
    oauth_client_id: &str,
    invocation_id: &str,
    public_tool_name: &str,
) -> HostedMcpRun {
    let run = match app
        .run_store
        .create(
            workspace_id,
            environment_id,
            CreateRunRequest {
                agent_id: agent_id.to_string(),
                kind: RunKind::Workflow,
                status: Some(RunStatus::Running),
                external_id: Some(invocation_id.to_string()),
                metadata: json!({
                    "integration_mode": "hosted_mcp",
                    "oauth_client_id": oauth_client_id,
                    "public_tool_name": public_tool_name,
                }),
            },
        )
        .await
    {
        Ok(run) => run,
        Err(error) => {
            tracing::warn!(
                workspace_id,
                agent_id,
                invocation_id,
                error = %error,
                "hosted MCP run creation failed; request will continue without run grouping"
            );
            return HostedMcpRun::default();
        }
    };
    let event = app
        .run_store
        .create_event(
            workspace_id,
            environment_id,
            &run.id,
            CreateRunEventRequest {
                kind: RunEventKind::ToolCall,
                sequence: None,
                label: Some(format!("Hosted MCP: {public_tool_name}")),
                input_summary: None,
                output_summary: None,
                metadata: json!({
                    "integration_mode": "hosted_mcp",
                    "invocation_id": invocation_id,
                    "public_tool_name": public_tool_name,
                }),
                occurred_at: None,
            },
        )
        .await;
    let run_event_id = match event {
        Ok(event) => Some(event.id),
        Err(error) => {
            tracing::warn!(
                workspace_id,
                run_id = %run.id,
                error = %error,
                "hosted MCP run event creation failed"
            );
            None
        }
    };
    HostedMcpRun {
        run_id: Some(run.id),
        run_event_id,
    }
}

pub(super) async fn finish_hosted_mcp_run(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
    status: RunStatus,
) {
    let Some(run_id) = run_id else {
        return;
    };
    if let Err(error) = app
        .run_store
        .update(
            workspace_id,
            environment_id,
            run_id,
            UpdateRunRequest {
                status: Some(status),
                metadata: None,
                ended_at: None,
            },
        )
        .await
    {
        tracing::warn!(
            workspace_id,
            run_id,
            error = %error,
            "hosted MCP run status update failed"
        );
    }
}
