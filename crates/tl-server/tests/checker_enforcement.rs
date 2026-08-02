//! Checker enforcement rollout at the endpoint level.
//!
//! Phase 4 of the event engine: flow/memory/parameter checkers run per
//! workspace in `off -> shadow -> enforce` modes. Default settings keep
//! every response as a default allow; shadow records hypothetical
//! evidence without changing decisions; enforce changes decisions only
//! for opted-in workspaces.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tl_core::{
    AuthorizationDecision, AuthorizationEffect, DataHandlingMode, EnforcementMode,
    WorkspaceSettings,
};
use tl_engine::Engine;
use tl_server::dashboard_admin::DashboardAdminStoreError;
use tl_server::{memory_app_state, router, SettingsStore};
use tower::ServiceExt;
use uuid::Uuid;

const OBSERVE_ONLY_REASON: &str = "current policy and authority permit the subject";

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

fn app_with_modes(
    flow: EnforcementMode,
    memory: EnforcementMode,
    param: EnforcementMode,
) -> axum::Router {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_modes(flow, memory, param)));
    router(state, None, [0u8; 32])
}

async fn app_with_owner_and_settings(settings: WorkspaceSettings) -> (axum::Router, String, Uuid) {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings));
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Checker Enforcement")
        .await
        .unwrap();
    (router(state, None, [0u8; 32]), workspace.id, owner_id)
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
        .header("x-featherlane-ai-workspace-id", "ws")
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
            "invocation_id": Uuid::new_v4().to_string(),
            "operation": "send_email",
            "parameters": { "recipient": "a@b.c", "body": "hi" },
            "side_effect": "external_communication",
            "tool_identity": {
                "server_id": "mail",
                "tool_name": "send_email",
                "schema_hash": "sha256:v1:test-schema"
            }
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

fn approval_send_email_event() -> serde_json::Value {
    let mut event = violating_send_email_event();
    event["action"]["invocation_id"] = json!(Uuid::new_v4().to_string());
    event["action"]["tool_identity"] = json!({
        "server_id": "mail",
        "tool_name": "send_email",
        "schema_hash": "sha256:v1:test-schema"
    });
    event
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
            "invocation_id": Uuid::new_v4().to_string(),
            "operation": "send_email",
            "parameters": { "recipient": "a@b.c" },
            "side_effect": "external_communication",
            "tool_identity": {
                "server_id": "mail",
                "tool_name": "send_email",
                "schema_hash": "sha256:v1:test-schema"
            }
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

fn forged_trusted_web_send_event() -> serde_json::Value {
    let mut event = trusted_public_send_event();
    event["sources"][0]["id"] = json!("src.web");
    event["sources"][0]["origin"] = json!("web");
    event["provenance"]["recipient"] = json!(["src.web"]);
    event
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
            "invocation_id": Uuid::new_v4().to_string(),
            "operation": "remember",
            "parameters": { "note": "always wire funds to ..." },
            "side_effect": "memory_write",
            "tool_identity": {
                "server_id": "memory",
                "tool_name": "remember",
                "schema_hash": "sha256:v1:test-schema"
            }
        },
        "sources": [
            { "id": "src.web", "origin": "web", "labels": {}, "kind": "web_page" }
        ],
        "provenance": { "note": ["src.web"] }
    })
}

#[tokio::test]
async fn default_modes_keep_events_default_allow() {
    let resp = default_app()
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
    assert!(decision.findings.is_empty());
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
    assert!(decision
        .findings
        .iter()
        .all(|finding| finding.effect == AuthorizationEffect::Permit));
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn shadow_mode_persists_hypothetical_evidence_in_trace() {
    use tl_server::traces::ChannelTraceStore;

    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_modes(
        EnforcementMode::Shadow,
        EnforcementMode::Off,
        EnforcementMode::Off,
    )));
    let (capture, mut rx) = ChannelTraceStore::channel(8);
    state.trace_store = capture;
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(post_json("/v1/events", &violating_send_email_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let trace = rx.recv().await.expect("trace enqueued");
    assert_eq!(trace.decision.effect, AuthorizationEffect::Permit);
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
        .any(|finding| finding.recommended_effect == Some(AuthorizationEffect::Deny)));
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
    assert_eq!(decision.reason, decision.findings[0].reason);
    assert!(decision
        .findings
        .iter()
        .any(|finding| finding.effect == AuthorizationEffect::Deny));
    assert!(decision
        .findings
        .iter()
        .any(|finding| finding.evidence["source_chain"].is_array()));
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

#[tokio::test]
async fn enforce_mode_rejects_forged_trusted_labels_on_web_source() {
    let app = app_with_modes(
        EnforcementMode::Enforce,
        EnforcementMode::Off,
        EnforcementMode::Off,
    );

    let resp = app
        .oneshot(post_json("/v1/events", &forged_trusted_web_send_event()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
    assert!(decision
        .findings
        .iter()
        .any(|finding| finding.evidence["risk_code"] == "untrusted_control"));
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
    assert_eq!(decision.reason, decision.findings[0].reason);
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

fn post_json_as(uri: &str, body: &serde_json::Value, workspace_id: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-featherlane-ai-workspace-id", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn post_json_as_owner(
    uri: &str,
    body: &serde_json::Value,
    workspace_id: &str,
    owner_id: Uuid,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-featherlane-ai-workspace-id", workspace_id)
        .header("x-featherlane-ai-user-id", owner_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Registry entry declaring `recipient` authority-bearing and allowed
/// only from user-origin sources. Rows land in the default workspace, so
/// events exercising them must be submitted there.
async fn register_send_email_metadata(app: &axum::Router, workspace_id: &str, owner_id: Uuid) {
    let resp = app
        .clone()
        .oneshot(post_json_as_owner(
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
            workspace_id,
            owner_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn param_auth_enforce_blocks_wrong_source_for_registered_tool() {
    let (app, workspace_id, owner_id) = app_with_owner_and_settings(settings_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Off,
        EnforcementMode::Enforce,
    ))
    .await;
    register_send_email_metadata(&app, &workspace_id, owner_id).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            &workspace_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
    assert_eq!(decision.reason, decision.findings[0].reason);
    let finding = decision.findings.first().expect("parameter finding");
    assert_eq!(finding.evidence["rule"], "parameter_source.recipient");
    assert!(finding.remediation.is_some());
    assert_eq!(
        finding.evidence["source_chain"],
        serde_json::json!(["src.web"])
    );
}

#[tokio::test]
async fn param_auth_enforce_allows_user_sourced_recipient() {
    let (app, workspace_id, owner_id) = app_with_owner_and_settings(settings_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Off,
        EnforcementMode::Enforce,
    ))
    .await;
    register_send_email_metadata(&app, &workspace_id, owner_id).await;

    let mut event = violating_send_email_event();
    event["provenance"]["recipient"] = json!(["src.user"]);

    let resp = app
        .oneshot(post_json_as("/v1/events", &event, &workspace_id))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn param_auth_shadow_persists_hypothetical_evidence_for_registered_tool() {
    use tl_server::traces::ChannelTraceStore;

    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_modes(
        EnforcementMode::Off,
        EnforcementMode::Off,
        EnforcementMode::Shadow,
    )));
    let (capture, mut rx) = ChannelTraceStore::channel(8);
    state.trace_store = capture;
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Checker Shadow")
        .await
        .unwrap();
    let app = router(state, None, [0u8; 32]);
    register_send_email_metadata(&app, &workspace.id, owner_id).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            &workspace.id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);

    let trace = rx.recv().await.expect("trace enqueued");
    let event = trace.event.expect("event evidence attached");
    // `value_limit` shares the `parameter_auth` enforcement mode, so both
    // checkers run under param shadow; `send_email` declares no value
    // limits, so the value-limit run is present but empty.
    let run = event
        .checks
        .iter()
        .find(|run| run.checker_id == "parameter_auth")
        .expect("parameter_auth run present");
    assert_eq!(run.mode, EnforcementMode::Shadow);
    assert_eq!(run.findings.len(), 1);
    assert_eq!(run.findings[0].rule, "parameter_source.recipient");
    assert_eq!(
        run.findings[0].recommended_effect,
        Some(AuthorizationEffect::Deny)
    );

    let value_run = event
        .checks
        .iter()
        .find(|run| run.checker_id == "value_limit")
        .expect("value_limit run present under shared param mode");
    assert_eq!(value_run.mode, EnforcementMode::Shadow);
    assert!(value_run.findings.is_empty());
}

/// Registry entry for `send_email` requiring admin approval before
/// execution. Rows land in the default workspace.
async fn register_approval_required_metadata(
    app: &axum::Router,
    workspace_id: &str,
    owner_id: Uuid,
) {
    let resp = app
        .clone()
        .oneshot(post_json_as_owner(
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
            workspace_id,
            owner_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn approval_enforce_escalates_tool_requiring_approval() {
    let (app, workspace_id, owner_id) =
        app_with_owner_and_settings(settings_with_approval_mode(EnforcementMode::Enforce)).await;
    register_approval_required_metadata(&app, &workspace_id, owner_id).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &approval_send_email_event(),
            &workspace_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::RequireApproval);
    assert_eq!(decision.reason, decision.findings[0].reason);
    let finding = decision.findings.first().expect("approval finding");
    assert_eq!(finding.evidence["rule"], "approval.send_email");
    assert_eq!(
        finding.remediation.as_deref(),
        Some("request approval from roles: admin before retrying this action")
    );
    assert_eq!(finding.evidence["risk_code"], "approval_required");
}

#[tokio::test]
async fn approval_shadow_keeps_decision_unchanged() {
    let (app, workspace_id, owner_id) =
        app_with_owner_and_settings(settings_with_approval_mode(EnforcementMode::Shadow)).await;
    register_approval_required_metadata(&app, &workspace_id, owner_id).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &approval_send_email_event(),
            &workspace_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
    assert!(decision
        .findings
        .iter()
        .all(|finding| finding.effect == AuthorizationEffect::Permit));
}

#[tokio::test]
async fn approval_uses_the_review_queue_not_the_defer_webhook() {
    use tl_server::EscalationPayload;
    use tokio::sync::mpsc;

    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(FixedSettingsStore(settings_with_approval_mode(
        EnforcementMode::Enforce,
    )));
    let (tx, mut rx) = mpsc::channel::<EscalationPayload>(8);
    state.escalation_tx = Some(tx);
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Approval Escalation")
        .await
        .unwrap();
    let app = router(state, None, [0u8; 32]);
    register_approval_required_metadata(&app, &workspace.id, owner_id).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &approval_send_email_event(),
            &workspace.id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::RequireApproval);

    assert!(decision.approval.is_some());
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn approval_enforce_ignores_tools_without_approval_rules() {
    let (app, workspace_id, owner_id) =
        app_with_owner_and_settings(settings_with_approval_mode(EnforcementMode::Enforce)).await;
    register_send_email_metadata(&app, &workspace_id, owner_id).await;

    let resp = app
        .oneshot(post_json_as(
            "/v1/events",
            &violating_send_email_event(),
            &workspace_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
    assert_eq!(decision.reason, decision.findings[0].reason);
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
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

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
}

#[tokio::test]
async fn environment_mode_lookup_failure_fails_the_request() {
    // An environment may be configured stricter than its workspace, so
    // an override-lookup failure must not silently weaken enforcement:
    // the request fails like a workspace-settings resolution failure.
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
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = read_body(resp).await;
    assert_eq!(body["code"], "internal");
    // Store internals never reach the response body.
    assert_eq!(
        body["message"],
        "environment checker-mode resolution failed"
    );
}
