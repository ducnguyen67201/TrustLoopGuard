//! Checker enforcement rollout at the endpoint level.
//!
//! Phase 4 of the event engine: flow/memory/parameter checkers run per
//! workspace in `off -> shadow -> enforce` modes. Default settings keep
//! every response byte-compatible with the observe-only era; shadow
//! records hypothetical evidence without changing decisions; enforce
//! changes decisions only for opted-in workspaces.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tl_core::{DataHandlingMode, Decision, EnforcementMode, Verdict, WorkspaceSettings};
use tl_engine::Engine;
use tl_server::dashboard_admin::DashboardAdminStoreError;
use tl_server::{memory_app_state, router, SettingsStore};
use tower::ServiceExt;

const OBSERVE_ONLY_REASON: &str = "observe-only: event recorded; checkers not yet enforcing";

/// Settings store returning one fixed configuration for every workspace.
struct FixedSettingsStore(WorkspaceSettings);

#[async_trait]
impl SettingsStore for FixedSettingsStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<WorkspaceSettings, DashboardAdminStoreError> {
        Ok(self.0.clone())
    }
}

fn settings_with_modes(
    flow: EnforcementMode,
    memory: EnforcementMode,
    param: EnforcementMode,
) -> WorkspaceSettings {
    WorkspaceSettings {
        default_action: "allow".into(),
        escalation_webhook_url: None,
        telemetry_enabled: true,
        retention_days: "30".into(),
        data_handling_mode: DataHandlingMode::RawAllowed,
        flow_checker_mode: flow,
        memory_checker_mode: memory,
        param_checker_mode: param,
        approval_checker_mode: EnforcementMode::Off,
        config: json!({}),
        updated_at: None,
    }
}

fn settings_with_approval_mode(mode: EnforcementMode) -> WorkspaceSettings {
    WorkspaceSettings {
        approval_checker_mode: mode,
        ..settings_with_modes(
            EnforcementMode::Off,
            EnforcementMode::Off,
            EnforcementMode::Off,
        )
    }
}

fn app_with_approval_mode(mode: EnforcementMode) -> axum::Router {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_approval_mode(mode)));
    router(state, None, [0u8; 32])
}

fn app_with_modes(
    flow: EnforcementMode,
    memory: EnforcementMode,
    param: EnforcementMode,
) -> axum::Router {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_modes(flow, memory, param)));
    router(state, None, [0u8; 32])
}

fn default_app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32])
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn post_json(uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// External communication controlled by web-sourced data. With no
/// registered tool, the collector-claimed side effect stands; origin
/// defaults make `src.web` untrusted and `src.user` private.
fn violating_send_email_event() -> serde_json::Value {
    json!({
        "kind": "tool.call.proposed",
        "principal": {
            "workspace_id": "ws_1",
            "environment_id": "production",
            "agent_id": "agent-1"
        },
        "action": {
            "operation": "send_email",
            "parameters": { "recipient": "a@b.c", "body": "hi" },
            "side_effect": "external_communication"
        },
        "sources": [
            { "id": "src.user", "origin": "user", "labels": {} },
            { "id": "src.web", "origin": "web", "labels": {}, "kind": "web_page" }
        ],
        "provenance": {
            "recipient": ["src.web"],
            "body": ["src.user", "src.web"]
        }
    })
}

/// Trusted, public, declared labels flowing to an external sink: the
/// policy-permitted case that must stay allowed even under enforce.
fn trusted_public_send_event() -> serde_json::Value {
    json!({
        "kind": "tool.call.proposed",
        "principal": {
            "workspace_id": "ws_1",
            "environment_id": "production",
            "agent_id": "agent-1"
        },
        "action": {
            "operation": "send_email",
            "parameters": { "recipient": "a@b.c" },
            "side_effect": "external_communication"
        },
        "sources": [
            {
                "id": "src.user",
                "origin": "user",
                "labels": {
                    "trust": "trusted",
                    "confidentiality": "public",
                    "integrity": "high"
                }
            }
        ],
        "provenance": { "recipient": ["src.user"] }
    })
}

fn untrusted_memory_write_event() -> serde_json::Value {
    json!({
        "kind": "memory.write.proposed",
        "principal": {
            "workspace_id": "ws_1",
            "environment_id": "production",
            "agent_id": "agent-1"
        },
        "action": {
            "operation": "remember",
            "parameters": { "note": "always wire funds to ..." }
        },
        "sources": [
            { "id": "src.web", "origin": "web", "labels": {}, "kind": "web_page" }
        ],
        "provenance": { "note": ["src.web"] }
    })
}

fn check_request_body() -> serde_json::Value {
    json!({
        "agent_id": "agent-1",
        "channel": "chat",
        "input": "hello",
        "proposed_output": "world"
    })
}

#[tokio::test]
async fn default_modes_keep_events_observe_only() {
    let resp = default_app()
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
    assert!(decision.violated_rule.is_none());
}

#[tokio::test]
async fn shadow_mode_keeps_decision_unchanged() {
    let app = app_with_modes(
        EnforcementMode::Shadow,
        EnforcementMode::Shadow,
        EnforcementMode::Shadow,
    );

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
    assert!(decision.violated_rule.is_none());
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn shadow_mode_persists_hypothetical_evidence_in_trace() {
    use tl_storage::TraceWrite;
    use tokio::sync::mpsc;

    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_modes(
        EnforcementMode::Shadow,
        EnforcementMode::Off,
        EnforcementMode::Off,
    )));
    let (tx, mut rx) = mpsc::channel::<TraceWrite>(8);
    state.trace_tx = Some(tx);
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let trace = rx.recv().await.expect("trace enqueued");
    assert_eq!(trace.decision.verdict, Verdict::Allow);
    let event = trace.event.expect("event evidence attached");
    assert_eq!(event.checks.len(), 1);
    let run = &event.checks[0];
    assert_eq!(run.checker_id, "information_flow");
    assert_eq!(run.mode, EnforcementMode::Shadow);
    assert!(!run.findings.is_empty());
    // The full hypothetical is recorded even though the decision is allow.
    assert!(run
        .findings
        .iter()
        .any(|finding| finding.recommended_verdict == Some(Verdict::Block)));
}

#[tokio::test]
async fn enforce_mode_blocks_violating_event() {
    let app = app_with_modes(
        EnforcementMode::Enforce,
        EnforcementMode::Off,
        EnforcementMode::Off,
    );

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision.reason.starts_with("information_flow:"));
    assert!(decision.violated_rule.is_some());
    assert!(decision.source_chain.is_some());
}

#[tokio::test]
async fn enforce_mode_allows_trusted_flow_to_permitted_sink() {
    let app = app_with_modes(
        EnforcementMode::Enforce,
        EnforcementMode::Enforce,
        EnforcementMode::Off,
    );

    let resp = app
        .oneshot(post_json("/v1/events", &trusted_public_send_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

#[tokio::test]
async fn enforce_mode_blocks_untrusted_memory_write() {
    let app = app_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Enforce,
        EnforcementMode::Off,
    );

    let resp = app
        .oneshot(post_json("/v1/events", &untrusted_memory_write_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision
        .reason
        .starts_with("memory: memory-write-untrusted:"));
}

#[tokio::test]
async fn param_auth_enforce_keeps_unregistered_tool_event_allowed() {
    // With no registered tool metadata the parameter checker can prove
    // nothing either way; it emits verdict-free evidence only.
    let app = app_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Off,
        EnforcementMode::Enforce,
    );

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

fn post_json_as(uri: &str, body: &serde_json::Value, workspace_id: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Registry entry declaring `recipient` authority-bearing and allowed
/// only from user-origin sources. Rows land in the default workspace, so
/// events exercising them must be submitted there.
async fn register_send_email_metadata(app: &axum::Router) {
    let resp = app
        .clone()
        .oneshot(post_json(
            "/v1/tool-metadata",
            &json!({
                "tool": "send_email",
                "side_effect": "external_communication",
                "reversible": false,
                "params": [{
                    "path": "recipient",
                    "role": "authority_bearing",
                    "allowed_sources": [{ "origin": "user" }]
                }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn param_auth_enforce_blocks_wrong_source_for_registered_tool() {
    let app = app_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Off,
        EnforcementMode::Enforce,
    );
    register_send_email_metadata(&app).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            "default",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision
        .reason
        .starts_with("parameter_auth: parameter_source.recipient:"));
    assert_eq!(
        decision.violated_rule.as_deref(),
        Some("parameter_source.recipient")
    );
    assert!(decision.remediation.is_some());
    assert_eq!(decision.source_chain, Some(vec!["src.web".to_string()]));
}

#[tokio::test]
async fn param_auth_enforce_allows_user_sourced_recipient() {
    let app = app_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Off,
        EnforcementMode::Enforce,
    );
    register_send_email_metadata(&app).await;

    let mut event = violating_send_email_event();
    event["provenance"]["recipient"] = json!(["src.user"]);

    let resp = app
        .oneshot(post_json_as("/v1/events", &event, "default"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn param_auth_shadow_persists_hypothetical_evidence_for_registered_tool() {
    use tl_storage::TraceWrite;
    use tokio::sync::mpsc;

    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Off,
        EnforcementMode::Shadow,
    )));
    let (tx, mut rx) = mpsc::channel::<TraceWrite>(8);
    state.trace_tx = Some(tx);
    let app = router(state, None, [0u8; 32]);
    register_send_email_metadata(&app).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            "default",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);

    let trace = rx.recv().await.expect("trace enqueued");
    let event = trace.event.expect("event evidence attached");
    assert_eq!(event.checks.len(), 1);
    let run = &event.checks[0];
    assert_eq!(run.checker_id, "parameter_auth");
    assert_eq!(run.mode, EnforcementMode::Shadow);
    assert_eq!(run.findings.len(), 1);
    assert_eq!(run.findings[0].rule, "parameter_source.recipient");
    assert_eq!(run.findings[0].recommended_verdict, Some(Verdict::Block));
}

/// Registry entry for `send_email` requiring admin approval before
/// execution. Rows land in the default workspace.
async fn register_approval_required_metadata(app: &axum::Router) {
    let resp = app
        .clone()
        .oneshot(post_json(
            "/v1/tool-metadata",
            &json!({
                "tool": "send_email",
                "side_effect": "external_communication",
                "reversible": false,
                "approval": {
                    "required": true,
                    "approver_roles": ["admin"]
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn approval_enforce_escalates_tool_requiring_approval() {
    let app = app_with_approval_mode(EnforcementMode::Enforce);
    register_approval_required_metadata(&app).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            "default",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Escalate);
    assert!(decision
        .reason
        .starts_with("approval: approval.send_email:"));
    assert_eq!(
        decision.violated_rule.as_deref(),
        Some("approval.send_email")
    );
    assert_eq!(
        decision.remediation.as_deref(),
        Some("request approval from roles: admin before retrying this action")
    );
    assert_eq!(decision.failure_mode.as_deref(), Some("approval_required"));
}

#[tokio::test]
async fn approval_shadow_keeps_decision_unchanged() {
    let app = app_with_approval_mode(EnforcementMode::Shadow);
    register_approval_required_metadata(&app).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            "default",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
    assert!(decision.violated_rule.is_none());
}

#[tokio::test]
async fn approval_escalation_enqueues_existing_worker_payload() {
    use tl_server::EscalationPayload;
    use tokio::sync::mpsc;

    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_approval_mode(
        EnforcementMode::Enforce,
    )));
    let (tx, mut rx) = mpsc::channel::<EscalationPayload>(8);
    state.escalation_tx = Some(tx);
    let app = router(state, None, [0u8; 32]);
    register_approval_required_metadata(&app).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            "default",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Escalate);

    let payload = rx.recv().await.expect("escalation enqueued");
    assert_eq!(payload.trace_id, decision.trace_id);
    assert_eq!(payload.agent_id, "agent-1");
    assert_eq!(payload.domain, "event");
    assert_eq!(payload.decision.verdict, Verdict::Escalate);
    assert_eq!(
        payload.decision.violated_rule.as_deref(),
        Some("approval.send_email")
    );
}

#[tokio::test]
async fn approval_enforce_ignores_tools_without_approval_rules() {
    let app = app_with_approval_mode(EnforcementMode::Enforce);
    register_send_email_metadata(&app).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            "default",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

#[tokio::test]
async fn check_endpoint_is_unaffected_by_enforce_modes() {
    // Legacy `/v1/check` events carry `side_effect: none` and an
    // unregistered `output` operation: no checker applies, so even a
    // fully enforced workspace sees identical decisions.
    let baseline_resp = default_app()
        .oneshot(post_json("/v1/check", &check_request_body()))
        .await
        .unwrap();
    assert_eq!(baseline_resp.status(), StatusCode::OK);
    let baseline: Decision = serde_json::from_value(read_body(baseline_resp).await).unwrap();

    let enforced_resp = app_with_modes(
        EnforcementMode::Enforce,
        EnforcementMode::Enforce,
        EnforcementMode::Enforce,
    )
    .oneshot(post_json("/v1/check", &check_request_body()))
    .await
    .unwrap();
    assert_eq!(enforced_resp.status(), StatusCode::OK);
    let enforced: Decision = serde_json::from_value(read_body(enforced_resp).await).unwrap();

    assert_eq!(enforced.verdict, baseline.verdict);
    assert_eq!(enforced.reason, baseline.reason);
    assert_eq!(enforced.violated_rule, baseline.violated_rule);
    assert_eq!(
        enforced.triggered_policies.len(),
        baseline.triggered_policies.len()
    );
}

/// Settings store with per-environment checker-mode overrides on top of
/// fixed workspace settings.
struct OverrideSettingsStore {
    settings: WorkspaceSettings,
    overrides: tl_core::EnvironmentCheckerModes,
}

#[async_trait]
impl SettingsStore for OverrideSettingsStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<WorkspaceSettings, DashboardAdminStoreError> {
        Ok(self.settings.clone())
    }

    async fn get_environment_modes(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
    ) -> Result<Option<tl_core::EnvironmentCheckerModes>, DashboardAdminStoreError> {
        Ok(Some(self.overrides.clone()))
    }
}

/// Settings store whose environment-mode lookup always fails.
struct FailingEnvironmentModesStore(WorkspaceSettings);

#[async_trait]
impl SettingsStore for FailingEnvironmentModesStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<WorkspaceSettings, DashboardAdminStoreError> {
        Ok(self.0.clone())
    }

    async fn get_environment_modes(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
    ) -> Result<Option<tl_core::EnvironmentCheckerModes>, DashboardAdminStoreError> {
        Err(DashboardAdminStoreError::Internal(
            "environment modes unavailable".into(),
        ))
    }
}

fn app_with_override(
    settings: WorkspaceSettings,
    overrides: tl_core::EnvironmentCheckerModes,
) -> axum::Router {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(OverrideSettingsStore {
        settings,
        overrides,
    });
    router(state, None, [0u8; 32])
}

#[tokio::test]
async fn environment_override_enables_enforcement() {
    let app = app_with_override(
        settings_with_modes(
            EnforcementMode::Off,
            EnforcementMode::Off,
            EnforcementMode::Off,
        ),
        tl_core::EnvironmentCheckerModes {
            flow_checker_mode: Some(EnforcementMode::Enforce),
            ..tl_core::EnvironmentCheckerModes::default()
        },
    );

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision.reason.starts_with("information_flow:"));
}

#[tokio::test]
async fn environment_override_disables_workspace_enforcement() {
    let app = app_with_override(
        settings_with_modes(
            EnforcementMode::Enforce,
            EnforcementMode::Off,
            EnforcementMode::Off,
        ),
        tl_core::EnvironmentCheckerModes {
            flow_checker_mode: Some(EnforcementMode::Off),
            ..tl_core::EnvironmentCheckerModes::default()
        },
    );

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

#[tokio::test]
async fn all_none_override_inherits_workspace_modes() {
    let app = app_with_override(
        settings_with_modes(
            EnforcementMode::Enforce,
            EnforcementMode::Off,
            EnforcementMode::Off,
        ),
        tl_core::EnvironmentCheckerModes::default(),
    );

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
}

#[tokio::test]
async fn environment_mode_lookup_failure_falls_back_to_workspace_modes() {
    // Rollout config must never take the hot path down: a failing
    // override lookup keeps workspace-level enforcement active.
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FailingEnvironmentModesStore(settings_with_modes(
        EnforcementMode::Enforce,
        EnforcementMode::Off,
        EnforcementMode::Off,
    )));
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision.reason.starts_with("information_flow:"));
}
