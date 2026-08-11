mod checks;
mod output;
mod request;
mod response;
mod runs;

use axum::{
    http::{HeaderMap, StatusCode},
    response::Response,
};
use bytes::Bytes;
use std::time::Instant;
use tl_core::{
    AuthorizationEffect, GatewayProviderKind, GatewayReliabilityMode, RunProviderUsage, RunStatus,
};
use uuid::Uuid;

use crate::policies::workspace_id_from_headers;

use super::budget;
use super::crypto::unseal_provider_key;
use super::errors::{api_error_response, gateway_store_error_response};
use super::provider::GatewayProvider;
use super::store::GatewayStoreError;
use super::GatewayState;
use checks::{
    check_gateway_content, log_gateway_decision, GatewayContentCheck, GatewayDecisionLog,
    GATEWAY_INPUT_SOURCE_ID, GATEWAY_OUTPUT_SOURCE_ID,
};
use output::{handle_output_enforcement, OutputEnforcement};
use request::{parse_provider_request, prepare_streaming_request};
use response::{finalize_gateway_response, handle_provider_failure, EnforcementHeaders};
use runs::{
    attach_gateway_run_headers, create_gateway_assistant_event,
    create_gateway_provider_failure_event, create_gateway_run, create_gateway_turn_event,
    finish_gateway_run, gateway_session_context,
};

fn with_run_headers(
    mut response: Response,
    run_id: Option<&str>,
    session: &runs::GatewaySessionContext,
) -> Response {
    attach_gateway_run_headers(&mut response, run_id, session);
    response
}

pub(super) async fn proxy_provider_request<P: GatewayProvider>(
    state: GatewayState,
    headers: HeaderMap,
    route_id: String,
    body: Bytes,
    expected_kind: GatewayProviderKind,
    provider: P,
    runtime_key: Option<crate::auth::WorkspaceKeyContext>,
) -> Response {
    let gateway_request_id = Uuid::now_v7().to_string();
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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

    let mut request = match parse_provider_request(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let wants_stream = prepare_streaming_request(&provider, &mut request);

    let metered = expected_kind == GatewayProviderKind::OpenaiCompatible;

    let session = match gateway_session_context(&headers, &gateway_request_id) {
        Ok(session) => session,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    let auto_finalize_run = session.finalize_after_response;
    let run_id = create_gateway_run(
        &state.app,
        &workspace_id,
        &environment_id,
        &resolved,
        &gateway_request_id,
        &session.external_id,
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
                auto_finalize_run,
            )
            .await;
            return with_run_headers(
                api_error_response(StatusCode::INTERNAL_SERVER_ERROR, message),
                run_id.as_deref(),
                &session,
            );
        }
    };

    let input = provider.extract_input(&request);
    let run_event_id = create_gateway_turn_event(
        &state.app,
        &workspace_id,
        &environment_id,
        &resolved,
        &gateway_request_id,
        &input,
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
            text_source_id: GATEWAY_INPUT_SOURCE_ID,
            proposed_output: &input,
            run_id: run_id.as_deref(),
            run_event_id: run_event_id.as_deref(),
            gateway_request_id: &gateway_request_id,
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
                auto_finalize_run,
            )
            .await;
            return with_run_headers(response, run_id.as_deref(), &session);
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

    match input_decision.effect {
        AuthorizationEffect::Permit => {}
        AuthorizationEffect::Transform => {
            if let Some(safe_input) = input_decision.safe_output.as_deref() {
                provider.apply_input_rewrite(&mut request, safe_input);
            } else {
                let response = blocked_response(
                    &provider,
                    wants_stream,
                    &request,
                    &input_decision,
                    "blocked",
                    "input",
                );
                finish_gateway_run(
                    &state.app,
                    &workspace_id,
                    &environment_id,
                    run_id.as_deref(),
                    RunStatus::Completed,
                    auto_finalize_run,
                )
                .await;
                return with_run_headers(response, run_id.as_deref(), &session);
            }
        }
        AuthorizationEffect::Deny
        | AuthorizationEffect::RequireApproval
        | AuthorizationEffect::Defer => {
            let effect = match input_decision.effect {
                AuthorizationEffect::RequireApproval => "require_approval",
                AuthorizationEffect::Defer => "defer",
                _ => "deny",
            };
            let response = blocked_response(
                &provider,
                wants_stream,
                &request,
                &input_decision,
                effect,
                "input",
            );
            finish_gateway_run(
                &state.app,
                &workspace_id,
                &environment_id,
                run_id.as_deref(),
                RunStatus::Completed,
                auto_finalize_run,
            )
            .await;
            return with_run_headers(response, run_id.as_deref(), &session);
        }
    }

    let provider_name = super::normalization::provider_kind_text(expected_kind);
    for fallback in &resolved.fallback_provider_connections {
        if fallback.connection.kind != expected_kind
            || fallback.connection.kind == GatewayProviderKind::PaymentHttp
        {
            finish_gateway_run(
                &state.app,
                &workspace_id,
                &environment_id,
                run_id.as_deref(),
                RunStatus::Failed,
                auto_finalize_run,
            )
            .await;
            return with_run_headers(
                api_error_response(
                    StatusCode::BAD_REQUEST,
                    "fallback provider must use the route provider protocol".into(),
                ),
                run_id.as_deref(),
                &session,
            );
        }
    }

    let mut attempts = vec![(&resolved.provider_connection, provider_api_key)];
    if resolved.route.reliability_mode == GatewayReliabilityMode::Standard {
        let retry_key = attempts[0].1.clone();
        attempts.push((&resolved.provider_connection, retry_key));
        if let Some(fallback) = resolved.fallback_provider_connections.first() {
            let key = match unseal_provider_key(&fallback.encrypted_api_key, &state.seal_key) {
                Ok(key) => key,
                Err(_) => {
                    finish_gateway_run(
                        &state.app,
                        &workspace_id,
                        &environment_id,
                        run_id.as_deref(),
                        RunStatus::Failed,
                        auto_finalize_run,
                    )
                    .await;
                    return with_run_headers(
                        api_error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "fallback provider credential could not be resolved".into(),
                        ),
                        run_id.as_deref(),
                        &session,
                    );
                }
            };
            attempts.push((&fallback.connection, key));
        }
    }

    let attempt_deadline = Instant::now() + std::time::Duration::from_secs(120);
    let mut final_error = None;
    let mut successful = None;
    for (index, (connection, api_key)) in attempts.into_iter().enumerate() {
        let attempt = index as u32 + 1;
        let attempt_request_id = format!("{gateway_request_id}:attempt:{attempt}");
        let mut attempt_request = request.clone();
        if attempt_request.get("model").is_none() {
            attempt_request["model"] = serde_json::json!(connection.default_model);
        }
        let budget_reservation = if metered {
            match budget::reserve_llm_budget(
                &state.app,
                &workspace_id,
                &environment_id,
                runtime_key.as_ref(),
                &attempt_request_id,
                &attempt_request,
                run_id.as_deref(),
            )
            .await
            {
                Ok(reservation) => reservation,
                Err(response) => {
                    finish_gateway_run(
                        &state.app,
                        &workspace_id,
                        &environment_id,
                        run_id.as_deref(),
                        RunStatus::Failed,
                        auto_finalize_run,
                    )
                    .await;
                    return with_run_headers(response, run_id.as_deref(), &session);
                }
            }
        } else {
            None
        };
        let provider_started = Instant::now();
        match provider
            .forward(&state.http, connection, &api_key, attempt_request.clone())
            .await
        {
            Ok(provider_response) => {
                let latency_ms = provider_started.elapsed().as_millis() as u64;
                let usage = if metered {
                    budget::meter_llm_usage(
                        &state.app,
                        budget::MeterLlmUsage {
                            workspace_id: &workspace_id,
                            environment_id: &environment_id,
                            key: runtime_key.as_ref(),
                            route_id: &route_id,
                            provider_connection_id: &connection.id,
                            attempt,
                            gateway_request_id: &attempt_request_id,
                            reservation: budget_reservation.as_ref(),
                            request: &attempt_request,
                            provider_response: &provider_response,
                            provider: provider_name,
                            latency_ms,
                            run_id: run_id.as_deref(),
                        },
                    )
                    .await
                } else {
                    generic_provider_usage(
                        &attempt_request_id,
                        &route_id,
                        &connection.id,
                        attempt,
                        provider_name,
                        latency_ms,
                        &attempt_request,
                        &provider_response,
                    )
                };
                successful = Some((attempt_request, provider_response, usage));
                break;
            }
            Err(error) => {
                let latency_ms = provider_started.elapsed().as_millis() as u64;
                budget::release_llm_budget(
                    &state.app,
                    &workspace_id,
                    &environment_id,
                    run_id.as_deref(),
                    budget_reservation.as_ref(),
                )
                .await;
                let failure_usage = failed_provider_usage(
                    &attempt_request_id,
                    &route_id,
                    &connection.id,
                    attempt,
                    provider_name,
                    latency_ms,
                    &attempt_request,
                    error.code,
                );
                create_gateway_provider_failure_event(
                    &state.app,
                    &workspace_id,
                    &environment_id,
                    &gateway_request_id,
                    &failure_usage,
                    run_id.as_deref(),
                )
                .await;
                let retryable = error.is_retryable();
                let retry_delay = error
                    .retry_after
                    .unwrap_or(std::time::Duration::from_millis(250))
                    .max(std::time::Duration::from_millis(250));
                final_error = Some(error);
                if !retryable || Instant::now() + retry_delay >= attempt_deadline {
                    break;
                }
                if index == 0 {
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }
    let Some((request, provider_response, provider_usage)) = successful else {
        if let Err(error) = state
            .app
            .notification_store
            .enqueue(crate::notifications::EnqueueNotification {
                workspace_id: workspace_id.clone(),
                environment_id: environment_id.clone(),
                agent_id: Some(resolved.route.agent_id.clone()),
                rule_id: None,
                event_kind: tl_core::NotificationEventKind::ProviderTerminalFailure,
                subject_id: gateway_request_id.clone(),
                subject_version: "v1".into(),
                run_id: run_id.clone(),
                payload: serde_json::json!({
                    "title": "Provider request failed",
                    "detail": "All configured provider attempts were exhausted."
                }),
            })
            .await
        {
            tracing::warn!(workspace_id, error = %error, "provider terminal notification enqueue failed");
        }
        finish_gateway_run(
            &state.app,
            &workspace_id,
            &environment_id,
            run_id.as_deref(),
            RunStatus::Failed,
            auto_finalize_run,
        )
        .await;
        return with_run_headers(
            handle_provider_failure(final_error.expect("attempt plan contains a primary")),
            run_id.as_deref(),
            &session,
        );
    };

    let output = provider.extract_output(&provider_response);
    let assistant_event_id = create_gateway_assistant_event(
        &state.app,
        &workspace_id,
        &environment_id,
        &gateway_request_id,
        &output,
        &provider_usage,
        run_id.as_deref(),
    )
    .await;
    let output_decision = match check_gateway_content(
        &state.app,
        GatewayContentCheck {
            workspace_id: &workspace_id,
            environment_id: &environment_id,
            resolved: &resolved,
            phase: "gateway_output_check",
            text_source_id: GATEWAY_OUTPUT_SOURCE_ID,
            proposed_output: &output,
            run_id: run_id.as_deref(),
            run_event_id: assistant_event_id.as_deref(),
            gateway_request_id: &gateway_request_id,
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
                auto_finalize_run,
            )
            .await;
            return with_run_headers(response, run_id.as_deref(), &session);
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

    let mut response = handle_output_enforcement(OutputEnforcement {
        state: &state,
        provider: &provider,
        workspace_id: &workspace_id,
        environment_id: &environment_id,
        request,
        provider_response,
        output_decision,
        run_id: run_id.as_deref(),
        auto_finalize_run,
        wants_stream,
    })
    .await;
    attach_gateway_run_headers(&mut response, run_id.as_deref(), &session);
    response
}

fn generic_provider_usage(
    gateway_request_id: &str,
    route_id: &str,
    provider_connection_id: &str,
    attempt: u32,
    provider: &str,
    latency_ms: u64,
    request: &serde_json::Value,
    response: &serde_json::Value,
) -> RunProviderUsage {
    let usage = response.get("usage");
    let prompt_tokens = usage.and_then(|value| {
        value
            .get("prompt_tokens")
            .or_else(|| value.get("input_tokens"))
            .and_then(serde_json::Value::as_i64)
    });
    let completion_tokens = usage.and_then(|value| {
        value
            .get("completion_tokens")
            .or_else(|| value.get("output_tokens"))
            .and_then(serde_json::Value::as_i64)
    });
    RunProviderUsage {
        gateway_request_id: gateway_request_id.to_string(),
        route_id: route_id.to_string(),
        attempt,
        provider_connection_id: provider_connection_id.to_string(),
        provider: provider.to_string(),
        model: response
            .get("model")
            .or_else(|| request.get("model"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_response_id: response
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        status: "succeeded".to_string(),
        failure_code: None,
        prompt_tokens,
        completion_tokens,
        total_tokens: prompt_tokens
            .zip(completion_tokens)
            .map(|(input, output)| input.saturating_add(output)),
        latency_ms,
        estimated_cost_usd_nanos: None,
        input_rate_usd_per_million_nanos: None,
        output_rate_usd_per_million_nanos: None,
    }
}

fn failed_provider_usage(
    gateway_request_id: &str,
    route_id: &str,
    provider_connection_id: &str,
    attempt: u32,
    provider: &str,
    latency_ms: u64,
    request: &serde_json::Value,
    failure_code: &str,
) -> RunProviderUsage {
    RunProviderUsage {
        gateway_request_id: gateway_request_id.to_string(),
        route_id: route_id.to_string(),
        attempt,
        provider_connection_id: provider_connection_id.to_string(),
        provider: provider.to_string(),
        model: request
            .get("model")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_response_id: None,
        status: "failed".to_string(),
        failure_code: Some(failure_code.to_string()),
        prompt_tokens: None,
        completion_tokens: None,
        total_tokens: None,
        latency_ms,
        estimated_cost_usd_nanos: None,
        input_rate_usd_per_million_nanos: None,
        output_rate_usd_per_million_nanos: None,
    }
}

fn blocked_response<P: GatewayProvider>(
    provider: &P,
    wants_stream: bool,
    request: &serde_json::Value,
    decision: &tl_core::Decision,
    effect: &'static str,
    phase: &'static str,
) -> Response {
    let policy_id = decision
        .triggered_policies
        .first()
        .map(|policy| policy.id.as_str());
    finalize_gateway_response(
        provider,
        wants_stream,
        provider.blocked_response(request),
        Some(EnforcementHeaders {
            effect,
            trace_id: &decision.trace_id,
            phase,
            policy_id,
        }),
    )
}
