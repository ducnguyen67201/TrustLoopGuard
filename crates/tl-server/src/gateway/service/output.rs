use axum::response::Response;
use serde_json::Value;
use tl_core::{AuthorizationEffect, Decision, RunStatus};

use super::super::provider::GatewayProvider;
use super::super::GatewayState;
use super::response::{finalize_gateway_response, EnforcementHeaders};
use super::runs::finish_gateway_run;

pub(super) struct OutputEnforcement<'a, P: GatewayProvider> {
    pub(super) state: &'a GatewayState,
    pub(super) provider: &'a P,
    pub(super) workspace_id: &'a str,
    pub(super) environment_id: &'a str,
    pub(super) request: Value,
    pub(super) provider_response: Value,
    pub(super) output_decision: Decision,
    pub(super) run_id: Option<&'a str>,
    pub(super) auto_finalize_run: bool,
    pub(super) wants_stream: bool,
}

pub(super) async fn handle_output_enforcement<P: GatewayProvider>(
    enforcement: OutputEnforcement<'_, P>,
) -> Response {
    let OutputEnforcement {
        state,
        provider,
        workspace_id,
        environment_id,
        request,
        provider_response,
        output_decision,
        run_id,
        auto_finalize_run,
        wants_stream,
    } = enforcement;

    let response = match output_decision.effect {
        AuthorizationEffect::Permit => {
            finalize_gateway_response(provider, wants_stream, provider_response, None)
        }
        AuthorizationEffect::Transform => match output_decision.safe_output.as_deref() {
            Some(safe_output) => finalize_gateway_response(
                provider,
                wants_stream,
                provider.apply_output_rewrite(provider_response, safe_output),
                Some(enforcement_headers(&output_decision, "transform")),
            ),
            None => {
                output_blocked_response(provider, &request, wants_stream, &output_decision, "deny")
            }
        },
        AuthorizationEffect::Deny => {
            output_blocked_response(provider, &request, wants_stream, &output_decision, "deny")
        }
        AuthorizationEffect::RequireApproval => output_blocked_response(
            provider,
            &request,
            wants_stream,
            &output_decision,
            "require_approval",
        ),
        AuthorizationEffect::Defer => {
            output_blocked_response(provider, &request, wants_stream, &output_decision, "defer")
        }
    };

    finish_completed(
        state,
        workspace_id,
        environment_id,
        run_id,
        auto_finalize_run,
    )
    .await;
    response
}

fn output_blocked_response<P: GatewayProvider>(
    provider: &P,
    request: &Value,
    wants_stream: bool,
    decision: &Decision,
    effect: &'static str,
) -> Response {
    finalize_gateway_response(
        provider,
        wants_stream,
        provider.blocked_response(request),
        Some(enforcement_headers(decision, effect)),
    )
}

fn enforcement_headers<'a>(decision: &'a Decision, effect: &'static str) -> EnforcementHeaders<'a> {
    EnforcementHeaders {
        effect,
        trace_id: &decision.trace_id,
        phase: "output",
        policy_id: decision
            .triggered_policies
            .first()
            .map(|policy| policy.id.as_str()),
    }
}

async fn finish_completed(
    state: &GatewayState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
    auto_finalize_run: bool,
) {
    finish_gateway_run(
        &state.app,
        workspace_id,
        environment_id,
        run_id,
        RunStatus::Completed,
        auto_finalize_run,
    )
    .await;
}
