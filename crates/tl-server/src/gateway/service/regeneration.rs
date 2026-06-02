use serde_json::{json, Value};
use tl_core::{Decision, GatewayProviderConnection, Verdict};

use crate::AppState;

use super::super::provider::GatewayProvider;
use super::super::store::ResolvedGatewayRoute;
use super::checks::{check_gateway_content, GatewayContentCheck};

#[allow(clippy::too_many_arguments)]
pub(super) async fn check_and_maybe_regenerate<P: GatewayProvider>(
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
    let mut request = initial_request;
    let mut last_decision = initial_decision;

    append_assistant_turn(&mut request, provider.extract_output(&initial_response));
    provider.inject_feedback(&mut request, &last_decision.reason);

    for attempt in 1..=max {
        tracing::info!(
            gateway_request_id,
            attempt,
            max,
            trace_id = %last_decision.trace_id,
            "max_regenerations: re-forwarding to provider"
        );

        let retry_response = match provider
            .forward(http, connection, api_key, request.clone())
            .await
        {
            Ok(response) => response,
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

        let retry_output = provider.extract_output(&retry_response);
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
            Ok(decision) => decision,
            Err(_) => break,
        };

        if retry_decision.verdict == Verdict::Allow {
            tracing::info!(
                gateway_request_id,
                attempt,
                "max_regenerations: output passed on retry; self-healing succeeded"
            );
            return Ok(retry_response);
        }

        last_decision = retry_decision;
        if attempt < max {
            append_assistant_turn(&mut request, provider.extract_output(&retry_response));
            provider.inject_feedback(&mut request, &last_decision.reason);
        }
    }

    tracing::warn!(
        gateway_request_id,
        max,
        "max_regenerations: all attempts exhausted; falling back to safe response"
    );
    Err(last_decision)
}

fn append_assistant_turn(request: &mut Value, content: String) {
    if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
        messages.push(json!({ "role": "assistant", "content": content }));
    }
}
