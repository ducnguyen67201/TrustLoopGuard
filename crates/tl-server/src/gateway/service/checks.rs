use std::time::Instant;

use axum::response::Response;
use serde_json::json;
use tl_core::{
    Action, Decision, EventKind, GuardEvent, Labels, Origin, Principal, ProvenanceMap,
    RetentionMode, SideEffectClass, Source, Verdict,
};

use crate::{services::event_service::execute_event_submission, AppState};

use super::super::normalization::{provider_kind_text, retention_mode_text};
use super::super::store::ResolvedGatewayRoute;

pub(super) const GATEWAY_INPUT_SOURCE_ID: &str = "input.observed";
pub(super) const GATEWAY_OUTPUT_SOURCE_ID: &str = "model.output";

pub(super) struct GatewayContentCheck<'a> {
    pub workspace_id: &'a str,
    pub environment_id: &'a str,
    pub resolved: &'a ResolvedGatewayRoute,
    pub phase: &'a str,
    pub text_source_id: &'a str,
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
        "channel": "chat",
        "domain": check.phase,
    });
    if check.resolved.enforcement_profile.retention_mode == RetentionMode::MetadataOnly {
        context["body_retention"] = json!("omitted");
    }

    let mut provenance = ProvenanceMap::default();
    provenance.insert("text", vec![check.text_source_id.to_string()]);
    let event = GuardEvent {
        kind: EventKind::OutputProposed,
        principal: Principal {
            workspace_id: check.workspace_id.to_string(),
            environment_id: check.environment_id.to_string(),
            agent_id: check.resolved.route.agent_id.clone(),
            user_id: None,
            session_id: None,
            task_id: None,
            run_id: check.run_id.map(str::to_string),
            run_event_id: check.run_event_id.map(str::to_string),
        },
        action: Action {
            operation: "output".to_string(),
            parameters: json!({ "text": check.proposed_output }),
            side_effect: Some(SideEffectClass::None),
        },
        sources: vec![
            Source {
                id: GATEWAY_INPUT_SOURCE_ID.to_string(),
                origin: Origin::Unknown,
                labels: Labels::default(),
                kind: Some("gateway.input".to_string()),
            },
            Source {
                id: GATEWAY_OUTPUT_SOURCE_ID.to_string(),
                origin: Origin::Unknown,
                labels: Labels::default(),
                kind: Some("gateway.output".to_string()),
            },
        ],
        provenance,
        resolution: None,
        label_resolution: None,
        checks: vec![],
        signals: vec![],
        context,
    };

    let decision = execute_event_submission(
        state,
        check.workspace_id,
        check.environment_id,
        event,
        Instant::now(),
    )
    .await?;

    if let Some(run_id) = check.run_id {
        if let Err(e) = state
            .run_store
            .record_check(
                check.workspace_id,
                check.environment_id,
                run_id,
                verdict_name(decision.verdict),
                decision.latency_ms as i32,
            )
            .await
        {
            tracing::warn!(run_id, error = %e, "could not update run stats");
        }
    }

    Ok(decision)
}

fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "allow",
        Verdict::Rewrite => "rewrite",
        Verdict::Block => "block",
        Verdict::Escalate => "escalate",
    }
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
