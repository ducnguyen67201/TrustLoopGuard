//! Legacy `/v1/check` compatibility adapter.
//!
//! @depreciate soon: this entire module exists only while SDK and gateway
//! callers still enter through `CheckRequest`. The event pipeline contract
//! is `GuardEvent`-only; delete this module once direct `GuardEvent`
//! ingestion is the only runtime entry point.

use tl_core::{
    Action, CheckRequest, EventKind, GuardEvent, Labels, Origin, Principal, ProvenanceMap,
    SideEffectClass, Source,
};

/// Gateway-proxied checks carry this context marker (set by the gateway
/// service). Capture fidelity is low: the gateway sees model I/O but cannot
/// prove actual execution or parameter provenance, so its sources stay
/// `Origin::Unknown` with default labels.
///
/// SECURITY INVARIANT: `context` is caller-supplied, so this marker is
/// spoofable and must only ever select *lower*-fidelity evidence labeling
/// (`Origin::Unknown`, default labels). It must never gate enforcement,
/// elevate trust, or feed authorization. When an enforcement phase needs
/// real gateway identity, derive it from server-authenticated principal
/// context, not from the request body.
fn is_gateway_context(context: &serde_json::Value) -> bool {
    context
        .as_object()
        .and_then(|object| object.get("integration_mode"))
        .and_then(serde_json::Value::as_str)
        == Some("gateway")
}

fn gateway_sources(req: &CheckRequest) -> Vec<Source> {
    let mut sources = Vec::with_capacity(2);
    if !req.input.is_empty() {
        sources.push(Source {
            id: "input.observed".into(),
            origin: Origin::Unknown,
            labels: Labels::default(),
            kind: Some("gateway.input".into()),
        });
    }
    sources.push(Source {
        id: "model.output".into(),
        origin: Origin::Unknown,
        labels: Labels::default(),
        kind: Some("gateway.output".into()),
    });
    sources
}

/// Map a legacy `CheckRequest` into the canonical `GuardEvent` before it
/// enters the event pipeline.
pub fn legacy_check_to_event(
    req: &CheckRequest,
    workspace_id: &str,
    environment_id: &str,
) -> GuardEvent {
    let sources = if is_gateway_context(&req.context) {
        gateway_sources(req)
    } else if req.input.is_empty() {
        vec![]
    } else {
        vec![Source {
            id: "legacy.input".into(),
            origin: Origin::User,
            labels: Labels::default(),
            kind: Some("check_request.input".into()),
        }]
    };

    GuardEvent {
        kind: EventKind::OutputProposed,
        principal: Principal {
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            agent_id: req.agent_id.clone(),
            user_id: None,
            session_id: None,
            task_id: None,
            run_id: req.run_id.clone(),
            run_event_id: req.run_event_id.clone(),
        },
        action: Action {
            operation: "output".into(),
            parameters: serde_json::json!({ "text": req.proposed_output }),
            side_effect: Some(SideEffectClass::None),
        },
        sources,
        provenance: ProvenanceMap::default(),
        resolution: None,
        label_resolution: None,
        checks: vec![],
        context: req.context.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::Channel;

    fn req() -> CheckRequest {
        CheckRequest {
            workspace_id: None,
            run_id: Some("018f2222-2222-7222-8222-222222222222".into()),
            run_event_id: Some("018f3333-3333-7333-8333-333333333333".into()),
            run_event: None,
            agent_id: "agent-1".into(),
            channel: Channel::Chat,
            input: "hello".into(),
            proposed_output: "safe reply".into(),
            domain: None,
            policies: vec![],
            context: serde_json::json!({ "session": "s-1" }),
            trace_id: None,
            redaction: None,
        }
    }

    #[test]
    fn legacy_check_request_normalizes_to_output_proposed() {
        let event = legacy_check_to_event(&req(), "ws_1", "production");

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert_eq!(event.action.operation, "output");
        assert_eq!(event.action.parameters["text"], "safe reply");
        assert_eq!(event.action.side_effect, Some(SideEffectClass::None));
    }

    #[test]
    fn legacy_adapter_uses_resolved_workspace_and_environment() {
        let mut request = req();
        request.workspace_id = Some("caller_ws".into());

        let event = legacy_check_to_event(&request, "resolved_ws", "dev");

        assert_eq!(event.principal.workspace_id, "resolved_ws");
        assert_eq!(event.principal.environment_id, "dev");
        assert_eq!(event.principal.agent_id, "agent-1");
    }

    #[test]
    fn legacy_adapter_carries_run_and_run_event_ids() {
        let event = legacy_check_to_event(&req(), "ws_1", "production");

        assert_eq!(
            event.principal.run_id.as_deref(),
            Some("018f2222-2222-7222-8222-222222222222")
        );
        assert_eq!(
            event.principal.run_event_id.as_deref(),
            Some("018f3333-3333-7333-8333-333333333333")
        );
    }

    #[test]
    fn legacy_adapter_does_not_invent_sources_for_empty_input() {
        let mut request = req();
        request.input.clear();

        let event = legacy_check_to_event(&request, "ws_1", "production");

        assert!(event.sources.is_empty());
        assert!(event.provenance.is_empty());
    }

    #[test]
    fn gateway_check_normalizes_with_low_fidelity_sources() {
        let mut request = req();
        request.context = serde_json::json!({ "integration_mode": "gateway" });

        let event = legacy_check_to_event(&request, "ws_1", "production");

        let ids: Vec<&str> = event.sources.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["input.observed", "model.output"]);
        for source in &event.sources {
            assert_eq!(source.origin, Origin::Unknown);
            assert_eq!(source.labels, Labels::default());
        }
        assert_eq!(event.sources[0].kind.as_deref(), Some("gateway.input"));
        assert_eq!(event.sources[1].kind.as_deref(), Some("gateway.output"));
    }

    #[test]
    fn gateway_check_keeps_output_proposed_kind_and_action() {
        let mut request = req();
        request.context = serde_json::json!({ "integration_mode": "gateway" });

        let event = legacy_check_to_event(&request, "ws_1", "production");

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert_eq!(event.action.operation, "output");
        assert_eq!(event.action.parameters["text"], "safe reply");
    }

    #[test]
    fn gateway_check_with_empty_input_only_observes_model_output() {
        let mut request = req();
        request.context = serde_json::json!({ "integration_mode": "gateway" });
        request.input.clear();

        let event = legacy_check_to_event(&request, "ws_1", "production");

        let ids: Vec<&str> = event.sources.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["model.output"]);
    }

    #[test]
    fn non_gateway_check_keeps_legacy_source() {
        let event = legacy_check_to_event(&req(), "ws_1", "production");

        assert_eq!(event.sources.len(), 1);
        assert_eq!(event.sources[0].id, "legacy.input");
        assert_eq!(event.sources[0].origin, Origin::User);
    }
}
