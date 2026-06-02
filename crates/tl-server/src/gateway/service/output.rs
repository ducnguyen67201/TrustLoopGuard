use axum::response::Response;
use serde_json::Value;
use tl_core::{Decision, GatewayOutputAction, RunStatus, Verdict};

use super::super::provider::GatewayProvider;
use super::super::{GatewayState, ResolvedGatewayRoute};
use super::regeneration::check_and_maybe_regenerate;
use super::response::{finalize_gateway_response, EnforcementHeaders};
use super::runs::finish_gateway_run;

pub(super) struct OutputEnforcement<'a, P: GatewayProvider> {
    pub(super) state: &'a GatewayState,
    pub(super) provider: &'a P,
    pub(super) resolved: &'a ResolvedGatewayRoute,
    pub(super) provider_api_key: &'a str,
    pub(super) workspace_id: &'a str,
    pub(super) environment_id: &'a str,
    pub(super) request: Value,
    pub(super) provider_response: Value,
    pub(super) output_decision: Decision,
    pub(super) gateway_request_id: &'a str,
    pub(super) run_id: Option<&'a str>,
    pub(super) run_event_id: Option<&'a str>,
    pub(super) wants_stream: bool,
}

pub(super) async fn handle_output_enforcement<P: GatewayProvider>(
    enforcement: OutputEnforcement<'_, P>,
) -> Response {
    let OutputEnforcement {
        state,
        provider,
        resolved,
        provider_api_key,
        workspace_id,
        environment_id,
        request,
        provider_response,
        output_decision,
        gateway_request_id,
        run_id,
        run_event_id,
        wants_stream,
    } = enforcement;

    if output_decision.verdict == Verdict::Allow {
        finish_completed(state, workspace_id, environment_id, run_id).await;
        return finalize_gateway_response(provider, wants_stream, provider_response, None);
    }

    let response = match resolved.enforcement_profile.output_action {
        GatewayOutputAction::Allow => {
            finalize_gateway_response(provider, wants_stream, provider_response, None)
        }
        GatewayOutputAction::Block => output_safe_response(
            provider,
            resolved,
            &request,
            wants_stream,
            &output_decision,
            "blocked",
        ),
        GatewayOutputAction::Escalate => output_safe_response(
            provider,
            resolved,
            &request,
            wants_stream,
            &output_decision,
            "escalated",
        ),
        GatewayOutputAction::Rewrite => {
            return handle_rewrite_output(OutputEnforcement {
                state,
                provider,
                resolved,
                provider_api_key,
                workspace_id,
                environment_id,
                request,
                provider_response,
                output_decision,
                gateway_request_id,
                run_id,
                run_event_id,
                wants_stream,
            })
            .await;
        }
    };

    finish_completed(state, workspace_id, environment_id, run_id).await;
    response
}

async fn handle_rewrite_output<P: GatewayProvider>(
    enforcement: OutputEnforcement<'_, P>,
) -> Response {
    let OutputEnforcement {
        state,
        provider,
        resolved,
        provider_api_key,
        workspace_id,
        environment_id,
        request,
        provider_response,
        output_decision,
        gateway_request_id,
        run_id,
        run_event_id,
        wants_stream,
    } = enforcement;

    if let Some(safe_out) = output_decision.safe_output.as_deref() {
        let response = finalize_gateway_response(
            provider,
            wants_stream,
            provider.apply_output_rewrite(provider_response, safe_out),
            None,
        );
        finish_completed(state, workspace_id, environment_id, run_id).await;
        return response;
    }

    if resolved.enforcement_profile.max_regenerations > 0 {
        return handle_regeneration(OutputEnforcement {
            state,
            provider,
            resolved,
            provider_api_key,
            workspace_id,
            environment_id,
            request,
            provider_response,
            output_decision,
            gateway_request_id,
            run_id,
            run_event_id,
            wants_stream,
        })
        .await;
    }

    let response = output_safe_response(
        provider,
        resolved,
        &request,
        wants_stream,
        &output_decision,
        "blocked",
    );
    finish_completed(state, workspace_id, environment_id, run_id).await;
    response
}

async fn handle_regeneration<P: GatewayProvider>(
    enforcement: OutputEnforcement<'_, P>,
) -> Response {
    let OutputEnforcement {
        state,
        provider,
        resolved,
        provider_api_key,
        workspace_id,
        environment_id,
        request,
        provider_response,
        output_decision,
        gateway_request_id,
        run_id,
        run_event_id,
        wants_stream,
    } = enforcement;

    match check_and_maybe_regenerate(
        &state.app,
        provider,
        &state.http,
        &resolved.provider_connection,
        provider_api_key,
        workspace_id,
        environment_id,
        resolved,
        request.clone(),
        provider_response,
        output_decision,
        gateway_request_id,
        run_id,
        run_event_id,
    )
    .await
    {
        Ok(clean) => {
            finish_completed(state, workspace_id, environment_id, run_id).await;
            finalize_gateway_response(provider, wants_stream, clean, None)
        }
        Err(final_decision) => {
            let response = output_safe_response(
                provider,
                resolved,
                &request,
                wants_stream,
                &final_decision,
                "blocked",
            );
            finish_completed(state, workspace_id, environment_id, run_id).await;
            response
        }
    }
}

fn output_safe_response<P: GatewayProvider>(
    provider: &P,
    resolved: &ResolvedGatewayRoute,
    request: &Value,
    wants_stream: bool,
    decision: &Decision,
    verdict: &'static str,
) -> Response {
    let policy_id = decision
        .triggered_policies
        .first()
        .map(|policy| policy.id.as_str());
    finalize_gateway_response(
        provider,
        wants_stream,
        provider.safe_response(request, &resolved.enforcement_profile),
        Some(EnforcementHeaders {
            verdict,
            trace_id: &decision.trace_id,
            phase: "output",
            policy_id,
        }),
    )
}

async fn finish_completed(
    state: &GatewayState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
) {
    finish_gateway_run(
        &state.app,
        workspace_id,
        environment_id,
        run_id,
        RunStatus::Completed,
    )
    .await;
}
