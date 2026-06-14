mod checks;
mod output;
mod regeneration;
mod request;
mod response;
mod runs;

use axum::{
    http::{HeaderMap, StatusCode},
    response::Response,
};
use bytes::Bytes;
use tl_core::{GatewayInputAction, GatewayProviderKind, RunStatus, Verdict};
use uuid::Uuid;

use crate::policies::workspace_id_from_headers;

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
    let wants_stream =
        match prepare_streaming_request(&provider, &resolved.enforcement_profile, &mut request) {
            Ok(wants_stream) => wants_stream,
            Err(response) => return response,
        };

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
        resolved: &resolved,
        provider_api_key: &provider_api_key,
        workspace_id: &workspace_id,
        environment_id: &environment_id,
        request,
        provider_response,
        output_decision,
        gateway_request_id: &gateway_request_id,
        run_id: run_id.as_deref(),
        run_event_id: run_event_id.as_deref(),
        wants_stream,
    })
    .await
}
