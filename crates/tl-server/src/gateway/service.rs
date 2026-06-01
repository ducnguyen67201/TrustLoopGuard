use std::time::Instant;

use axum::{
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use serde_json::{json, Value};
use tl_core::{
    Channel, CheckRequest, CreateRunEventRequest, CreateRunRequest, Decision, EnforcementProfile,
    FailMode, GatewayInputAction, GatewayOutputAction, GatewayProviderConnection,
    GatewayProviderKind, ResponseMode, RetentionMode, RunEventKind, RunKind, RunStatus,
    UpdateRunRequest, Verdict,
};
use uuid::Uuid;

use crate::policies::workspace_id_from_headers;
use crate::runs::RunListFilter;
use crate::{execute_check_request, AppState};

use super::crypto::unseal_provider_key;
use super::errors::{api_error_response, gateway_store_error_response};
use super::normalization::{provider_kind_text, retention_mode_text};
use super::provider::{latest_user_message_content, GatewayProvider};
use super::store::{GatewayStoreError, ResolvedGatewayRoute};
use super::GatewayState;

const GATEWAY_RUN_EXTERNAL_ID_HEADER: &str = "x-tlg-run-external-id";

pub(super) async fn proxy_provider_request<P: GatewayProvider>(
    state: GatewayState,
    headers: HeaderMap,
    route_id: String,
    body: Bytes,
    expected_kind: GatewayProviderKind,
    provider: P,
) -> Response {
    let gateway_request_id = Uuid::now_v7().to_string();
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.app.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(environment_id) => environment_id,
        Err(error) => return crate::environments::environment_error_response(error),
    };
    let resolved = match state
        .store
        .resolve_gateway_route(&workspace_id, &route_id)
        .await
    {
        Ok(resolved) => resolved,
        Err(GatewayStoreError::NotFound) => {
            return api_error_response(StatusCode::NOT_FOUND, "gateway route not found".into());
        }
        Err(error) => return gateway_store_error_response(error),
    };

    if resolved.provider_connection.kind != expected_kind {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            "gateway route provider kind does not match endpoint".into(),
        );
    }

    const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
    if body.len() > MAX_BODY_BYTES {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            format!("request body exceeds maximum size of {MAX_BODY_BYTES} bytes"),
        );
    }

    let mut request = match serde_json::from_slice::<Value>(&body) {
        Ok(value) => value,
        Err(error) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                format!("provider request body must be JSON: {error}"),
            );
        }
    };

    let wants_stream = provider.is_streaming(&request);
    if wants_stream {
        if resolved.enforcement_profile.response_mode != ResponseMode::Streaming {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                "streaming is not enabled for this route; set the enforcement profile response_mode to \"streaming\"".into(),
            );
        }
        provider.strip_streaming_fields(&mut request);
    }

    let run_external_id = gateway_run_external_id(&headers, &gateway_request_id);
    let run_id = create_gateway_run(
        &state.app,
        &workspace_id,
        &environment_id,
        &resolved,
        &gateway_request_id,
        &run_external_id,
    )
    .await;

    let provider_api_key = match unseal_provider_key(&resolved.encrypted_api_key, &state.seal_key) {
        Ok(key) => key,
        Err(message) => {
            tracing::error!(
                workspace_id = %workspace_id,
                route_id = %route_id,
                connection_id = %resolved.provider_connection.id,
                "provider credential decryption failed"
            );
            finish_gateway_run(
                &state.app,
                &workspace_id,
                &environment_id,
                run_id.as_deref(),
                RunStatus::Failed,
            )
            .await;
            return api_error_response(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
    };

    let input = provider.extract_input(&request);
    let run_event_id = create_gateway_turn_event(
        &state.app,
        &workspace_id,
        &environment_id,
        &resolved,
        &gateway_request_id,
        &request,
        run_id.as_deref(),
    )
    .await;
    let input_decision = match check_gateway_content(
        &state.app,
        GatewayContentCheck {
            workspace_id: &workspace_id,
            environment_id: &environment_id,
            resolved: &resolved,
            phase: "gateway_input_check",
            input: &input,
            proposed_output: &input,
            run_id: run_id.as_deref(),
            run_event_id: run_event_id.as_deref(),
        },
    )
    .await
    {
        Ok(decision) => decision,
        Err(response) => {
            finish_gateway_run(
                &state.app,
                &workspace_id,
                &environment_id,
                run_id.as_deref(),
                RunStatus::Failed,
            )
            .await;
            return response;
        }
    };
    log_gateway_decision(GatewayDecisionLog {
        workspace_id: &workspace_id,
        environment_id: &environment_id,
        route_id: &route_id,
        run_id: run_id.as_deref(),
        phase: "input",
        decision: &input_decision,
        content_chars: input.chars().count(),
        wants_stream,
    });

    if input_decision.verdict != Verdict::Allow {
        match resolved.enforcement_profile.input_action {
            GatewayInputAction::Allow => {
                tracing::info!(
                    workspace_id = %workspace_id,
                    route_id = %route_id,
                    verdict = ?input_decision.verdict,
                    "input verdict is non-allow but enforcement profile input_action=allow; request proceeds"
                );
            }
            GatewayInputAction::Block => {
                let pid = input_decision
                    .triggered_policies
                    .first()
                    .map(|p| p.id.as_str());
                let response = finalize_gateway_response(
                    &provider,
                    wants_stream,
                    provider.safe_response(&request, &resolved.enforcement_profile),
                    Some(EnforcementHeaders {
                        verdict: "blocked",
                        trace_id: &input_decision.trace_id,
                        phase: "input",
                        policy_id: pid,
                    }),
                );
                finish_gateway_run(
                    &state.app,
                    &workspace_id,
                    &environment_id,
                    run_id.as_deref(),
                    RunStatus::Completed,
                )
                .await;
                return response;
            }
            GatewayInputAction::Redact => {
                let safe_input = input_decision
                    .safe_output
                    .clone()
                    .unwrap_or_else(|| "[redacted]".to_string());
                provider.apply_input_rewrite(&mut request, &safe_input);
            }
        }
    }

    let provider_response = match provider
        .forward(
            &state.http,
            &resolved.provider_connection,
            &provider_api_key,
            request.clone(),
        )
        .await
    {
        Ok(response) => response,
        Err(error) => {
            finish_gateway_run(
                &state.app,
                &workspace_id,
                &environment_id,
                run_id.as_deref(),
                RunStatus::Failed,
            )
            .await;
            return handle_provider_failure(
                &provider,
                wants_stream,
                &request,
                &resolved.enforcement_profile,
                error,
            );
        }
    };

    let output = provider.extract_output(&provider_response);
    let output_decision = match check_gateway_content(
        &state.app,
        GatewayContentCheck {
            workspace_id: &workspace_id,
            environment_id: &environment_id,
            resolved: &resolved,
            phase: "gateway_output_check",
            input: &input,
            proposed_output: &output,
            run_id: run_id.as_deref(),
            run_event_id: run_event_id.as_deref(),
        },
    )
    .await
    {
        Ok(decision) => decision,
        Err(response) => {
            finish_gateway_run(
                &state.app,
                &workspace_id,
                &environment_id,
                run_id.as_deref(),
                RunStatus::Failed,
            )
            .await;
            return response;
        }
    };
    log_gateway_decision(GatewayDecisionLog {
        workspace_id: &workspace_id,
        environment_id: &environment_id,
        route_id: &route_id,
        run_id: run_id.as_deref(),
        phase: "output",
        decision: &output_decision,
        content_chars: output.chars().count(),
        wants_stream,
    });

    if output_decision.verdict == Verdict::Allow {
        finish_gateway_run(
            &state.app,
            &workspace_id,
            &environment_id,
            run_id.as_deref(),
            RunStatus::Completed,
        )
        .await;
        return finalize_gateway_response(&provider, wants_stream, provider_response, None);
    }

    let response = match resolved.enforcement_profile.output_action {
        GatewayOutputAction::Allow => {
            finalize_gateway_response(&provider, wants_stream, provider_response, None)
        }
        GatewayOutputAction::Block => {
            let pid = output_decision
                .triggered_policies
                .first()
                .map(|p| p.id.as_str());
            finalize_gateway_response(
                &provider,
                wants_stream,
                provider.safe_response(&request, &resolved.enforcement_profile),
                Some(EnforcementHeaders {
                    verdict: "blocked",
                    trace_id: &output_decision.trace_id,
                    phase: "output",
                    policy_id: pid,
                }),
            )
        }
        GatewayOutputAction::Escalate => {
            let pid = output_decision
                .triggered_policies
                .first()
                .map(|p| p.id.as_str());
            finalize_gateway_response(
                &provider,
                wants_stream,
                provider.safe_response(&request, &resolved.enforcement_profile),
                Some(EnforcementHeaders {
                    verdict: "escalated",
                    trace_id: &output_decision.trace_id,
                    phase: "output",
                    policy_id: pid,
                }),
            )
        }
        GatewayOutputAction::Rewrite => {
            if let Some(safe_out) = output_decision.safe_output.as_deref() {
                let response = finalize_gateway_response(
                    &provider,
                    wants_stream,
                    provider.apply_output_rewrite(provider_response, safe_out),
                    None,
                );
                finish_gateway_run(
                    &state.app,
                    &workspace_id,
                    &environment_id,
                    run_id.as_deref(),
                    RunStatus::Completed,
                )
                .await;
                return response;
            }

            if resolved.enforcement_profile.max_regenerations > 0 {
                match check_and_maybe_regenerate(
                    &state.app,
                    &provider,
                    &state.http,
                    &resolved.provider_connection,
                    &provider_api_key,
                    &workspace_id,
                    &environment_id,
                    &resolved,
                    request.clone(),
                    provider_response,
                    output_decision,
                    &gateway_request_id,
                    run_id.as_deref(),
                    run_event_id.as_deref(),
                )
                .await
                {
                    Ok(clean) => {
                        finish_gateway_run(
                            &state.app,
                            &workspace_id,
                            &environment_id,
                            run_id.as_deref(),
                            RunStatus::Completed,
                        )
                        .await;
                        return finalize_gateway_response(&provider, wants_stream, clean, None);
                    }
                    Err(final_decision) => {
                        let pid = final_decision
                            .triggered_policies
                            .first()
                            .map(|p| p.id.as_str());
                        let response = finalize_gateway_response(
                            &provider,
                            wants_stream,
                            provider.safe_response(&request, &resolved.enforcement_profile),
                            Some(EnforcementHeaders {
                                verdict: "blocked",
                                trace_id: &final_decision.trace_id,
                                phase: "output",
                                policy_id: pid,
                            }),
                        );
                        finish_gateway_run(
                            &state.app,
                            &workspace_id,
                            &environment_id,
                            run_id.as_deref(),
                            RunStatus::Completed,
                        )
                        .await;
                        return response;
                    }
                }
            }

            let pid = output_decision
                .triggered_policies
                .first()
                .map(|p| p.id.as_str());
            finalize_gateway_response(
                &provider,
                wants_stream,
                provider.safe_response(&request, &resolved.enforcement_profile),
                Some(EnforcementHeaders {
                    verdict: "blocked",
                    trace_id: &output_decision.trace_id,
                    phase: "output",
                    policy_id: pid,
                }),
            )
        }
    };

    finish_gateway_run(
        &state.app,
        &workspace_id,
        &environment_id,
        run_id.as_deref(),
        RunStatus::Completed,
    )
    .await;
    response
}

struct GatewayContentCheck<'a> {
    workspace_id: &'a str,
    environment_id: &'a str,
    resolved: &'a ResolvedGatewayRoute,
    phase: &'a str,
    input: &'a str,
    proposed_output: &'a str,
    run_id: Option<&'a str>,
    run_event_id: Option<&'a str>,
}

async fn check_gateway_content(
    state: &AppState,
    check: GatewayContentCheck<'_>,
) -> Result<Decision, Response> {
    let mut context = json!({
        "integration_mode": "gateway",
        "gateway_phase": check.phase,
        "provider": provider_kind_text(check.resolved.provider_connection.kind),
        "route_id": check.resolved.route.id,
        "enforcement_profile_id": check.resolved.enforcement_profile.id,
        "retention_mode": retention_mode_text(check.resolved.enforcement_profile.retention_mode),
    });
    if check.resolved.enforcement_profile.retention_mode == RetentionMode::MetadataOnly {
        context["body_retention"] = json!("omitted");
    }
    let req = CheckRequest {
        workspace_id: Some(check.workspace_id.to_string()),
        agent_id: check.resolved.route.agent_id.clone(),
        channel: Channel::Chat,
        input: check.input.to_string(),
        proposed_output: check.proposed_output.to_string(),
        domain: Some(check.phase.to_string()),
        run_id: check.run_id.map(str::to_string),
        run_event_id: check.run_event_id.map(str::to_string),
        context,
        ..CheckRequest::default()
    };
    execute_check_request(
        state,
        check.workspace_id,
        check.environment_id,
        req,
        Instant::now(),
    )
    .await
}

async fn create_gateway_turn_event(
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

struct GatewayDecisionLog<'a> {
    workspace_id: &'a str,
    environment_id: &'a str,
    route_id: &'a str,
    run_id: Option<&'a str>,
    phase: &'a str,
    decision: &'a Decision,
    content_chars: usize,
    wants_stream: bool,
}

fn log_gateway_decision(fields: GatewayDecisionLog<'_>) {
    let policy_ids = fields
        .decision
        .triggered_policies
        .iter()
        .map(|policy| policy.id.as_str())
        .collect::<Vec<_>>()
        .join(",");

    tracing::info!(
        workspace_id = %fields.workspace_id,
        environment_id = %fields.environment_id,
        route_id = %fields.route_id,
        run_id = fields.run_id.unwrap_or(""),
        phase = %fields.phase,
        verdict = ?fields.decision.verdict,
        trace_id = %fields.decision.trace_id,
        latency_ms = fields.decision.latency_ms,
        triggered_policy_count = fields.decision.triggered_policies.len(),
        policy_ids = %policy_ids,
        reason = %fields.decision.reason,
        content_chars = fields.content_chars,
        streaming = fields.wants_stream,
        "gateway policy check completed"
    );
}

async fn create_gateway_run(
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

fn gateway_run_external_id(headers: &HeaderMap, fallback: &str) -> String {
    headers
        .get(GATEWAY_RUN_EXTERNAL_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

async fn finish_gateway_run(
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

struct EnforcementHeaders<'a> {
    verdict: &'static str,
    trace_id: &'a str,
    phase: &'static str,
    policy_id: Option<&'a str>,
}

fn apply_enforcement_headers(response: &mut Response, h: &EnforcementHeaders<'_>) {
    let hm = response.headers_mut();
    hm.insert(
        HeaderName::from_static("x-trustloopguard-verdict"),
        HeaderValue::from_static(h.verdict),
    );
    hm.insert(
        HeaderName::from_static("x-trustloopguard-trace-id"),
        HeaderValue::from_str(h.trace_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    hm.insert(
        HeaderName::from_static("x-trustloopguard-phase"),
        HeaderValue::from_static(h.phase),
    );
    if let Some(pid) = h.policy_id {
        if let Ok(v) = HeaderValue::from_str(pid) {
            hm.insert(HeaderName::from_static("x-trustloopguard-policy-id"), v);
        }
    }
}

fn finalize_gateway_response<P: GatewayProvider>(
    provider: &P,
    wants_stream: bool,
    body: Value,
    headers: Option<EnforcementHeaders<'_>>,
) -> Response {
    let mut response = if wants_stream {
        let mut resp = provider.streaming_sse_body(&body).into_response();
        let hm = resp.headers_mut();
        hm.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        hm.insert(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-cache"),
        );
        resp
    } else {
        Json(body).into_response()
    };
    if let Some(h) = headers {
        apply_enforcement_headers(&mut response, &h);
    }
    response
}

fn append_assistant_turn(request: &mut Value, content: String) {
    if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
        messages.push(json!({ "role": "assistant", "content": content }));
    }
}

#[allow(clippy::too_many_arguments)]
async fn check_and_maybe_regenerate<P: GatewayProvider>(
    app_state: &AppState,
    provider: &P,
    http: &reqwest::Client,
    connection: &GatewayProviderConnection,
    api_key: &str,
    workspace_id: &str,
    environment_id: &str,
    resolved: &ResolvedGatewayRoute,
    initial_request: Value,
    initial_response: Value,
    initial_decision: Decision,
    gateway_request_id: &str,
    run_id: Option<&str>,
    run_event_id: Option<&str>,
) -> Result<Value, Decision> {
    let max = resolved.enforcement_profile.max_regenerations as usize;
    let original_input = provider.extract_input(&initial_request);
    let mut req = initial_request;
    let mut last_decision = initial_decision;

    append_assistant_turn(&mut req, provider.extract_output(&initial_response));
    provider.inject_feedback(&mut req, &last_decision.reason);

    for attempt in 1..=max {
        tracing::info!(
            gateway_request_id,
            attempt,
            max,
            trace_id = %last_decision.trace_id,
            "max_regenerations: re-forwarding to provider"
        );

        let retry_resp = match provider
            .forward(http, connection, api_key, req.clone())
            .await
        {
            Ok(r) => r,
            Err(error) => {
                tracing::warn!(
                    gateway_request_id,
                    attempt,
                    error,
                    "regeneration attempt failed at provider"
                );
                break;
            }
        };

        let retry_output = provider.extract_output(&retry_resp);
        let retry_decision = match check_gateway_content(
            app_state,
            GatewayContentCheck {
                workspace_id,
                environment_id,
                resolved,
                phase: "gateway_output_check",
                input: &original_input,
                proposed_output: &retry_output,
                run_id,
                run_event_id,
            },
        )
        .await
        {
            Ok(d) => d,
            Err(_) => break,
        };

        if retry_decision.verdict == Verdict::Allow {
            tracing::info!(
                gateway_request_id,
                attempt,
                "max_regenerations: output passed on retry; self-healing succeeded"
            );
            return Ok(retry_resp);
        }

        last_decision = retry_decision;
        if attempt < max {
            append_assistant_turn(&mut req, provider.extract_output(&retry_resp));
            provider.inject_feedback(&mut req, &last_decision.reason);
        }
    }

    tracing::warn!(
        gateway_request_id,
        max,
        "max_regenerations: all attempts exhausted; falling back to safe response"
    );
    Err(last_decision)
}

fn handle_provider_failure<P: GatewayProvider>(
    provider: &P,
    wants_stream: bool,
    request: &Value,
    profile: &EnforcementProfile,
    error: String,
) -> Response {
    match profile.fail_mode {
        FailMode::Open => {
            tracing::warn!(error = %error, "upstream provider request failed");
            api_error_response(
                StatusCode::BAD_GATEWAY,
                "upstream provider request failed".into(),
            )
        }
        FailMode::Closed => {
            tracing::warn!(error = %error, "provider failure suppressed by fail_mode=closed; returning safe response");
            finalize_gateway_response(
                provider,
                wants_stream,
                provider.safe_response(request, profile),
                Some(EnforcementHeaders {
                    verdict: "blocked",
                    trace_id: &Uuid::now_v7().to_string(),
                    phase: "output",
                    policy_id: None,
                }),
            )
        }
    }
}
