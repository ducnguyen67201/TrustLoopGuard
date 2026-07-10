use axum::response::Response;
use serde_json::Value;
use tl_core::{Decision, RunStatus, Verdict};

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
        wants_stream,
    } = enforcement;

    let response = match output_decision.verdict {
        Verdict::Allow => {
            finalize_gateway_response(provider, wants_stream, provider_response, None)
        }
        Verdict::Rewrite => match output_decision.safe_output.as_deref() {
            Some(safe_output) => finalize_gateway_response(
                provider,
                wants_stream,
                provider.apply_output_rewrite(provider_response, safe_output),
                Some(enforcement_headers(&output_decision, "rewrite")),
            ),
            None => output_blocked_response(
                provider,
                &request,
                wants_stream,
                &output_decision,
                "blocked",
            ),
        },
        Verdict::Block => output_blocked_response(
            provider,
            &request,
            wants_stream,
            &output_decision,
            "blocked",
        ),
        Verdict::Escalate => output_blocked_response(
            provider,
            &request,
            wants_stream,
            &output_decision,
            "escalated",
        ),
    };

    finish_completed(state, workspace_id, environment_id, run_id).await;
    response
}

fn output_blocked_response<P: GatewayProvider>(
    provider: &P,
    request: &Value,
    wants_stream: bool,
    decision: &Decision,
    verdict: &'static str,
) -> Response {
    finalize_gateway_response(
        provider,
        wants_stream,
        provider.blocked_response(request),
        Some(enforcement_headers(decision, verdict)),
    )
}

fn enforcement_headers<'a>(
    decision: &'a Decision,
    verdict: &'static str,
) -> EnforcementHeaders<'a> {
    EnforcementHeaders {
        verdict,
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
