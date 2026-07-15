//! End-to-end pipeline tests: one event through Phase 2 action
//! resolution and Phase 3 label resolution + provenance propagation.
//!
//! `PipelineFixture` is the one place tests assemble a pipeline with any
//! mix of live stages. Future phases extend the fixture with their own
//! stage fields instead of inventing new harnesses.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    Action, AllowedSource, AuthorizationEffect, Confidentiality, Decision, EnforcementMode,
    EventKind, GuardEvent, Integrity, LabelBasis, LabelPolicyStatus, Labels, LimitAction, Origin,
    ParamLimit, ParamRole, ParamSpec, Principal, ProvenanceMap, SideEffectClass, Source,
    SourceLabelPolicy, ToolMetadata, ToolResolution, Trust,
};

use super::labels::{LabelPolicyProvider, LabelPolicyUnavailable};
use super::{
    ApprovalChecker, Checker, CheckerModes, EventPipelineCtx, ModeAwareDecisionComposer,
    ParameterAuthChecker, PolicyLabelResolver, ProvenancePropagator, ToolMetadataProvider,
    ToolMetadataUnavailable, ValueLimitChecker,
};

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

struct StubLabelPolicyProvider(Vec<SourceLabelPolicy>);

#[async_trait]
impl LabelPolicyProvider for StubLabelPolicyProvider {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicy>, LabelPolicyUnavailable> {
        Ok(self.0.clone())
    }
}

struct FailingLabelPolicyProvider;

#[async_trait]
impl LabelPolicyProvider for FailingLabelPolicyProvider {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicy>, LabelPolicyUnavailable> {
        Err(LabelPolicyUnavailable)
    }
}

/// One place to assemble a pipeline with any mix of live stages.
#[derive(Default)]
struct PipelineFixture {
    tools: Vec<ToolMetadata>,
    policies: Vec<SourceLabelPolicy>,
    policy_unavailable: bool,
    checkers: Vec<Arc<dyn Checker>>,
}

impl PipelineFixture {
    fn with_tools(mut self, tools: Vec<ToolMetadata>) -> Self {
        self.tools = tools;
        self
    }

    fn with_policies(mut self, policies: Vec<SourceLabelPolicy>) -> Self {
        self.policies = policies;
        self
    }

    fn with_policy_outage(mut self) -> Self {
        self.policy_unavailable = true;
        self
    }

    fn with_checkers(mut self, checkers: Vec<Arc<dyn Checker>>) -> Self {
        self.checkers = checkers;
        self
    }

    fn ctx(&self) -> EventPipelineCtx {
        let policy_provider: Arc<dyn LabelPolicyProvider> = if self.policy_unavailable {
            Arc::new(FailingLabelPolicyProvider)
        } else {
            Arc::new(StubLabelPolicyProvider(self.policies.clone()))
        };
        EventPipelineCtx {
            tool_metadata: Arc::new(StubToolMetadataProvider(
                self.tools
                    .iter()
                    .map(|m| (m.tool.clone(), m.clone()))
                    .collect(),
            )),
            label_resolver: Arc::new(PolicyLabelResolver::new(policy_provider)),
            provenance_resolver: Arc::new(ProvenancePropagator),
            checkers: self.checkers.clone(),
            // The live composer is inert without enforce-mode findings, so
            // fixtures without checkers keep their observe-only behavior.
            composer: Arc::new(ModeAwareDecisionComposer),
            ..EventPipelineCtx::no_op()
        }
    }
}

fn send_email_metadata() -> ToolMetadata {
    ToolMetadata {
        tool: "send_email".into(),
        side_effect: SideEffectClass::ExternalCommunication,
        reversible: false,
        params: vec![ParamSpec {
            path: "recipient".into(),
            role: ParamRole::AuthorityBearing,
            allowed_sources: vec![AllowedSource {
                origin: Origin::User,
                source_id: None,
                kind: None,
            }],
            limit: None,
        }],
        approval: None,
        sandbox_hint: None,
    }
}

/// `tool.call.proposed` for send_email: recipient sourced from the web,
/// body from both the user and the web.
fn send_email_event() -> GuardEvent {
    let mut provenance = ProvenanceMap::default();
    provenance.insert("recipient", vec!["src.web".into()]);
    provenance.insert("body", vec!["src.user".into(), "src.web".into()]);

    GuardEvent {
        kind: EventKind::ToolCallProposed,
        principal: Principal {
            workspace_id: "ws_1".into(),
            environment_id: "production".into(),
            agent_id: "agent-1".into(),
            user_id: Some("user-1".into()),
            session_id: None,
            task_id: None,
            run_id: None,
            run_event_id: None,
        },
        action: Action {
            operation: "send_email".into(),
            parameters: serde_json::json!({ "recipient": "a@b.c", "body": "hi" }),
            side_effect: Some(SideEffectClass::None),
            invocation_id: None,
            tool_identity: None,
            authorization: None,
        },
        sources: vec![
            Source {
                id: "src.user".into(),
                origin: Origin::User,
                labels: Labels::default(),
                kind: None,
            },
            Source {
                id: "src.web".into(),
                origin: Origin::Web,
                labels: Labels::default(),
                kind: Some("web_page".into()),
            },
        ],
        provenance,
        resolution: None,
        label_resolution: None,
        checks: vec![],
        signals: vec![],
        context: serde_json::Value::Null,
    }
}

#[tokio::test]
async fn full_pipeline_resolves_tool_then_labels_then_derives_provenance() {
    let fixture = PipelineFixture::default()
        .with_tools(vec![send_email_metadata()])
        .with_policies(vec![SourceLabelPolicy {
            origin: Origin::Web,
            trust: None,
            confidentiality: Some(Confidentiality::Private),
            integrity: None,
        }]);
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = fixture
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            CheckerModes::default(),
            decision,
        )
        .await;

    // Phase 2: registry resolution is authoritative for the side effect.
    assert!(matches!(
        event.resolution,
        Some(ToolResolution::Resolved { .. })
    ));
    assert_eq!(
        event.action.side_effect,
        Some(SideEffectClass::ExternalCommunication)
    );

    // Phase 3a: source labels resolved in place with basis evidence.
    let resolution = event.label_resolution.as_ref().expect("label evidence");
    assert_eq!(resolution.policy_status, LabelPolicyStatus::Applied);

    let user = &event.sources[0];
    assert_eq!(user.labels.trust, Trust::Trusted);
    assert_eq!(user.labels.confidentiality, Confidentiality::Private);
    assert_eq!(user.labels.integrity, Integrity::High);

    let web = &event.sources[1];
    assert_eq!(web.labels.trust, Trust::Untrusted);
    // The workspace override upgrades web confidentiality to private.
    assert_eq!(web.labels.confidentiality, Confidentiality::Private);
    assert_eq!(web.labels.integrity, Integrity::Low);

    let web_evidence = &resolution.sources[1];
    assert_eq!(web_evidence.source_id, "src.web");
    assert_eq!(
        web_evidence.basis.confidentiality,
        LabelBasis::WorkspaceOverride
    );
    assert_eq!(web_evidence.basis.trust, LabelBasis::OriginDefault);

    // Phase 3b: derived labels per provenance path.
    let recipient = &resolution.derived["recipient"];
    assert_eq!(recipient.trust, Trust::Untrusted);
    assert_eq!(recipient.confidentiality, Confidentiality::Private);
    assert_eq!(recipient.integrity, Integrity::Low);

    let body = &resolution.derived["body"];
    assert_eq!(body.trust, Trust::Untrusted); // user ⊓ web: any untrusted
    assert_eq!(body.confidentiality, Confidentiality::Private);
    assert_eq!(body.integrity, Integrity::Low); // weakest source

    // Observe-only invariant: the decision is byte-identical.
    assert_eq!(serde_json::to_value(after).unwrap(), before);
}

#[tokio::test]
async fn unregistered_tool_still_gets_labels() {
    let fixture = PipelineFixture::default();
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = fixture
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            CheckerModes::default(),
            decision,
        )
        .await;

    assert_eq!(event.resolution, Some(ToolResolution::Unregistered));
    let resolution = event.label_resolution.as_ref().expect("label evidence");
    assert_eq!(resolution.policy_status, LabelPolicyStatus::NotConfigured);
    assert_eq!(resolution.sources.len(), 2);
    assert_eq!(event.sources[1].labels.trust, Trust::Untrusted);
    assert!(resolution.derived.contains_key("recipient"));
    assert_eq!(serde_json::to_value(after).unwrap(), before);
}

#[tokio::test]
async fn policy_store_outage_falls_back_to_defaults_and_marks_unavailable() {
    let fixture = PipelineFixture::default()
        .with_tools(vec![send_email_metadata()])
        .with_policy_outage();
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = fixture
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            CheckerModes::default(),
            decision,
        )
        .await;

    let resolution = event.label_resolution.as_ref().expect("label evidence");
    assert_eq!(resolution.policy_status, LabelPolicyStatus::Unavailable);
    // Built-in defaults still applied; the outage never blocks.
    assert_eq!(event.sources[0].labels.trust, Trust::Trusted);
    assert_eq!(event.sources[1].labels.trust, Trust::Untrusted);
    assert_eq!(serde_json::to_value(after).unwrap(), before);
}

#[tokio::test]
async fn event_with_no_sources_and_no_provenance_yields_empty_evidence() {
    let fixture = PipelineFixture::default();
    let mut sparse = send_email_event();
    sparse.sources.clear();
    sparse.provenance = ProvenanceMap::default();

    let (event, _after) = fixture
        .ctx()
        .process(
            sparse,
            "ws_1",
            "production",
            CheckerModes::default(),
            Decision::allow("trace-1"),
        )
        .await;

    // Evidence is never invented: the resolution container records the
    // policy status but carries no per-source or derived entries.
    let resolution = event.label_resolution.as_ref().expect("label evidence");
    assert!(resolution.sources.is_empty());
    assert!(resolution.derived.is_empty());
}

#[tokio::test]
async fn propagation_with_ghost_source_id_derives_unknown() {
    let fixture = PipelineFixture::default();
    let mut event = send_email_event();
    event.provenance.insert("subject", vec!["src.ghost".into()]);

    let (event, _after) = fixture
        .ctx()
        .process(
            event,
            "ws_1",
            "production",
            CheckerModes::default(),
            Decision::allow("trace-1"),
        )
        .await;

    let resolution = event.label_resolution.as_ref().expect("label evidence");
    let subject = &resolution.derived["subject"];
    // A provenance reference to a source the event never declared folds
    // to all-Unknown: missing provenance is unknown, never clean.
    assert_eq!(subject.trust, Trust::Unknown);
    assert_eq!(subject.confidentiality, Confidentiality::Unknown);
    assert_eq!(subject.integrity, Integrity::Unknown);
}

/// `send_email` with the registry's authority-bearing `recipient` param
/// (allowed origin: user) and a live parameter-auth checker. The default
/// `send_email_event` sources the recipient from the web — a violation.
fn param_auth_fixture() -> PipelineFixture {
    PipelineFixture::default()
        .with_tools(vec![send_email_metadata()])
        .with_checkers(vec![Arc::new(ParameterAuthChecker)])
}

fn param_auth_modes(mode: EnforcementMode) -> CheckerModes {
    CheckerModes {
        parameter_auth: mode,
        ..CheckerModes::default()
    }
}

#[tokio::test]
async fn param_auth_off_records_nothing_and_decision_unchanged() {
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = param_auth_fixture()
        .ctx()
        .process(
            send_email_event(),
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
async fn param_auth_shadow_records_hypothetical_block_without_changing_decision() {
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = param_auth_fixture()
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            param_auth_modes(EnforcementMode::Shadow),
            decision,
        )
        .await;

    assert_eq!(serde_json::to_value(after).unwrap(), before);
    assert_eq!(event.checks.len(), 1);
    let run = &event.checks[0];
    assert_eq!(run.checker_id, "parameter_auth");
    assert_eq!(run.mode, EnforcementMode::Shadow);
    assert_eq!(run.findings.len(), 1);
    assert_eq!(run.findings[0].rule, "parameter_source.recipient");
    assert_eq!(
        run.findings[0].recommended_effect,
        Some(AuthorizationEffect::Deny)
    );
}

#[tokio::test]
async fn param_auth_enforce_blocks_wrong_source() {
    let (event, after) = param_auth_fixture()
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            param_auth_modes(EnforcementMode::Enforce),
            Decision::allow("trace-1"),
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::Deny);
    assert!(after
        .reason
        .starts_with("parameter_auth: parameter_source.recipient:"));
    assert_eq!(
        after.violated_rule.as_deref(),
        Some("parameter_source.recipient")
    );
    assert!(after.remediation.is_some());
    assert_eq!(
        after.source_chain.as_deref(),
        Some(&["src.web".to_string()][..])
    );
    assert_eq!(event.checks.len(), 1);
    assert_eq!(event.checks[0].mode, EnforcementMode::Enforce);
}

#[tokio::test]
async fn param_auth_enforce_allows_correct_source() {
    let mut event = send_email_event();
    event
        .provenance
        .insert("recipient", vec!["src.user".into()]);

    let (_event, after) = param_auth_fixture()
        .ctx()
        .process(
            event,
            "ws_1",
            "production",
            param_auth_modes(EnforcementMode::Enforce),
            Decision::allow("trace-1"),
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::Permit);
}

#[tokio::test]
async fn param_auth_enforce_defers_missing_provenance() {
    let mut event = send_email_event();
    event.provenance.0.remove("recipient");

    let (_event, after) = param_auth_fixture()
        .ctx()
        .process(
            event,
            "ws_1",
            "production",
            param_auth_modes(EnforcementMode::Enforce),
            Decision::allow("trace-1"),
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::Defer);
    assert_eq!(after.risk_code.as_deref(), Some("missing_provenance"));
}

/// `send_email` whose registry entry requires admin approval, with a live
/// approval checker. The default `send_email_event` resolves against it.
fn approval_fixture() -> PipelineFixture {
    let mut metadata = send_email_metadata();
    metadata.approval = Some(tl_core::ApprovalRule {
        required: true,
        approver_roles: vec!["admin".into()],
        reason: None,
    });
    PipelineFixture::default()
        .with_tools(vec![metadata])
        .with_checkers(vec![Arc::new(ApprovalChecker)])
}

fn approval_modes(mode: EnforcementMode) -> CheckerModes {
    CheckerModes {
        approval: mode,
        ..CheckerModes::default()
    }
}

#[tokio::test]
async fn approval_off_records_nothing_and_decision_unchanged() {
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = approval_fixture()
        .ctx()
        .process(
            send_email_event(),
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
async fn approval_shadow_records_hypothetical_escalate_without_changing_decision() {
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = approval_fixture()
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            approval_modes(EnforcementMode::Shadow),
            decision,
        )
        .await;

    assert_eq!(serde_json::to_value(after).unwrap(), before);
    assert_eq!(event.checks.len(), 1);
    let run = &event.checks[0];
    assert_eq!(run.checker_id, "approval");
    assert_eq!(run.mode, EnforcementMode::Shadow);
    assert_eq!(run.findings.len(), 1);
    assert_eq!(run.findings[0].rule, "approval.send_email");
    assert_eq!(
        run.findings[0].recommended_effect,
        Some(AuthorizationEffect::RequireApproval)
    );
}

#[tokio::test]
async fn approval_enforce_escalates_required_tool() {
    let (event, after) = approval_fixture()
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            approval_modes(EnforcementMode::Enforce),
            Decision::allow("trace-1"),
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::RequireApproval);
    assert!(after.reason.starts_with("approval: approval.send_email:"));
    assert_eq!(after.violated_rule.as_deref(), Some("approval.send_email"));
    assert_eq!(
        after.remediation.as_deref(),
        Some("request approval from roles: admin before retrying this action")
    );
    assert_eq!(after.risk_code.as_deref(), Some("approval_required"));
    assert_eq!(after.harm_class.as_deref(), Some("authorization"));
    assert_eq!(event.checks.len(), 1);
    assert_eq!(event.checks[0].mode, EnforcementMode::Enforce);
}

#[tokio::test]
async fn approval_enforce_does_not_demote_an_engine_block() {
    let mut blocked = Decision::allow("trace-1");
    blocked.effect = AuthorizationEffect::Deny;
    blocked.reason = "tier1 policy `pii` triggered".into();

    let (_event, after) = approval_fixture()
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            approval_modes(EnforcementMode::Enforce),
            blocked,
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::Deny);
    assert_eq!(after.reason, "tier1 policy `pii` triggered");
}

#[tokio::test]
async fn approval_enforce_ignores_tools_without_approval_rules() {
    let (_event, after) = PipelineFixture::default()
        .with_tools(vec![send_email_metadata()])
        .with_checkers(vec![Arc::new(ApprovalChecker)])
        .ctx()
        .process(
            send_email_event(),
            "ws_1",
            "production",
            approval_modes(EnforcementMode::Enforce),
            Decision::allow("trace-1"),
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::Permit);
}

/// `issue_refund` whose `amount` parameter carries a value limit. Value
/// limits ride the `parameter_auth` enforcement mode (see
/// `CheckerModes::for_checker`), so these tests gate on `param_auth_modes`.
fn refund_with_limit(limit: ParamLimit) -> ToolMetadata {
    ToolMetadata {
        tool: "issue_refund".into(),
        side_effect: SideEffectClass::ApiMutation,
        reversible: false,
        params: vec![ParamSpec {
            path: "amount".into(),
            role: ParamRole::AuthorityBearing,
            allowed_sources: vec![],
            limit: Some(limit),
        }],
        approval: None,
        sandbox_hint: None,
    }
}

fn issue_refund_event(amount: i64) -> GuardEvent {
    GuardEvent {
        kind: EventKind::ToolCallProposed,
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
            operation: "issue_refund".into(),
            parameters: serde_json::json!({ "amount": amount }),
            side_effect: Some(SideEffectClass::ApiMutation),
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
        context: serde_json::Value::Null,
    }
}

fn value_limit_fixture(limit: ParamLimit) -> PipelineFixture {
    PipelineFixture::default()
        .with_tools(vec![refund_with_limit(limit)])
        .with_checkers(vec![Arc::new(ValueLimitChecker)])
}

fn max_block(max: i64) -> ParamLimit {
    ParamLimit {
        max: Some(max),
        min: None,
        on_breach: LimitAction::Deny,
    }
}

#[tokio::test]
async fn value_limit_enforce_blocks_over_max_refund() {
    let (event, after) = value_limit_fixture(max_block(500))
        .ctx()
        .process(
            issue_refund_event(9999),
            "ws_1",
            "production",
            param_auth_modes(EnforcementMode::Enforce),
            Decision::allow("trace-1"),
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::Deny);
    assert!(after
        .reason
        .starts_with("value_limit: parameter_value.amount:"));
    assert_eq!(
        after.violated_rule.as_deref(),
        Some("parameter_value.amount")
    );
    assert_eq!(after.risk_code.as_deref(), Some("amount_over_limit"));
    assert_eq!(after.harm_class.as_deref(), Some("authorization"));
    assert_eq!(event.checks.len(), 1);
    assert_eq!(event.checks[0].checker_id, "value_limit");
    assert_eq!(event.checks[0].mode, EnforcementMode::Enforce);
}

#[tokio::test]
async fn value_limit_enforce_allows_within_max_refund() {
    let (_event, after) = value_limit_fixture(max_block(500))
        .ctx()
        .process(
            issue_refund_event(500),
            "ws_1",
            "production",
            param_auth_modes(EnforcementMode::Enforce),
            Decision::allow("trace-1"),
        )
        .await;

    assert_eq!(after.effect, AuthorizationEffect::Permit);
}

#[tokio::test]
async fn value_limit_off_records_nothing_and_decision_unchanged() {
    let decision = Decision::allow("trace-1");
    let before = serde_json::to_value(&decision).unwrap();

    let (event, after) = value_limit_fixture(max_block(500))
        .ctx()
        .process(
            issue_refund_event(9999),
            "ws_1",
            "production",
            CheckerModes::default(),
            decision,
        )
        .await;

    assert!(event.checks.is_empty());
    assert_eq!(serde_json::to_value(after).unwrap(), before);
}
