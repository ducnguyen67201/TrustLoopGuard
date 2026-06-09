use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{
    Action, CheckRequest, Decision, EventKind, GuardEvent, Labels, Origin, Principal,
    ProvenanceMap, Severity, SideEffectClass, Source, ToolMetadata, Verdict,
};

pub trait Normalizer: Send + Sync {
    fn normalize_check_request(
        &self,
        req: &CheckRequest,
        workspace_id: &str,
        environment_id: &str,
    ) -> GuardEvent;
}

pub trait PrincipalResolver: Send + Sync {
    fn resolve(&self, event: &mut GuardEvent);
}

pub trait ToolMetadataProvider: Send + Sync {
    fn get(&self, workspace_id: &str, tool: &str) -> Option<ToolMetadata>;
}

pub trait LabelResolver: Send + Sync {
    fn resolve(&self, event: &mut GuardEvent);
}

pub trait ProvenanceResolver: Send + Sync {
    fn resolve(&self, event: &mut GuardEvent);
}

pub trait Checker: Send + Sync {
    fn check(&self, event: &GuardEvent) -> Vec<CheckerFinding>;
}

#[async_trait]
pub trait SignalProvider: Send + Sync {
    async fn signals(&self, event: &GuardEvent) -> Vec<Signal>;
}

pub trait DecisionComposer: Send + Sync {
    fn compose(
        &self,
        current: Decision,
        findings: &[CheckerFinding],
        signals: &[Signal],
    ) -> Decision;
}

pub trait TracePersister: Send + Sync {
    fn enqueue(&self, event: &GuardEvent, decision: &Decision);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckerFinding {
    pub checker_id: String,
    pub verdict: Option<Verdict>,
    pub reason: String,
    pub violated_rule: Option<String>,
    pub remediation: Option<String>,
    pub source_chain: Vec<String>,
    pub risk_source: Option<String>,
    pub failure_mode: Option<String>,
    pub harm_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub provider_id: String,
    pub message: String,
    pub severity: Option<Severity>,
}

// @depreciate soon: This adapter exists for `/v1/check` compatibility while
// SDK and gateway callers still enter through `CheckRequest`. Replace it once
// direct `GuardEvent` ingestion is the runtime entry point.
pub struct LegacyCheckNormalizer;

impl Normalizer for LegacyCheckNormalizer {
    fn normalize_check_request(
        &self,
        req: &CheckRequest,
        workspace_id: &str,
        environment_id: &str,
    ) -> GuardEvent {
        let sources = if req.input.is_empty() {
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
            context: req.context.clone(),
        }
    }
}

pub struct NoOpPrincipalResolver;

impl PrincipalResolver for NoOpPrincipalResolver {
    fn resolve(&self, _event: &mut GuardEvent) {}
}

pub struct NoOpToolMetadataProvider;

impl ToolMetadataProvider for NoOpToolMetadataProvider {
    fn get(&self, _workspace_id: &str, _tool: &str) -> Option<ToolMetadata> {
        None
    }
}

pub struct NoOpLabelResolver;

impl LabelResolver for NoOpLabelResolver {
    fn resolve(&self, _event: &mut GuardEvent) {}
}

pub struct NoOpProvenanceResolver;

impl ProvenanceResolver for NoOpProvenanceResolver {
    fn resolve(&self, _event: &mut GuardEvent) {}
}

pub struct NoOpChecker;

impl Checker for NoOpChecker {
    fn check(&self, _event: &GuardEvent) -> Vec<CheckerFinding> {
        vec![]
    }
}

pub struct NoOpSignalProvider;

#[async_trait]
impl SignalProvider for NoOpSignalProvider {
    async fn signals(&self, _event: &GuardEvent) -> Vec<Signal> {
        vec![]
    }
}

pub struct NoOpDecisionComposer;

impl DecisionComposer for NoOpDecisionComposer {
    fn compose(
        &self,
        current: Decision,
        _findings: &[CheckerFinding],
        _signals: &[Signal],
    ) -> Decision {
        current
    }
}

pub struct NoOpTracePersister;

impl TracePersister for NoOpTracePersister {
    fn enqueue(&self, _event: &GuardEvent, _decision: &Decision) {}
}

#[derive(Clone)]
pub struct EventPipelineCtx {
    pub normalizer: Arc<dyn Normalizer>,
    pub principal_resolver: Arc<dyn PrincipalResolver>,
    pub tool_metadata: Arc<dyn ToolMetadataProvider>,
    pub label_resolver: Arc<dyn LabelResolver>,
    pub provenance_resolver: Arc<dyn ProvenanceResolver>,
    pub checker: Arc<dyn Checker>,
    pub signals: Arc<dyn SignalProvider>,
    pub composer: Arc<dyn DecisionComposer>,
    pub traces: Arc<dyn TracePersister>,
}

impl EventPipelineCtx {
    pub fn no_op() -> Self {
        Self {
            normalizer: Arc::new(LegacyCheckNormalizer),
            principal_resolver: Arc::new(NoOpPrincipalResolver),
            tool_metadata: Arc::new(NoOpToolMetadataProvider),
            label_resolver: Arc::new(NoOpLabelResolver),
            provenance_resolver: Arc::new(NoOpProvenanceResolver),
            checker: Arc::new(NoOpChecker),
            signals: Arc::new(NoOpSignalProvider),
            composer: Arc::new(NoOpDecisionComposer),
            traces: Arc::new(NoOpTracePersister),
        }
    }

    pub fn normalize_legacy_check(
        &self,
        req: &CheckRequest,
        workspace_id: &str,
        environment_id: &str,
    ) -> GuardEvent {
        self.normalizer
            .normalize_check_request(req, workspace_id, environment_id)
    }

    // @depreciate soon: This method runs the initial no-op stage chain for
    // compatibility proof only. Replace it with the real event pipeline once
    // stage implementations are wired into runtime traffic.
    pub async fn process_noop(
        &self,
        req: &CheckRequest,
        workspace_id: &str,
        environment_id: &str,
        current_decision: Decision,
    ) -> (GuardEvent, Decision) {
        let mut event = self.normalize_legacy_check(req, workspace_id, environment_id);
        self.principal_resolver.resolve(&mut event);
        let _ = self
            .tool_metadata
            .get(&event.principal.workspace_id, &event.action.operation);
        self.label_resolver.resolve(&mut event);
        self.provenance_resolver.resolve(&mut event);

        let findings = self.checker.check(&event);
        let signals = self.signals.signals(&event).await;
        let decision = self.composer.compose(current_decision, &findings, &signals);
        self.traces.enqueue(&event, &decision);
        (event, decision)
    }
}

impl Default for EventPipelineCtx {
    fn default() -> Self {
        Self::no_op()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{Channel, Verdict};

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
        let event = LegacyCheckNormalizer.normalize_check_request(&req(), "ws_1", "production");

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert_eq!(event.action.operation, "output");
        assert_eq!(event.action.parameters["text"], "safe reply");
        assert_eq!(event.action.side_effect, Some(SideEffectClass::None));
    }

    #[test]
    fn normalizer_uses_resolved_workspace_and_environment() {
        let mut request = req();
        request.workspace_id = Some("caller_ws".into());

        let event = LegacyCheckNormalizer.normalize_check_request(&request, "resolved_ws", "dev");

        assert_eq!(event.principal.workspace_id, "resolved_ws");
        assert_eq!(event.principal.environment_id, "dev");
        assert_eq!(event.principal.agent_id, "agent-1");
    }

    #[test]
    fn normalizer_carries_run_and_run_event_ids() {
        let event = LegacyCheckNormalizer.normalize_check_request(&req(), "ws_1", "production");

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
    fn normalizer_does_not_invent_sources_for_empty_input() {
        let mut request = req();
        request.input.clear();

        let event = LegacyCheckNormalizer.normalize_check_request(&request, "ws_1", "production");

        assert!(event.sources.is_empty());
        assert!(event.provenance.is_empty());
    }

    #[test]
    fn event_pipeline_no_op_context_has_all_collaborators() {
        let ctx = EventPipelineCtx::no_op();
        let event = ctx.normalize_legacy_check(&req(), "ws_1", "production");

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert!(ctx.tool_metadata.get("ws_1", "output").is_none());
        assert!(ctx.checker.check(&event).is_empty());
    }

    #[tokio::test]
    async fn no_op_checker_and_signal_provider_return_empty() {
        let ctx = EventPipelineCtx::no_op();
        let event = ctx.normalize_legacy_check(&req(), "ws_1", "production");

        assert!(ctx.checker.check(&event).is_empty());
        assert!(ctx.signals.signals(&event).await.is_empty());
    }

    #[tokio::test]
    async fn no_op_pipeline_returns_current_decision_unchanged() {
        let ctx = EventPipelineCtx::no_op();
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (_event, after) = ctx
            .process_noop(&req(), "ws_1", "production", decision)
            .await;

        assert_eq!(after.verdict, Verdict::Allow);
        assert_eq!(serde_json::to_value(after).unwrap(), before);
    }
}
