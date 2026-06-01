use std::time::Instant;

use axum::response::Response;
use serde_json::json;
use tl_core::{Channel, CheckRequest, Decision, RetentionMode};

use crate::{execute_check_request, AppState};

use super::super::normalization::{provider_kind_text, retention_mode_text};
use super::super::store::ResolvedGatewayRoute;

pub(super) struct GatewayContentCheck<'a> {
    pub workspace_id: &'a str,
    pub environment_id: &'a str,
    pub resolved: &'a ResolvedGatewayRoute,
    pub phase: &'a str,
    pub input: &'a str,
    pub proposed_output: &'a str,
    pub run_id: Option<&'a str>,
    pub run_event_id: Option<&'a str>,
}

pub(super) async fn check_gateway_content(
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

    let request = CheckRequest {
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
        request,
        Instant::now(),
    )
    .await
}

pub(super) struct GatewayDecisionLog<'a> {
    pub workspace_id: &'a str,
    pub environment_id: &'a str,
    pub route_id: &'a str,
    pub run_id: Option<&'a str>,
    pub phase: &'a str,
    pub decision: &'a Decision,
    pub content_chars: usize,
    pub wants_stream: bool,
}

pub(super) fn log_gateway_decision(fields: GatewayDecisionLog<'_>) {
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
