use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{
    CheckerFindingEvidence, CheckerRun, Decision, EnforcementMode, GuardEvent, Severity,
    SignalEvidence, ToolMetadata, ToolResolution, Verdict,
};

pub mod checkers;
pub mod labels;
pub mod legacy_adapter;
#[cfg(test)]
mod pipeline_e2e;

pub use checkers::{ApprovalChecker, InformationFlowChecker, MemoryChecker, ParameterAuthChecker};
pub use labels::{
    combine_labels, origin_default_labels, resolve_source_labels, LabelPolicyProvider,
    LabelPolicyUnavailable, NoOpLabelPolicyProvider, PolicyLabelResolver, ProvenancePropagator,
};
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

/// Attaches trust/confidentiality/integrity labels to event sources and
/// records label-resolution evidence. Async so implementations can read
/// workspace label policies through a cached provider; resolution itself
/// stays deterministic and evidence-only.
#[async_trait]
pub trait LabelResolver: Send + Sync {
    async fn resolve(&self, event: &mut GuardEvent);
}

pub trait ProvenanceResolver: Send + Sync {
    fn resolve(&self, event: &mut GuardEvent);
}

/// Deterministic, in-process action-safety check. Checkers are pure: no
/// I/O, no clock, no mode awareness — they always report what they see and
/// the pipeline decides what the findings are allowed to affect.
pub trait Checker: Send + Sync {
    /// Stable identifier used to resolve this checker's enforcement mode
    /// and to label its evidence (e.g. `"information_flow"`).
    fn id(&self) -> &'static str;

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

#[async_trait]
impl LabelResolver for NoOpLabelResolver {
    async fn resolve(&self, _event: &mut GuardEvent) {}
}

pub struct NoOpProvenanceResolver;

impl ProvenanceResolver for NoOpProvenanceResolver {
    fn resolve(&self, _event: &mut GuardEvent) {}
}

pub struct NoOpChecker;

impl Checker for NoOpChecker {
    fn id(&self) -> &'static str {
        "no_op"
    }

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

/// Per-checker rollout modes resolved by the service layer from workspace
/// settings before the pipeline runs. Defaults to all `off`, so the
/// pipeline stays observe-only for any caller that does not opt in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckerModes {
    pub information_flow: EnforcementMode,
    pub memory: EnforcementMode,
    pub parameter_auth: EnforcementMode,
    pub approval: EnforcementMode,
}

impl CheckerModes {
    /// Mode for a checker id. Unknown ids resolve to `Off`, so a checker
    /// registered without explicit configuration can never enforce.
    pub fn for_checker(&self, id: &str) -> EnforcementMode {
        match id {
            checkers::INFORMATION_FLOW_CHECKER_ID => self.information_flow,
            checkers::MEMORY_CHECKER_ID => self.memory,
            checkers::PARAMETER_AUTH_CHECKER_ID => self.parameter_auth,
            checkers::APPROVAL_CHECKER_ID => self.approval,
            _ => EnforcementMode::Off,
        }
    }
}

/// Folds enforced checker findings into the decision: worst verdict wins
/// and the winning finding's evidence is copied onto the decision.
///
/// Shadow findings reach this composer with `verdict: None` (the pipeline
/// strips them), so they can never change the decision. Signals are
/// deliberately ignored for verdict computation: LLM/classifier output
/// never decides action safety.
pub struct ModeAwareDecisionComposer;

impl DecisionComposer for ModeAwareDecisionComposer {
    fn compose(
        &self,
        current: Decision,
        findings: &[CheckerFinding],
        _signals: &[Signal],
    ) -> Decision {
        let mut worst: Option<(&CheckerFinding, Verdict)> = None;
        for finding in findings {
            let Some(verdict) = finding.verdict else {
                continue;
            };
            let more_severe = match worst {
                Some((_, current_worst)) => {
                    current_worst.worst_with(verdict) == verdict && current_worst != verdict
                }
                None => true,
            };
            if more_severe {
                worst = Some((finding, verdict));
            }
        }

        let Some((finding, finding_verdict)) = worst else {
            return current;
        };
        let composed_verdict = current.verdict.worst_with(finding_verdict);
        if composed_verdict == current.verdict {
            // The seeded decision is already at least as severe; keep its
            // reason and evidence (an engine block survives untouched).
            return current;
        }

        let mut decision = current;
        decision.verdict = composed_verdict;
        decision.reason = format!(
            "{}: {}: {}",
            finding.checker_id,
            finding.violated_rule.as_deref().unwrap_or("unspecified"),
            finding.reason
        );
        decision.violated_rule = finding.violated_rule.clone();
        decision.remediation = finding.remediation.clone();
        decision.source_chain =
            (!finding.source_chain.is_empty()).then(|| finding.source_chain.clone());
        decision.risk_source = finding.risk_source.clone();
        decision.failure_mode = finding.failure_mode.clone();
        decision.harm_class = finding.harm_class.clone();
        decision
    }
}

pub struct NoOpTracePersister;

impl TracePersister for NoOpTracePersister {
    fn enqueue(&self, _event: &GuardEvent, _decision: &Decision) {}
}

/// Hard deadline for the advisory signal provider. Signals are sheddable
/// by contract: when the provider exceeds this budget the pipeline
/// continues without signals so the deterministic core stays available
/// under overload.
pub const DEFAULT_SIGNAL_BUDGET: std::time::Duration = std::time::Duration::from_millis(250);

#[derive(Clone)]
pub struct EventPipelineCtx {
    pub normalizer: Arc<dyn Normalizer>,
    pub principal_resolver: Arc<dyn PrincipalResolver>,
    pub tool_metadata: Arc<dyn ToolMetadataProvider>,
    pub label_resolver: Arc<dyn LabelResolver>,
    pub provenance_resolver: Arc<dyn ProvenanceResolver>,
    pub checkers: Vec<Arc<dyn Checker>>,
    pub signals: Arc<dyn SignalProvider>,
    /// Deadline for the advisory signal call; exceeded budgets shed the
    /// signals for that request instead of delaying the decision.
    pub signal_budget: std::time::Duration,
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
            checkers: vec![],
            signals: Arc::new(NoOpSignalProvider),
            signal_budget: DEFAULT_SIGNAL_BUDGET,
            composer: Arc::new(NoOpDecisionComposer),
            traces: Arc::new(NoOpTracePersister),
        }
    }

    /// Run one event through the stage chain. With no-op stages
    /// (observe-only), the returned decision is the unchanged
    /// `current_decision`; the returned event carries the collected
    /// evidence for trace enrichment. No stage performs blocking I/O:
    /// tool-metadata and label-policy reads go through cached providers
    /// that fail open.
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
        modes: CheckerModes,
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
        // providers below see the resolved event, so a checker observes the
        // registry value on resolution and the claimed value when
        // resolution did not succeed.
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
        self.label_resolver.resolve(&mut event).await;
        self.provenance_resolver.resolve(&mut event);

        // Checker and signal evidence is server-populated, like identity:
        // reset before evaluating so collector-submitted values never
        // survive.
        event.checks = Vec::new();
        event.signals = Vec::new();
        let mut findings = Vec::new();
        for checker in &self.checkers {
            let mode = modes.for_checker(checker.id());
            if mode == EnforcementMode::Off {
                continue;
            }
            let mut checker_findings = checker.check(&event);
            event
                .checks
                .push(checker_run_evidence(checker.id(), mode, &checker_findings));
            if mode == EnforcementMode::Shadow {
                // Evidence above keeps the full hypothetical verdicts; the
                // findings handed to the composer carry none, so shadow can
                // never change the decision.
                for finding in &mut checker_findings {
                    finding.verdict = None;
                }
            }
            findings.extend(checker_findings);
        }
        // The advisory path is sheddable: deterministic checkers above
        // already ran, so an over-budget provider costs evidence, never
        // availability or safety.
        let signals =
            match tokio::time::timeout(self.signal_budget, self.signals.signals(&event)).await {
                Ok(signals) => signals,
                Err(_) => {
                    tracing::warn!(
                        workspace_id = %event.principal.workspace_id,
                        budget_ms = self.signal_budget.as_millis() as u64,
                        "advisory signal provider exceeded budget; shed"
                    );
                    Vec::new()
                }
            };
        // Signals are advisory: they become trace-visible evidence here
        // but the composer never lets them decide action verdicts.
        event.signals = signals
            .iter()
            .map(|signal| SignalEvidence {
                provider_id: signal.provider_id.clone(),
                message: signal.message.clone(),
                severity: signal.severity,
            })
            .collect();
        let decision = self.composer.compose(current_decision, &findings, &signals);
        self.traces.enqueue(&event, &decision);
        (event, decision)
    }
}

/// Wire-shaped evidence for one checker evaluation, captured before any
/// shadow-mode verdict stripping.
fn checker_run_evidence(
    checker_id: &str,
    mode: EnforcementMode,
    findings: &[CheckerFinding],
) -> CheckerRun {
    CheckerRun {
        checker_id: checker_id.to_string(),
        mode,
        findings: findings
            .iter()
            .map(|finding| CheckerFindingEvidence {
                rule: finding
                    .violated_rule
                    .clone()
                    .unwrap_or_else(|| "unspecified".to_string()),
                reason: finding.reason.clone(),
                recommended_verdict: finding.verdict,
                source_chain: finding.source_chain.clone(),
                risk_source: finding.risk_source.clone(),
                failure_mode: finding.failure_mode.clone(),
                harm_class: finding.harm_class.clone(),
            })
            .collect(),
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
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn event_pipeline_no_op_context_has_all_collaborators() {
        let ctx = EventPipelineCtx::no_op();
        let event = output_event();

        assert_eq!(event.kind, EventKind::OutputProposed);
        assert_eq!(ctx.tool_metadata.get("ws_1", "output").await, Ok(None));
        assert!(ctx.checkers.is_empty());
    }

    #[tokio::test]
    async fn no_op_checker_and_signal_provider_return_empty() {
        let ctx = EventPipelineCtx::no_op();
        let event = output_event();

        assert!(NoOpChecker.check(&event).is_empty());
        assert!(ctx.signals.signals(&event).await.is_empty());
    }

    #[tokio::test]
    async fn no_op_pipeline_returns_current_decision_unchanged() {
        let ctx = EventPipelineCtx::no_op();
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (_event, after) = ctx
            .process(
                output_event(),
                "ws_1",
                "production",
                CheckerModes::default(),
                decision,
            )
            .await;

        assert_eq!(after.verdict, Verdict::Allow);
        assert_eq!(serde_json::to_value(after).unwrap(), before);
    }

    /// Sleeps past any test budget before returning a signal, so a test
    /// that sees its signal means shedding did NOT happen.
    struct SlowSignalProvider {
        delay: std::time::Duration,
    }

    #[async_trait]
    impl SignalProvider for SlowSignalProvider {
        async fn signals(&self, _event: &GuardEvent) -> Vec<Signal> {
            tokio::time::sleep(self.delay).await;
            vec![Signal {
                provider_id: "slow".into(),
                message: "late advisory".into(),
                severity: None,
            }]
        }
    }

    struct FastSignalProvider;

    #[async_trait]
    impl SignalProvider for FastSignalProvider {
        async fn signals(&self, _event: &GuardEvent) -> Vec<Signal> {
            vec![Signal {
                provider_id: "fast".into(),
                message: "advisory".into(),
                severity: None,
            }]
        }
    }

    #[tokio::test]
    async fn over_budget_signal_provider_is_shed() {
        // The 5s delay is intentionally enormous next to the 10ms budget
        // so the timeout always wins the race; the test completes in
        // ~10ms of real time because the provider future is dropped the
        // moment the budget elapses (sleep is cancellation-safe).
        let ctx = EventPipelineCtx {
            signals: Arc::new(SlowSignalProvider {
                delay: std::time::Duration::from_secs(5),
            }),
            signal_budget: std::time::Duration::from_millis(10),
            ..EventPipelineCtx::no_op()
        };
        let decision = Decision::allow("trace-shed");
        let before = serde_json::to_value(&decision).unwrap();

        let (event, after) = ctx
            .process(
                output_event(),
                "ws_1",
                "production",
                CheckerModes::default(),
                decision,
            )
            .await;

        // Shedding costs evidence, never the decision.
        assert!(event.signals.is_empty());
        assert_eq!(serde_json::to_value(after).unwrap(), before);
    }

    #[tokio::test]
    async fn within_budget_signal_provider_records_evidence() {
        let ctx = EventPipelineCtx {
            signals: Arc::new(FastSignalProvider),
            ..EventPipelineCtx::no_op()
        };
        let decision = Decision::allow("trace-fast");
        let before = serde_json::to_value(&decision).unwrap();

        let (event, after) = ctx
            .process(
                output_event(),
                "ws_1",
                "production",
                CheckerModes::default(),
                decision,
            )
            .await;

        assert_eq!(event.signals.len(), 1);
        assert_eq!(event.signals[0].provider_id, "fast");
        // Advisory evidence never changes the decision by itself.
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
            label_resolution: None,
            checks: vec![],
            signals: vec![],
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

            let (_event, after) = ctx
                .process(
                    event,
                    "ws_1",
                    "production",
                    CheckerModes::default(),
                    decision,
                )
                .await;

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
                CheckerModes::default(),
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
                CheckerModes::default(),
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
                CheckerModes::default(),
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
                .process(
                    high_fidelity_event(),
                    "ws_1",
                    "production",
                    CheckerModes::default(),
                    decision,
                )
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
            .process(
                high_fidelity_event(),
                "ws_1",
                "production",
                CheckerModes::default(),
                decision,
            )
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
            .process(
                event,
                "ws_1",
                "production",
                CheckerModes::default(),
                Decision::allow("trace-1"),
            )
            .await;

        assert_eq!(event.action.side_effect, Some(SideEffectClass::DbMutation));
    }

    fn untrusted_labels() -> Labels {
        Labels {
            trust: tl_core::Trust::Untrusted,
            confidentiality: tl_core::Confidentiality::Public,
            integrity: tl_core::Integrity::Low,
        }
    }

    fn trusted_labels() -> Labels {
        Labels {
            trust: tl_core::Trust::Trusted,
            confidentiality: tl_core::Confidentiality::Public,
            integrity: tl_core::Integrity::High,
        }
    }

    /// External-communication event controlled by an untrusted web source:
    /// the canonical flow-checker violation.
    fn untrusted_external_event() -> GuardEvent {
        let mut event = high_fidelity_event();
        event.sources[0].labels = untrusted_labels();
        event.sources[1].labels = trusted_labels();
        event
    }

    fn checker_ctx() -> EventPipelineCtx {
        EventPipelineCtx {
            checkers: vec![Arc::new(InformationFlowChecker), Arc::new(MemoryChecker)],
            composer: Arc::new(ModeAwareDecisionComposer),
            ..EventPipelineCtx::no_op()
        }
    }

    #[tokio::test]
    async fn off_mode_skips_evaluation_and_records_no_evidence() {
        let ctx = checker_ctx();
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (event, after) = ctx
            .process(
                untrusted_external_event(),
                "ws_1",
                "production",
                CheckerModes::default(),
                decision,
            )
            .await;

        assert!(event.checks.is_empty());
        assert_eq!(serde_json::to_value(after).unwrap(), before);
    }

    #[tokio::test]
    async fn shadow_mode_records_hypothetical_evidence_without_changing_decision() {
        let ctx = checker_ctx();
        let modes = CheckerModes {
            information_flow: EnforcementMode::Shadow,
            ..CheckerModes::default()
        };
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (event, after) = ctx
            .process(
                untrusted_external_event(),
                "ws_1",
                "production",
                modes,
                decision,
            )
            .await;

        assert_eq!(serde_json::to_value(after).unwrap(), before);
        assert_eq!(event.checks.len(), 1);
        let run = &event.checks[0];
        assert_eq!(run.checker_id, "information_flow");
        assert_eq!(run.mode, EnforcementMode::Shadow);
        assert_eq!(run.findings.len(), 1);
        assert_eq!(run.findings[0].recommended_verdict, Some(Verdict::Block));
        assert_eq!(run.findings[0].rule, "action-integrity");
    }

    #[tokio::test]
    async fn enforce_mode_applies_worst_finding_to_decision() {
        let ctx = checker_ctx();
        let modes = CheckerModes {
            information_flow: EnforcementMode::Enforce,
            ..CheckerModes::default()
        };

        let (event, decision) = ctx
            .process(
                untrusted_external_event(),
                "ws_1",
                "production",
                modes,
                Decision::allow("trace-1"),
            )
            .await;

        assert_eq!(decision.verdict, Verdict::Block);
        assert!(decision
            .reason
            .starts_with("information_flow: action-integrity:"));
        assert_eq!(decision.violated_rule.as_deref(), Some("action-integrity"));
        assert_eq!(decision.source_chain, Some(vec!["src.web".to_string()]));
        assert_eq!(decision.risk_source.as_deref(), Some("web"));
        assert_eq!(event.checks.len(), 1);
        assert_eq!(event.checks[0].mode, EnforcementMode::Enforce);
    }

    #[tokio::test]
    async fn enforce_mode_with_no_findings_keeps_decision_byte_identical() {
        let ctx = checker_ctx();
        let modes = CheckerModes {
            information_flow: EnforcementMode::Enforce,
            memory: EnforcementMode::Enforce,
            ..CheckerModes::default()
        };
        let mut event = high_fidelity_event();
        event.sources[0].labels = trusted_labels();
        event.sources[1].labels = trusted_labels();
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (event, after) = ctx
            .process(event, "ws_1", "production", modes, decision)
            .await;

        assert_eq!(serde_json::to_value(after).unwrap(), before);
        // Both checkers ran and recorded clean evaluations.
        assert_eq!(event.checks.len(), 2);
        assert!(event.checks.iter().all(|run| run.findings.is_empty()));
    }

    #[tokio::test]
    async fn modes_gate_each_checker_independently() {
        let ctx = checker_ctx();
        let mut provenance = ProvenanceMap::default();
        provenance.insert("note", vec!["src.web".into()]);
        let mut event = high_fidelity_event();
        event.kind = EventKind::MemoryWriteProposed;
        event.action.side_effect = None;
        event.sources[0].labels = untrusted_labels();
        event.provenance = provenance;

        // Memory checker off: the violating memory write passes untouched.
        let modes = CheckerModes {
            information_flow: EnforcementMode::Enforce,
            ..CheckerModes::default()
        };
        let (processed, decision) = ctx
            .process(
                event.clone(),
                "ws_1",
                "production",
                modes,
                Decision::allow("trace-1"),
            )
            .await;
        assert_eq!(decision.verdict, Verdict::Allow);
        // The memory checker never ran; only the (clean) flow run recorded.
        assert!(processed
            .checks
            .iter()
            .all(|run| run.checker_id != "memory"));

        // Memory checker enforced: the same event blocks.
        let modes = CheckerModes {
            memory: EnforcementMode::Enforce,
            ..CheckerModes::default()
        };
        let (processed, decision) = ctx
            .process(
                event,
                "ws_1",
                "production",
                modes,
                Decision::allow("trace-1"),
            )
            .await;
        assert_eq!(decision.verdict, Verdict::Block);
        assert_eq!(processed.checks.len(), 1);
        assert_eq!(processed.checks[0].checker_id, "memory");
    }

    #[tokio::test]
    async fn client_submitted_checker_evidence_never_survives() {
        let ctx = checker_ctx();
        let mut event = untrusted_external_event();
        event.checks = vec![tl_core::CheckerRun {
            checker_id: "information_flow".into(),
            mode: EnforcementMode::Enforce,
            findings: vec![],
        }];

        let (processed, _decision) = ctx
            .process(
                event,
                "ws_1",
                "production",
                CheckerModes::default(),
                Decision::allow("trace-1"),
            )
            .await;

        // All modes off: the fabricated run is wiped and nothing replaces it.
        assert!(processed.checks.is_empty());
    }

    fn finding_with(verdict: Option<Verdict>, rule: &str) -> CheckerFinding {
        CheckerFinding {
            checker_id: "information_flow".into(),
            verdict,
            reason: "reason".into(),
            violated_rule: Some(rule.into()),
            remediation: None,
            source_chain: vec!["src.web".into()],
            risk_source: Some("web".into()),
            failure_mode: Some("untrusted_control".into()),
            harm_class: Some("integrity".into()),
        }
    }

    #[test]
    fn composer_keeps_decision_when_no_finding_carries_a_verdict() {
        let current = Decision::allow("trace-1");
        let before = serde_json::to_value(&current).unwrap();
        let findings = vec![finding_with(None, "action-integrity")];

        let composed = ModeAwareDecisionComposer.compose(current, &findings, &[]);

        assert_eq!(serde_json::to_value(composed).unwrap(), before);
    }

    #[test]
    fn composer_applies_worst_finding_and_copies_evidence_fields() {
        let findings = vec![
            finding_with(Some(Verdict::Escalate), "missing-provenance"),
            finding_with(Some(Verdict::Block), "action-integrity"),
        ];

        let composed =
            ModeAwareDecisionComposer.compose(Decision::allow("trace-1"), &findings, &[]);

        assert_eq!(composed.verdict, Verdict::Block);
        assert_eq!(
            composed.reason,
            "information_flow: action-integrity: reason"
        );
        assert_eq!(composed.violated_rule.as_deref(), Some("action-integrity"));
        assert_eq!(composed.source_chain, Some(vec!["src.web".to_string()]));
        assert_eq!(composed.failure_mode.as_deref(), Some("untrusted_control"));
        assert_eq!(composed.harm_class.as_deref(), Some("integrity"));
    }

    #[test]
    fn composer_never_downgrades_the_seeded_verdict() {
        let mut current = Decision::allow("trace-1");
        current.verdict = Verdict::Block;
        current.reason = "engine block".into();
        let findings = vec![finding_with(Some(Verdict::Escalate), "missing-provenance")];

        let composed = ModeAwareDecisionComposer.compose(current, &findings, &[]);

        // The engine's block survives with its own reason; the less severe
        // checker finding does not overwrite it.
        assert_eq!(composed.verdict, Verdict::Block);
        assert_eq!(composed.reason, "engine block");
    }

    #[test]
    fn composer_upgrades_rewrite_seed_and_preserves_it_against_weaker_findings() {
        // Rewrite sits between Allow and Escalate in the severity ranking;
        // exercise both directions around it.
        let mut current = Decision::allow("trace-1");
        current.verdict = Verdict::Rewrite;
        current.reason = "engine rewrite".into();

        let upgraded = ModeAwareDecisionComposer.compose(
            current.clone(),
            &[finding_with(Some(Verdict::Block), "action-integrity")],
            &[],
        );
        assert_eq!(upgraded.verdict, Verdict::Block);

        let preserved = ModeAwareDecisionComposer.compose(
            current,
            &[finding_with(None, "action-integrity")],
            &[],
        );
        assert_eq!(preserved.verdict, Verdict::Rewrite);
        assert_eq!(preserved.reason, "engine rewrite");
    }

    #[test]
    fn composer_ignores_signals_for_verdict() {
        let current = Decision::allow("trace-1");
        let before = serde_json::to_value(&current).unwrap();
        let signals = vec![Signal {
            provider_id: "llm".into(),
            message: "looks dangerous".into(),
            severity: Some(Severity::Critical),
        }];

        let composed = ModeAwareDecisionComposer.compose(current, &[], &signals);

        assert_eq!(serde_json::to_value(composed).unwrap(), before);
    }

    #[test]
    fn deterministic_block_wins_over_advisory_allow_signal() {
        let finding = CheckerFinding {
            checker_id: "information_flow".into(),
            verdict: Some(Verdict::Block),
            reason: "sensitive data flows to an external sink".into(),
            violated_rule: Some("destination-permission".into()),
            remediation: None,
            source_chain: vec!["src.web".into()],
            risk_source: Some("web".into()),
            failure_mode: Some("data_exfiltration".into()),
            harm_class: Some("confidentiality".into()),
        };
        let advisory_allow = vec![Signal {
            provider_id: "llm".into(),
            message: "this looks fine to me".into(),
            severity: None,
        }];

        let composed = ModeAwareDecisionComposer.compose(
            Decision::allow("trace-1"),
            &[finding],
            &advisory_allow,
        );

        assert_eq!(composed.verdict, Verdict::Block);
        assert_eq!(
            composed.violated_rule.as_deref(),
            Some("destination-permission")
        );
    }

    /// One fixed advisory signal for every event.
    struct StubSignalProvider;

    #[async_trait]
    impl SignalProvider for StubSignalProvider {
        async fn signals(&self, _event: &GuardEvent) -> Vec<Signal> {
            vec![Signal {
                provider_id: "llm".into(),
                message: "looks dangerous".into(),
                severity: Some(Severity::Critical),
            }]
        }
    }

    #[tokio::test]
    async fn pipeline_records_signal_evidence_without_changing_decision() {
        let ctx = EventPipelineCtx {
            signals: Arc::new(StubSignalProvider),
            composer: Arc::new(ModeAwareDecisionComposer),
            ..EventPipelineCtx::no_op()
        };
        let decision = Decision::allow("trace-1");
        let before = serde_json::to_value(&decision).unwrap();

        let (event, after) = ctx
            .process(
                output_event(),
                "ws_1",
                "production",
                CheckerModes::default(),
                decision,
            )
            .await;

        assert_eq!(serde_json::to_value(after).unwrap(), before);
        assert_eq!(
            event.signals,
            vec![SignalEvidence {
                provider_id: "llm".into(),
                message: "looks dangerous".into(),
                severity: Some(Severity::Critical),
            }]
        );
    }

    #[tokio::test]
    async fn pipeline_strips_collector_submitted_signal_evidence() {
        let ctx = EventPipelineCtx::no_op();
        let mut event = output_event();
        event.signals = vec![SignalEvidence {
            provider_id: "spoofed".into(),
            message: "collector-claimed signal".into(),
            severity: None,
        }];

        let (event, _decision) = ctx
            .process(
                event,
                "ws_1",
                "production",
                CheckerModes::default(),
                Decision::allow("trace-1"),
            )
            .await;

        assert!(event.signals.is_empty());
    }
}
