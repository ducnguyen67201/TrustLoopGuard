use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{Decision, GuardEvent, Severity, ToolMetadata, Verdict};

pub mod legacy_adapter;

pub use legacy_adapter::legacy_check_to_event;

/// The pipeline contract is `GuardEvent`-only. Collectors (SDK adapters,
/// the MCP proxy, the legacy `/v1/check` adapter) translate their raw
/// traffic into a `GuardEvent` before entering the pipeline.
pub trait Normalizer: Send + Sync {
    /// Normalize an event's structure before the stage chain runs.
    ///
    /// The event passes through with its sources and provenance preserved
    /// verbatim — the pipeline never invents or strips evidence. Workspace
    /// and environment identity are deliberately *not* a normalizer concern:
    /// `EventPipelineCtx::process` always overwrites them with the
    /// server-resolved values, so no `Normalizer` impl can skip that step
    /// and reopen workspace spoofing.
    fn normalize_event(&self, event: GuardEvent) -> GuardEvent {
        event
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

pub struct NoOpNormalizer;

impl Normalizer for NoOpNormalizer {}

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
            normalizer: Arc::new(NoOpNormalizer),
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

    /// Run one event through the stage chain. With no-op stages
    /// (observe-only), the returned decision is the unchanged
    /// `current_decision`; the returned event carries the collected
    /// evidence for trace enrichment. No stage performs I/O.
    ///
    /// The pipeline always overwrites the event principal's
    /// workspace/environment with the server-resolved values — after
    /// normalization and independent of the `Normalizer` impl — so callers
    /// cannot spoof workspace identity regardless of how the event was
    /// collected.
    pub async fn process(
        &self,
        event: GuardEvent,
        workspace_id: &str,
        environment_id: &str,
        current_decision: Decision,
    ) -> (GuardEvent, Decision) {
        let mut event = self.normalizer.normalize_event(event);
        // Identity is a pipeline invariant, not a normalizer concern:
        // overwrite with server-resolved values so no Normalizer impl can
        // skip it and reopen workspace spoofing.
        event.principal.workspace_id = workspace_id.to_string();
        event.principal.environment_id = environment_id.to_string();
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
    use tl_core::{
        Action, EventKind, Labels, Origin, Principal, ProvenanceMap, SideEffectClass, Source,
    };

    fn output_event() -> GuardEvent {
        GuardEvent {
            kind: EventKind::OutputProposed,
            principal: Principal {
                workspace_id: "ws_1".into(),
                environment_id: "production".into(),
                agent_id: "agent-1".into(),
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: Action {
                operation: "output".into(),
                parameters: serde_json::json!({ "text": "safe reply" }),
                side_effect: Some(SideEffectClass::None),
            },
            sources: vec![],
            provenance: ProvenanceMap::default(),
            context: serde_json::Value::Null,
        }
    }

    #[test]
    fn event_pipeline_no_op_context_has_all_collaborators() {
        let ctx = EventPipelineCtx::no_op();
        let event = output_event();

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert!(ctx.tool_metadata.get("ws_1", "output").is_none());
        assert!(ctx.checker.check(&event).is_empty());
    }

    #[tokio::test]
    async fn no_op_checker_and_signal_provider_return_empty() {
        let ctx = EventPipelineCtx::no_op();
        let event = output_event();

        assert!(ctx.checker.check(&event).is_empty());
        assert!(ctx.signals.signals(&event).await.is_empty());
    }

    #[tokio::test]
    async fn no_op_pipeline_returns_current_decision_unchanged() {
        let ctx = EventPipelineCtx::no_op();
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (_event, after) = ctx
            .process(output_event(), "ws_1", "production", decision)
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
    fn normalize_event_preserves_evidence_verbatim() {
        let event = NoOpNormalizer.normalize_event(high_fidelity_event());

        // The normalizer never touches identity — that is the pipeline's
        // job (see pipeline_process_overwrites_spoofed_principal_identity).
        assert_eq!(event.principal.workspace_id, "client_claimed_ws");
        assert_eq!(event.principal.environment_id, "client_claimed_env");
        // High-fidelity evidence is preserved verbatim.
        assert_eq!(event.kind, EventKind::ToolCallProposed);
        assert_eq!(event.sources, high_fidelity_event().sources);
        assert_eq!(event.provenance, high_fidelity_event().provenance);
        assert_eq!(event.principal.user_id.as_deref(), Some("user-1"));
    }

    #[test]
    fn normalize_event_with_missing_provenance_still_normalizes() {
        let mut sparse = high_fidelity_event();
        sparse.sources.clear();
        sparse.provenance = ProvenanceMap::default();

        let event = NoOpNormalizer.normalize_event(sparse);

        // Observe-only: missing evidence never blocks and is never invented.
        assert!(event.sources.is_empty());
        assert!(event.provenance.is_empty());
    }

    #[tokio::test]
    async fn pipeline_process_returns_current_decision_unchanged_for_all_events() {
        let ctx = EventPipelineCtx::no_op();
        let events = [output_event(), high_fidelity_event()];

        for event in events {
            let decision = Decision::allow("trace-1");
            let before = serde_json::to_value(&decision).unwrap();

            let (_event, after) = ctx.process(event, "ws_1", "production", decision).await;

            assert_eq!(serde_json::to_value(after).unwrap(), before);
        }
    }

    #[tokio::test]
    async fn pipeline_process_overwrites_spoofed_principal_identity() {
        let ctx = EventPipelineCtx::no_op();

        let (event, _decision) = ctx
            .process(
                high_fidelity_event(),
                "ws_resolved",
                "production",
                Decision::allow("trace-1"),
            )
            .await;

        // The client-claimed workspace/environment never survive processing.
        assert_eq!(event.principal.workspace_id, "ws_resolved");
        assert_eq!(event.principal.environment_id, "production");
    }
}
