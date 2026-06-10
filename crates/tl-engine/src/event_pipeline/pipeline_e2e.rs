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
    Action, AllowedSource, Confidentiality, Decision, EventKind, GuardEvent, Integrity, LabelBasis,
    LabelPolicyStatus, Labels, Origin, ParamRole, ParamSpec, Principal, ProvenanceMap,
    SideEffectClass, Source, SourceLabelPolicy, ToolMetadata, ToolResolution, Trust,
};

use super::labels::{LabelPolicyProvider, LabelPolicyUnavailable};
use super::{
    EventPipelineCtx, PolicyLabelResolver, ProvenancePropagator, ToolMetadataProvider,
    ToolMetadataUnavailable,
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
        .process(send_email_event(), "ws_1", "production", decision)
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
        .process(send_email_event(), "ws_1", "production", decision)
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
        .process(send_email_event(), "ws_1", "production", decision)
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
        .process(sparse, "ws_1", "production", Decision::allow("trace-1"))
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
        .process(event, "ws_1", "production", Decision::allow("trace-1"))
        .await;

    let resolution = event.label_resolution.as_ref().expect("label evidence");
    let subject = &resolution.derived["subject"];
    // A provenance reference to a source the event never declared folds
    // to all-Unknown: missing provenance is unknown, never clean.
    assert_eq!(subject.trust, Trust::Unknown);
    assert_eq!(subject.confidentiality, Confidentiality::Unknown);
    assert_eq!(subject.integrity, Integrity::Unknown);
}
