use axum::{
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Response,
};
use serde_json::json;
use tl_core::{
    CreateRunEventRequest, CreateRunRequest, FinalizeRunRequest, RunBoundarySource, RunEventKind,
    RunKind, RunParticipantRole, RunProviderUsage, RunStatus,
};

use crate::runs::RunListFilter;
use crate::AppState;

use super::super::normalization::{normalize_session_id, provider_kind_text};
use super::super::store::ResolvedGatewayRoute;

const GATEWAY_SESSION_ID_HEADER: &str = "x-featherlane-session-id";
const LEGACY_GATEWAY_SESSION_ID_HEADER: &str = "x-featherlane-ai-run-external-id";
const GATEWAY_SESSION_END_HEADER: &str = "x-featherlane-session-end";

#[derive(Debug, Clone)]
pub(super) struct GatewaySessionContext {
    pub(super) external_id: String,
    pub(super) finalize_after_response: bool,
    pub(super) customer_session: bool,
}

pub(super) fn gateway_session_context(
    headers: &HeaderMap,
    request_id: &str,
) -> Result<GatewaySessionContext, String> {
    let preferred = session_header(headers, GATEWAY_SESSION_ID_HEADER)?;
    let legacy = session_header(headers, LEGACY_GATEWAY_SESSION_ID_HEADER)?;
    if preferred.is_some() && legacy.is_some() && preferred != legacy {
        return Err(format!(
            "{GATEWAY_SESSION_ID_HEADER} and {LEGACY_GATEWAY_SESSION_ID_HEADER} disagree"
        ));
    }
    let external_id = preferred.or(legacy);
    let explicit_end = match headers.get(GATEWAY_SESSION_END_HEADER) {
        None => false,
        Some(value) => match value.to_str().map(str::trim) {
            Ok("true" | "1") => true,
            Ok("false" | "0") => false,
            _ => {
                return Err(format!(
                    "{GATEWAY_SESSION_END_HEADER} must be true, false, 1, or 0"
                ))
            }
        },
    };
    let customer_session = external_id.is_some();
    if explicit_end && !customer_session {
        return Err(format!(
            "{GATEWAY_SESSION_END_HEADER} requires {GATEWAY_SESSION_ID_HEADER}"
        ));
    }
    Ok(GatewaySessionContext {
        external_id: external_id.unwrap_or_else(|| request_id.to_string()),
        finalize_after_response: !customer_session || explicit_end,
        customer_session,
    })
}

fn session_header(headers: &HeaderMap, name: &str) -> Result<Option<String>, String> {
    let Some(value) = headers.get(name) else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| format!("{name} must contain visible UTF-8 characters"))?;
    normalize_session_id(value, name).map(Some)
}

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
                status: Some(RunStatus::Running),
                external_id: Some(external_id.to_string()),
                limit: 1,
                ..RunListFilter::default()
            },
        )
        .await
    {
        Ok(runs) => {
            if let Some(run) = runs.into_iter().find(|run| {
                run.metadata
                    .get("integration_mode")
                    .and_then(serde_json::Value::as_str)
                    == Some("gateway")
            }) {
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
                }),
            },
        )
        .await;

    match run {
        Ok(run) => {
            if let Err(error) = state
                .evaluation_store
                .register_participant_and_freeze_manifest(
                    workspace_id,
                    environment_id,
                    &run.id,
                    &resolved.route.agent_id,
                    RunParticipantRole::Primary,
                )
                .await
            {
                tracing::warn!(workspace_id, run_id = %run.id, error = %error, "gateway evaluation manifest freeze failed");
            }
            Some(run.id)
        }
        Err(crate::runs::RunStoreError::Conflict) => state
            .run_store
            .list(
                workspace_id,
                environment_id,
                RunListFilter {
                    agent_id: Some(resolved.route.agent_id.clone()),
                    status: Some(RunStatus::Running),
                    kind: Some(RunKind::ChatSession),
                    external_id: Some(external_id.to_string()),
                    limit: 1,
                    ..RunListFilter::default()
                },
            )
            .await
            .ok()
            .and_then(|runs| {
                runs.into_iter().find(|run| {
                    run.metadata
                        .get("integration_mode")
                        .and_then(serde_json::Value::as_str)
                        == Some("gateway")
                })
            })
            .map(|run| run.id),
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
    checked_input: &str,
    run_id: Option<&str>,
) -> Option<String> {
    let run_id = run_id?;
    let input_summary = if retain_gateway_body(state, workspace_id).await {
        Some(checked_input.to_string())
    } else {
        None
    };

    let event = CreateRunEventRequest {
        agent_id: None,
        kind: RunEventKind::UserTurn,
        sequence: None,
        label: Some("Gateway turn".to_string()),
        input_summary,
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
        Ok(event) => {
            touch_gateway_activity(state, workspace_id, environment_id, run_id).await;
            Some(event.id)
        }
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

pub(super) async fn create_gateway_assistant_event(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    gateway_request_id: &str,
    output: &str,
    usage: &RunProviderUsage,
    run_id: Option<&str>,
) -> Option<String> {
    let output_summary = if retain_gateway_body(state, workspace_id).await {
        Some(output.to_string())
    } else {
        None
    };
    create_gateway_evidence_event(
        state,
        workspace_id,
        environment_id,
        run_id,
        CreateRunEventRequest {
            agent_id: None,
            kind: RunEventKind::AssistantTurn,
            sequence: None,
            label: Some("Provider response".to_string()),
            input_summary: None,
            output_summary,
            metadata: json!({
                "integration_mode": "gateway",
                "gateway_request_id": gateway_request_id,
                "evidence_kind": "provider_usage",
                "provider_usage": usage,
            }),
            occurred_at: None,
        },
    )
    .await
}

async fn retain_gateway_body(state: &AppState, workspace_id: &str) -> bool {
    match state.settings_store.get(workspace_id).await {
        Ok(settings) => {
            crate::services::evidence_privacy::may_persist_gateway_body(settings.data_handling_mode)
        }
        Err(error) => {
            tracing::warn!(workspace_id, error = %error, "workspace privacy lookup failed; gateway body will not be persisted");
            false
        }
    }
}

pub(super) async fn create_gateway_provider_failure_event(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    gateway_request_id: &str,
    usage: &RunProviderUsage,
    run_id: Option<&str>,
) -> Option<String> {
    create_gateway_evidence_event(
        state,
        workspace_id,
        environment_id,
        run_id,
        CreateRunEventRequest {
            agent_id: None,
            kind: RunEventKind::SystemEvent,
            sequence: None,
            label: Some("Provider call failed".to_string()),
            input_summary: None,
            output_summary: None,
            metadata: json!({
                "integration_mode": "gateway",
                "gateway_request_id": gateway_request_id,
                "evidence_kind": "provider_usage",
                "provider_usage": usage,
            }),
            occurred_at: None,
        },
    )
    .await
}

async fn create_gateway_evidence_event(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
    event: CreateRunEventRequest,
) -> Option<String> {
    let run_id = run_id?;
    match state
        .run_store
        .create_event(workspace_id, environment_id, run_id, event)
        .await
    {
        Ok(event) => {
            touch_gateway_activity(state, workspace_id, environment_id, run_id).await;
            Some(event.id)
        }
        Err(error) => {
            tracing::warn!(workspace_id, environment_id, run_id, error = %error, "could not create gateway evidence event");
            None
        }
    }
}

async fn touch_gateway_activity(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: &str,
) {
    if let Err(error) = state
        .run_store
        .touch_gateway_activity(workspace_id, environment_id, run_id)
        .await
    {
        tracing::debug!(workspace_id, environment_id, run_id, error = %error, "gateway Run activity touch lost a finalization race");
    }
}

pub(super) async fn finish_gateway_run(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
    status: RunStatus,
    finalize_after_response: bool,
) {
    if let Some(run_id) = run_id {
        touch_gateway_activity(state, workspace_id, environment_id, run_id).await;
    }
    if !finalize_after_response {
        return;
    }
    let Some(run_id) = run_id else {
        return;
    };

    if let Err(error) = state
        .run_store
        .finalize(
            workspace_id,
            environment_id,
            run_id,
            FinalizeRunRequest {
                status,
                ended_at: None,
                boundary_source: RunBoundarySource::FrameworkAdapter,
                expected_flush_id: None,
                last_event_sequence: None,
            },
            gateway_capture_wait_ms(state, workspace_id, environment_id, run_id).await,
        )
        .await
    {
        tracing::warn!(
            workspace_id,
            run_id,
            error = %error,
            "gateway run finalization failed"
        );
    }
}

async fn gateway_capture_wait_ms(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: &str,
) -> u64 {
    let Ok(run) = state
        .run_store
        .get(workspace_id, environment_id, run_id)
        .await
    else {
        return 30_000;
    };
    state
        .evaluation_store
        .get_profile(workspace_id, environment_id, &run.agent_id)
        .await
        .ok()
        .flatten()
        .filter(|profile| profile.enabled)
        .map_or(30_000, |profile| profile.max_capture_wait_ms)
}

pub(super) fn attach_gateway_run_headers(
    response: &mut Response,
    run_id: Option<&str>,
    session: &GatewaySessionContext,
) {
    if let Some(run_id) = run_id.and_then(|value| HeaderValue::from_str(value).ok()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-featherlane-run-id"), run_id);
    }
    response.headers_mut().insert(
        HeaderName::from_static("x-featherlane-session-state"),
        HeaderValue::from_static(if !session.customer_session {
            "one_request"
        } else if session.finalize_after_response {
            "finalized"
        } else {
            "open"
        }),
    );
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::gateway_session_context;

    #[test]
    fn request_without_session_is_one_request_boundary() {
        let session = gateway_session_context(&HeaderMap::new(), "request-1").unwrap();
        assert_eq!(session.external_id, "request-1");
        assert!(session.finalize_after_response);
        assert!(!session.customer_session);
    }

    #[test]
    fn preferred_and_legacy_headers_must_agree() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-featherlane-session-id",
            HeaderValue::from_static("session-a"),
        );
        headers.insert(
            "x-featherlane-ai-run-external-id",
            HeaderValue::from_static("session-b"),
        );
        assert!(gateway_session_context(&headers, "request-1")
            .unwrap_err()
            .contains("disagree"));
    }

    #[test]
    fn explicit_end_requires_and_finalizes_a_session() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-featherlane-session-id",
            HeaderValue::from_static("session-a"),
        );
        headers.insert(
            "x-featherlane-session-end",
            HeaderValue::from_static("true"),
        );
        let session = gateway_session_context(&headers, "request-1").unwrap();
        assert!(session.customer_session);
        assert!(session.finalize_after_response);
    }

    #[test]
    fn session_id_is_nonempty_and_bounded_by_bytes() {
        let mut empty = HeaderMap::new();
        empty.insert("x-featherlane-session-id", HeaderValue::from_static("  "));
        assert!(gateway_session_context(&empty, "request-1").is_err());

        let mut accepted = HeaderMap::new();
        accepted.insert(
            "x-featherlane-session-id",
            HeaderValue::from_str(&"x".repeat(200)).unwrap(),
        );
        assert!(gateway_session_context(&accepted, "request-1").is_ok());

        let mut rejected = HeaderMap::new();
        rejected.insert(
            "x-featherlane-session-id",
            HeaderValue::from_str(&"x".repeat(201)).unwrap(),
        );
        assert!(gateway_session_context(&rejected, "request-1").is_err());
    }
}
