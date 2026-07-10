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
use tl_core::{GatewayProviderKind, RunStatus, Verdict};
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
    create_gateway_run, create_gateway_turn_event, finish_gateway_run, gateway_run_external_id,
};

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

    let mut request = match parse_provider_request(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let wants_stream = prepare_streaming_request(&provider, &mut request);

    let metered = expected_kind == GatewayProviderKind::OpenaiCompatible;

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
            text_source_id: GATEWAY_INPUT_SOURCE_ID,
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

    match input_decision.verdict {
        Verdict::Allow => {}
        Verdict::Rewrite => {
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
                )
                .await;
                return response;
            }
        }
        Verdict::Block | Verdict::Escalate => {
            let verdict = if input_decision.verdict == Verdict::Escalate {
                "escalated"
            } else {
                "blocked"
            };
            let response = blocked_response(
                &provider,
                wants_stream,
                &request,
                &input_decision,
                verdict,
                "input",
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

    // Reserve the maximum possible provider cost after input policy
    // rewrites but before provider traffic. OpenAI-compatible requests
    // without a matching budget continue to meter after the response.
    if metered && request.get("model").is_none() {
        request["model"] = serde_json::json!(resolved.provider_connection.default_model);
    }
    let budget_reservation = if metered {
        match budget::reserve_llm_budget(
            &state.app,
            &workspace_id,
            &environment_id,
            runtime_key.as_ref(),
            &gateway_request_id,
            &request,
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
                    RunStatus::Completed,
                )
                .await;
                return response;
            }
        }
    } else {
        None
    };

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
            budget::release_llm_budget(&state.app, &workspace_id, budget_reservation.as_ref())
                .await;
            finish_gateway_run(
                &state.app,
                &workspace_id,
                &environment_id,
                run_id.as_deref(),
                RunStatus::Failed,
            )
            .await;
            return handle_provider_failure(error);
        }
    };

    if metered {
        // Meter the buffered upstream response (usage is always
        // present; SSE is synthesized from it). Never fails the
        // response.
        budget::meter_llm_usage(
            &state.app,
            budget::MeterLlmUsage {
                workspace_id: &workspace_id,
                environment_id: &environment_id,
                key: runtime_key.as_ref(),
                route_id: &route_id,
                gateway_request_id: &gateway_request_id,
                reservation: budget_reservation.as_ref(),
                request: &request,
                provider_response: &provider_response,
            },
        )
        .await;
    }

    let output = provider.extract_output(&provider_response);
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

    handle_output_enforcement(OutputEnforcement {
        state: &state,
        provider: &provider,
        workspace_id: &workspace_id,
        environment_id: &environment_id,
        request,
        provider_response,
        output_decision,
        run_id: run_id.as_deref(),
        wants_stream,
    })
    .await
}

fn blocked_response<P: GatewayProvider>(
    provider: &P,
    wants_stream: bool,
    request: &serde_json::Value,
    decision: &tl_core::Decision,
    verdict: &'static str,
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
            verdict,
            trace_id: &decision.trace_id,
            phase,
            policy_id,
        }),
    )
}
