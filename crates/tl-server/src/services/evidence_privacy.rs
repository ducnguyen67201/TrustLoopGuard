use tl_core::{
    AgentEvaluationProfile, ContentCaptureMode, DataHandlingMode, Decision, GuardEvent,
    RedactionStatus,
};

pub(crate) fn project_evidence_for_persistence(
    workspace_mode: DataHandlingMode,
    mut decision: Decision,
    mut event: GuardEvent,
    profile: Option<&AgentEvaluationProfile>,
) -> (Decision, Option<GuardEvent>) {
    let profile = profile.filter(|profile| profile.enabled);
    let requested_mode = profile
        .map(|profile| profile.content_mode)
        .unwrap_or(ContentCaptureMode::MetadataOnly);
    let effective_mode = effective_content_mode(workspace_mode, requested_mode, profile.is_some());
    let verified_redacted = effective_mode == ContentCaptureMode::Redacted
        && decision.redaction.as_ref().is_some_and(|redaction| {
            redaction.status == RedactionStatus::Applied
                && redaction.input_redacted
                && redaction.proposed_output_redacted
                && redaction.context_redacted
        });
    let may_persist_body = match workspace_mode {
        DataHandlingMode::RawAllowed | DataHandlingMode::PrivateDeployment => profile.is_none(),
        DataHandlingMode::RedactedOnly => verified_redacted,
        DataHandlingMode::NoBodyRetention => false,
    } || verified_redacted;
    if may_persist_body {
        return (decision, Some(event));
    }

    decision.safe_output = None;
    decision.checked_input_excerpt = None;
    decision.checked_output_excerpt = None;
    event.action.parameters = serde_json::Value::Null;
    event.sources.clear();
    event.provenance = Default::default();
    event.context = serde_json::Value::Null;
    (decision, Some(event))
}

pub(crate) fn may_persist_gateway_body(workspace_mode: DataHandlingMode) -> bool {
    matches!(
        workspace_mode,
        DataHandlingMode::RawAllowed | DataHandlingMode::PrivateDeployment
    )
}

fn effective_content_mode(
    workspace_mode: DataHandlingMode,
    profile_mode: ContentCaptureMode,
    has_profile: bool,
) -> ContentCaptureMode {
    match workspace_mode {
        DataHandlingMode::NoBodyRetention => ContentCaptureMode::MetadataOnly,
        DataHandlingMode::RedactedOnly => match profile_mode {
            ContentCaptureMode::EncryptedArtifactRef => ContentCaptureMode::EncryptedArtifactRef,
            _ => ContentCaptureMode::Redacted,
        },
        DataHandlingMode::RawAllowed | DataHandlingMode::PrivateDeployment if !has_profile => {
            ContentCaptureMode::Redacted
        }
        DataHandlingMode::RawAllowed | DataHandlingMode::PrivateDeployment => profile_mode,
    }
}

#[cfg(test)]
mod tests {
    use super::{may_persist_gateway_body, project_evidence_for_persistence};
    use tl_core::{
        Action, DataHandlingMode, Decision, EventKind, GuardEvent, Principal, ProvenanceMap,
    };

    fn event_with_secret() -> GuardEvent {
        GuardEvent {
            kind: EventKind::ToolCallProposed,
            principal: Principal {
                workspace_id: "workspace-1".into(),
                environment_id: "production".into(),
                agent_id: "agent-1".into(),
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: Action {
                operation: "send".into(),
                parameters: serde_json::json!({"secret": "canary-secret"}),
                side_effect: None,
                invocation_id: None,
                tool_identity: None,
                authorization: None,
            },
            sources: vec![],
            provenance: ProvenanceMap::default(),
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::json!({"secret": "canary-secret"}),
        }
    }

    #[test]
    fn gateway_body_retention_obeys_workspace_mode() {
        assert!(may_persist_gateway_body(DataHandlingMode::RawAllowed));
        assert!(may_persist_gateway_body(
            DataHandlingMode::PrivateDeployment
        ));
        assert!(!may_persist_gateway_body(DataHandlingMode::RedactedOnly));
        assert!(!may_persist_gateway_body(DataHandlingMode::NoBodyRetention));
    }

    #[test]
    fn no_body_retention_removes_runtime_content_before_persistence() {
        let mut decision = Decision::allow("trace-1");
        decision.safe_output = Some("canary-secret".into());
        decision.checked_input_excerpt = Some("canary-secret".into());
        let (decision, event) = project_evidence_for_persistence(
            DataHandlingMode::NoBodyRetention,
            decision,
            event_with_secret(),
            None,
        );
        let event = event.unwrap();
        assert!(decision.safe_output.is_none());
        assert!(decision.checked_input_excerpt.is_none());
        assert!(event.action.parameters.is_null());
        assert!(event.context.is_null());
    }

    #[test]
    fn raw_allowed_without_evaluation_profile_preserves_runtime_content() {
        let event = event_with_secret();
        let (_, projected) = project_evidence_for_persistence(
            DataHandlingMode::RawAllowed,
            Decision::allow("trace-1"),
            event.clone(),
            None,
        );
        assert_eq!(projected, Some(event));
    }
}
