use axum::http::HeaderMap;
use serde_json::{json, Value};
use tl_core::{
    CreateRunEventRequest, CreateRunRequest, RunEventKind, RunKind, RunStatus, UpdateRunRequest,
};

use crate::runs::RunListFilter;
use crate::AppState;

use super::super::normalization::provider_kind_text;
use super::super::provider::latest_user_message_content;
use super::super::store::ResolvedGatewayRoute;

const GATEWAY_RUN_EXTERNAL_ID_HEADER: &str = "x-tlg-run-external-id";

pub(super) async fn create_gateway_run(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    resolved: &ResolvedGatewayRoute,
    gateway_request_id: &str,
    external_id: &str,
) -> Option<String> {
    match state
        .run_store
        .list(
            workspace_id,
            environment_id,
            RunListFilter {
                agent_id: Some(resolved.route.agent_id.clone()),
                kind: Some(RunKind::ChatSession),
                external_id: Some(external_id.to_string()),
                limit: 1,
                ..RunListFilter::default()
            },
        )
        .await
    {
        Ok(runs) => {
            if let Some(run) = runs.into_iter().next() {
                return Some(run.id);
            }
        }
        Err(error) => {
            tracing::warn!(
                workspace_id,
                route_id = %resolved.route.id,
                external_id,
                error = %error,
                "gateway run lookup failed; creating a new run"
            );
        }
    }

    let run = state
        .run_store
        .create(
            workspace_id,
            environment_id,
            CreateRunRequest {
                agent_id: resolved.route.agent_id.clone(),
                kind: RunKind::ChatSession,
                status: Some(RunStatus::Running),
                external_id: Some(external_id.to_string()),
                metadata: json!({
                    "integration_mode": "gateway",
                    "route_id": resolved.route.id,
                    "gateway_request_id": gateway_request_id,
                    "provider": provider_kind_text(resolved.provider_connection.kind),
                    "enforcement_profile_id": resolved.enforcement_profile.id,
                }),
            },
        )
        .await;

    match run {
        Ok(run) => Some(run.id),
        Err(error) => {
            tracing::warn!(
                workspace_id,
                route_id = %resolved.route.id,
                error = %error,
                "gateway run creation failed; request will continue without run grouping"
            );
            None
        }
    }
}

pub(super) async fn create_gateway_turn_event(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    resolved: &ResolvedGatewayRoute,
    gateway_request_id: &str,
    request: &Value,
    run_id: Option<&str>,
) -> Option<String> {
    let run_id = run_id?;

    let event = CreateRunEventRequest {
        kind: RunEventKind::UserTurn,
        sequence: None,
        label: Some("Gateway turn".to_string()),
        input_summary: latest_user_message_content(request),
        output_summary: None,
        metadata: json!({
            "integration_mode": "gateway",
            "gateway_request_id": gateway_request_id,
            "route_id": resolved.route.id,
            "provider": provider_kind_text(resolved.provider_connection.kind),
        }),
        occurred_at: None,
    };

    match state
        .run_store
        .create_event(workspace_id, environment_id, run_id, event)
        .await
    {
        Ok(event) => Some(event.id),
        Err(error) => {
            tracing::warn!(
                workspace_id,
                environment_id,
                run_id,
                error = %error,
                "could not create gateway run event"
            );
            None
        }
    }
}

pub(super) fn gateway_run_external_id(headers: &HeaderMap, fallback: &str) -> String {
    headers
        .get(GATEWAY_RUN_EXTERNAL_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

pub(super) async fn finish_gateway_run(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
    status: RunStatus,
) {
    let Some(run_id) = run_id else {
        return;
    };

    if let Err(error) = state
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
            "gateway run status update failed"
        );
    }
}
