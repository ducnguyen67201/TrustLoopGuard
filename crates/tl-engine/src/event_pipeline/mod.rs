use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{
    Action, CheckRequest, Decision, EventKind, GuardEvent, Labels, Origin, Principal,
    ProvenanceMap, Severity, SideEffectClass, Source, ToolMetadata, Verdict,
};

/// Raw input accepted by the event pipeline before normalization.
///
/// `LegacyCheck` wraps today's `/v1/check` body (including gateway-proxied
/// traffic, which arrives as a `CheckRequest` with gateway context markers).
/// `Event` is the event-shaped path for high-fidelity collectors such as
/// SDK adapters and, later, the MCP proxy.
pub enum RawInput {
    LegacyCheck(CheckRequest),
    Event(GuardEvent),
}

pub trait Normalizer: Send + Sync {
    fn normalize_check_request(
        &self,
        req: &CheckRequest,
        workspace_id: &str,
        environment_id: &str,
    ) -> GuardEvent;

    /// Normalize any raw input into a `GuardEvent`.
    ///
    /// Event-shaped input passes through with its sources and provenance
    /// preserved verbatim — the pipeline never invents or strips evidence.
    /// Only the principal's workspace/environment are overwritten with the
    /// server-resolved values so callers cannot spoof workspace identity.
    fn normalize(&self, raw: &RawInput, workspace_id: &str, environment_id: &str) -> GuardEvent {
        match raw {
            RawInput::LegacyCheck(req) => {
                self.normalize_check_request(req, workspace_id, environment_id)
            }
            RawInput::Event(event) => {
                let mut event = event.clone();
                event.principal.workspace_id = workspace_id.to_string();
                event.principal.environment_id = environment_id.to_string();
                event
            }
        }
    }
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

impl Normalizer for LegacyCheckNormalizer {
    fn normalize_check_request(
        &self,
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

    /// Run one raw input through the stage chain. With no-op stages
    /// (observe-only), the returned decision is the unchanged
    /// `current_decision`; the returned event carries the collected
    /// evidence for trace enrichment. No stage performs I/O.
    pub async fn process(
        &self,
        raw: &RawInput,
        workspace_id: &str,
        environment_id: &str,
        current_decision: Decision,
    ) -> (GuardEvent, Decision) {
        let mut event = self.normalizer.normalize(raw, workspace_id, environment_id);
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
            .process(
                &RawInput::LegacyCheck(req()),
                "ws_1",
                "production",
                decision,
            )
            .await;

        assert_eq!(after.verdict, Verdict::Allow);
        assert_eq!(serde_json::to_value(after).unwrap(), before);
    }

    fn high_fidelity_event() -> GuardEvent {
        let mut provenance = ProvenanceMap::default();
        provenance.insert("recipient", vec!["src.web".into()]);

        GuardEvent {
            kind: EventKind::ToolCallProposed,
            principal: Principal {
                workspace_id: "client_claimed_ws".into(),
                environment_id: "client_claimed_env".into(),
                agent_id: "agent-1".into(),
                user_id: Some("user-1".into()),
                session_id: Some("sess-1".into()),
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: Action {
                operation: "send_email".into(),
                parameters: serde_json::json!({ "recipient": "a@b.c" }),
                side_effect: Some(SideEffectClass::ExternalCommunication),
            },
            sources: vec![
                Source {
                    id: "src.web".into(),
                    origin: Origin::Web,
                    labels: Labels::default(),
                    kind: Some("web_page".into()),
                },
                Source {
                    id: "src.user".into(),
                    origin: Origin::User,
                    labels: Labels::default(),
                    kind: None,
                },
            ],
            provenance,
            context: serde_json::json!({ "task": "t-1" }),
        }
    }

    #[test]
    fn raw_legacy_input_normalizes_same_as_check_request() {
        let normalizer = LegacyCheckNormalizer;
        let direct = normalizer.normalize_check_request(&req(), "ws_1", "production");
        let via_raw = normalizer.normalize(&RawInput::LegacyCheck(req()), "ws_1", "production");

        assert_eq!(via_raw, direct);
        assert_eq!(via_raw.kind, EventKind::OutputProposed);
    }

    #[test]
    fn raw_event_input_passes_through_with_resolved_principal() {
        let event = LegacyCheckNormalizer.normalize(
            &RawInput::Event(high_fidelity_event()),
            "ws_resolved",
            "staging",
        );

        assert_eq!(event.principal.workspace_id, "ws_resolved");
        assert_eq!(event.principal.environment_id, "staging");
        // High-fidelity evidence is preserved verbatim.
        assert_eq!(event.kind, EventKind::ToolCallProposed);
        assert_eq!(event.sources, high_fidelity_event().sources);
        assert_eq!(event.provenance, high_fidelity_event().provenance);
        assert_eq!(event.principal.user_id.as_deref(), Some("user-1"));
    }

    #[test]
    fn raw_event_input_with_missing_provenance_still_normalizes() {
        let mut sparse = high_fidelity_event();
        sparse.sources.clear();
        sparse.provenance = ProvenanceMap::default();

        let event = LegacyCheckNormalizer.normalize(&RawInput::Event(sparse), "ws_1", "production");

        // Observe-only: missing evidence never blocks and is never invented.
        assert!(event.sources.is_empty());
        assert!(event.provenance.is_empty());
    }

    #[test]
    fn gateway_check_normalizes_with_low_fidelity_sources() {
        let mut request = req();
        request.context = serde_json::json!({ "integration_mode": "gateway" });

        let event = LegacyCheckNormalizer.normalize_check_request(&request, "ws_1", "production");

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

        let event = LegacyCheckNormalizer.normalize_check_request(&request, "ws_1", "production");

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert_eq!(event.action.operation, "output");
        assert_eq!(event.action.parameters["text"], "safe reply");
    }

    #[test]
    fn gateway_check_with_empty_input_only_observes_model_output() {
        let mut request = req();
        request.context = serde_json::json!({ "integration_mode": "gateway" });
        request.input.clear();

        let event = LegacyCheckNormalizer.normalize_check_request(&request, "ws_1", "production");

        let ids: Vec<&str> = event.sources.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["model.output"]);
    }

    #[test]
    fn non_gateway_check_keeps_legacy_source() {
        let event = LegacyCheckNormalizer.normalize_check_request(&req(), "ws_1", "production");

        assert_eq!(event.sources.len(), 1);
        assert_eq!(event.sources[0].id, "legacy.input");
        assert_eq!(event.sources[0].origin, Origin::User);
    }

    #[tokio::test]
    async fn pipeline_process_returns_current_decision_unchanged_for_all_raw_inputs() {
        let ctx = EventPipelineCtx::no_op();
        let inputs = [
            RawInput::LegacyCheck(req()),
            RawInput::Event(high_fidelity_event()),
        ];

        for raw in &inputs {
            let decision = Decision::allow("trace-1");
            let before = serde_json::to_value(&decision).unwrap();

            let (_event, after) = ctx.process(raw, "ws_1", "production", decision).await;

            assert_eq!(serde_json::to_value(after).unwrap(), before);
        }
    }
}
