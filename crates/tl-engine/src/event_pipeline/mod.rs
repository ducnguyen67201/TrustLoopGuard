use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{Decision, GuardEvent, Severity, ToolMetadata, ToolResolution, Verdict};

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

/// Marker error: the registry could not be consulted (e.g. storage
/// failure). Implementations log the details; the pipeline records
/// `resolution_failed` evidence, leaves the event otherwise untouched,
/// and never lets the failure reach the decision (fail open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolMetadataUnavailable;

/// Runtime lookup seam for the workspace tool-metadata registry. Async so
/// implementations can read through a cache with a storage fallback; the
/// pipeline treats the result as evidence only.
///
/// `Ok(None)` means the tool is genuinely unregistered (or disabled) —
/// the conservative default. `Err(ToolMetadataUnavailable)` means the
/// registry itself was unreachable; the two are recorded as distinct
/// trace evidence so a storage outage never masquerades as absence.
#[async_trait]
pub trait ToolMetadataProvider: Send + Sync {
    async fn get(
        &self,
        workspace_id: &str,
        tool: &str,
    ) -> Result<Option<ToolMetadata>, ToolMetadataUnavailable>;
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

#[async_trait]
impl ToolMetadataProvider for NoOpToolMetadataProvider {
    async fn get(
        &self,
        _workspace_id: &str,
        _tool: &str,
    ) -> Result<Option<ToolMetadata>, ToolMetadataUnavailable> {
        Ok(None)
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
        // Action resolution: attach registry semantics as evidence. The
        // registry side effect is authoritative when the tool is registered
        // and overwrites the collector-claimed value — checkers and signal
        // providers below see the resolved event, so any future non-no-op
        // checker observes the registry value on resolution and the claimed
        // value when resolution did not succeed. Today every checker is a
        // no-op, so the decision never depends on this evidence.
        match self
            .tool_metadata
            .get(&event.principal.workspace_id, &event.action.operation)
            .await
        {
            Ok(Some(metadata)) => {
                event.action.side_effect = Some(metadata.side_effect);
                event.resolution = Some(ToolResolution::Resolved { metadata });
            }
            Ok(None) => {
                event.resolution = Some(ToolResolution::Unregistered);
            }
            Err(ToolMetadataUnavailable) => {
                event.resolution = Some(ToolResolution::ResolutionFailed);
            }
        }
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
            resolution: None,
            context: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn event_pipeline_no_op_context_has_all_collaborators() {
        let ctx = EventPipelineCtx::no_op();
        let event = output_event();

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert_eq!(ctx.tool_metadata.get("ws_1", "output").await, Ok(None));
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
            resolution: None,
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

    use std::collections::HashMap;
    use tl_core::{AllowedSource, ParamRole, ParamSpec};

    struct StubToolMetadataProvider(HashMap<String, ToolMetadata>);

    #[async_trait]
    impl ToolMetadataProvider for StubToolMetadataProvider {
        async fn get(
            &self,
            _workspace_id: &str,
            tool: &str,
        ) -> Result<Option<ToolMetadata>, ToolMetadataUnavailable> {
            Ok(self.0.get(tool).cloned())
        }
    }

    struct FailingToolMetadataProvider;

    #[async_trait]
    impl ToolMetadataProvider for FailingToolMetadataProvider {
        async fn get(
            &self,
            _workspace_id: &str,
            _tool: &str,
        ) -> Result<Option<ToolMetadata>, ToolMetadataUnavailable> {
            Err(ToolMetadataUnavailable)
        }
    }

    fn send_email_metadata(side_effect: SideEffectClass) -> ToolMetadata {
        ToolMetadata {
            tool: "send_email".into(),
            side_effect,
            reversible: false,
            params: vec![ParamSpec {
                path: "recipient".into(),
                role: ParamRole::AuthorityBearing,
                allowed_sources: vec![AllowedSource {
                    origin: Origin::User,
                    source_id: None,
                    kind: None,
                }],
            }],
            approval: None,
            sandbox_hint: None,
        }
    }

    fn ctx_with_metadata(metadata: &[ToolMetadata]) -> EventPipelineCtx {
        let stub = StubToolMetadataProvider(
            metadata
                .iter()
                .map(|m| (m.tool.clone(), m.clone()))
                .collect(),
        );
        EventPipelineCtx {
            tool_metadata: Arc::new(stub),
            ..EventPipelineCtx::no_op()
        }
    }

    #[tokio::test]
    async fn resolved_tool_attaches_metadata_and_side_effect() {
        let metadata = send_email_metadata(SideEffectClass::ExternalCommunication);
        let ctx = ctx_with_metadata(std::slice::from_ref(&metadata));

        let (event, _decision) = ctx
            .process(
                high_fidelity_event(),
                "ws_1",
                "production",
                Decision::allow("trace-1"),
            )
            .await;

        assert_eq!(
            event.action.side_effect,
            Some(SideEffectClass::ExternalCommunication)
        );
        assert_eq!(
            event.resolution,
            Some(ToolResolution::Resolved { metadata })
        );
    }

    #[tokio::test]
    async fn unregistered_tool_marks_conservative_evidence() {
        let ctx = ctx_with_metadata(&[]);

        let (event, _decision) = ctx
            .process(
                high_fidelity_event(),
                "ws_1",
                "production",
                Decision::allow("trace-1"),
            )
            .await;

        assert_eq!(event.resolution, Some(ToolResolution::Unregistered));
        // The collector-claimed side effect is left untouched.
        assert_eq!(
            event.action.side_effect,
            Some(SideEffectClass::ExternalCommunication)
        );
    }

    #[tokio::test]
    async fn resolution_never_changes_decision() {
        let resolved_ctx =
            ctx_with_metadata(&[send_email_metadata(SideEffectClass::ExternalCommunication)]);
        let unregistered_ctx = ctx_with_metadata(&[]);

        for ctx in [resolved_ctx, unregistered_ctx] {
            let decision = Decision::allow("trace-1");
            let before = serde_json::to_value(&decision).unwrap();

            let (_event, after) = ctx
                .process(high_fidelity_event(), "ws_1", "production", decision)
                .await;

            assert_eq!(serde_json::to_value(after).unwrap(), before);
        }
    }

    #[tokio::test]
    async fn provider_failure_marks_resolution_failed_and_decision_unchanged() {
        let ctx = EventPipelineCtx {
            tool_metadata: Arc::new(FailingToolMetadataProvider),
            ..EventPipelineCtx::no_op()
        };
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (event, after) = ctx
            .process(high_fidelity_event(), "ws_1", "production", decision)
            .await;

        // A registry outage is recorded as distinct evidence — never as
        // genuine absence — and the claimed side effect survives.
        assert_eq!(event.resolution, Some(ToolResolution::ResolutionFailed));
        assert_eq!(
            event.action.side_effect,
            Some(SideEffectClass::ExternalCommunication)
        );
        assert_eq!(serde_json::to_value(after).unwrap(), before);
    }

    #[tokio::test]
    async fn registry_side_effect_overrides_claimed_value() {
        let ctx = ctx_with_metadata(&[send_email_metadata(SideEffectClass::DbMutation)]);
        let mut event = high_fidelity_event();
        event.action.side_effect = Some(SideEffectClass::None);

        let (event, _decision) = ctx
            .process(event, "ws_1", "production", Decision::allow("trace-1"))
            .await;

        assert_eq!(event.action.side_effect, Some(SideEffectClass::DbMutation));
    }
}
