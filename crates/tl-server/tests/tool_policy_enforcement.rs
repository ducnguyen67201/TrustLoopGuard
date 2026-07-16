//! HTTP component coverage for the unified tool-policy runtime path.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{AuthorizationDecision, AuthorizationEffect, DEFAULT_WORKSPACE_ID};
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;
use uuid::Uuid;

const ROOT_DELETE_DENY: &str = r#"
family: tool
id: block-system-delete
description: Block recursive deletion of root and system paths.
severity: critical
when:
  side_effects: [shell_exec]
match:
  all:
    - fact: { key: shell.risk, equals: filesystem_recursive_delete }
    - fact: { key: shell.target_scope, one_of: [root, system] }
action: deny
reason: System deletion is prohibited.
remediation: Use a disposable workspace path.
"#;

const APPROVAL_POLICY: &str = r#"
family: tool
id: approve-recursive-delete
description: Review recursive workspace deletion.
severity: high
when:
  side_effects: [shell_exec]
match:
  fact: { key: shell.risk, equals: filesystem_recursive_delete }
action: require_approval
reason: Recursive deletion requires review.
approver_roles: [owner, admin]
max_grant_ttl_seconds: 600
"#;

fn app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32])
}

async fn app_with_owner() -> (axum::Router, String, Uuid) {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Tool Policy Enforcement")
        .await
        .unwrap();
    (router(state, None, [0u8; 32]), workspace.id, owner_id)
}

async fn read_body(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    content_type: &'static str,
    body: impl Into<Body>,
) -> (StatusCode, serde_json::Value) {
    request_in_workspace(
        app,
        DEFAULT_WORKSPACE_ID,
        None,
        method,
        uri,
        content_type,
        body,
    )
    .await
}

async fn request_in_workspace(
    app: &axum::Router,
    workspace_id: &str,
    user_id: Option<Uuid>,
    method: Method,
    uri: &str,
    content_type: &'static str,
    body: impl Into<Body>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, content_type)
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-environment-id", "production");
    if let Some(user_id) = user_id {
        builder = builder.header("x-tlg-user-id", user_id.to_string());
    }
    let response = app
        .clone()
        .oneshot(builder.body(body.into()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    (status, read_body(response).await)
}

async fn install_policy(app: &axum::Router, yaml: &str) {
    let (status, body) = request(
        app,
        Method::POST,
        "/v1/policies",
        "application/yaml",
        Body::from(yaml.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "policy response: {body}");
}

async fn install_policy_in_workspace(app: &axum::Router, workspace_id: &str, yaml: &str) {
    let (status, body) = request_in_workspace(
        app,
        workspace_id,
        None,
        Method::POST,
        "/v1/policies",
        "application/yaml",
        Body::from(yaml.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "policy response: {body}");
}

fn shell_event(command: &str, invocation_id: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "shell.action.proposed",
        "principal": {
            "workspace_id": "collector-claim-is-overridden",
            "environment_id": "collector-claim-is-overridden",
            "agent_id": "coding-agent",
            "session_id": "session-1"
        },
        "action": {
            "operation": "Bash",
            "parameters": {
                "command": command,
                "shell": "bash",
                "cwd": "/workspace/project",
                "workspace_root": "/workspace/project",
                "run_in_background": false
            },
            "side_effect": "read",
            "invocation_id": invocation_id,
            "tool_identity": {
                "server_id": "claude-code",
                "tool_name": "Bash",
                "schema_hash": "sha256:test-bash"
            }
        },
        "sources": [],
        "provenance": {},
        "context": { "channel": "claude-code" }
    })
}

async fn submit(app: &axum::Router, event: &serde_json::Value) -> AuthorizationDecision {
    let (status, body) = request(
        app,
        Method::POST,
        "/v1/events",
        "application/json",
        Body::from(event.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "event response: {body}");
    serde_json::from_value(body).unwrap()
}

async fn submit_in_workspace(
    app: &axum::Router,
    workspace_id: &str,
    event: &serde_json::Value,
) -> AuthorizationDecision {
    let (status, body) = request_in_workspace(
        app,
        workspace_id,
        None,
        Method::POST,
        "/v1/events",
        "application/json",
        Body::from(event.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "event response: {body}");
    serde_json::from_value(body).unwrap()
}

#[tokio::test]
async fn no_policy_retains_permit_and_shell_parameters_are_validated() {
    let app = app();
    let decision = submit(&app, &shell_event("rm -rf /", "tool-no-policy")).await;
    assert_eq!(decision.effect, AuthorizationEffect::Permit);

    let mut invalid = shell_event("echo safe", "tool-invalid");
    invalid["action"]["parameters"]["timeout_ms"] = serde_json::json!(0);
    let (status, body) = request(
        &app,
        Method::POST,
        "/v1/events",
        "application/json",
        Body::from(invalid.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "response: {body}");
}

#[tokio::test]
async fn enabled_policy_denies_nested_root_delete_but_not_quoted_lookalikes() {
    let app = app();
    install_policy(&app, ROOT_DELETE_DENY).await;

    let direct = submit(&app, &shell_event("rm -rf /", "tool-direct")).await;
    assert_eq!(direct.effect, AuthorizationEffect::Deny);
    assert_eq!(
        direct.findings[0].policy_id.as_deref(),
        Some("block-system-delete")
    );
    assert!(direct.lease.is_none());

    let nested = submit(&app, &shell_event("bash -c 'rm -rf /'", "tool-nested")).await;
    assert_eq!(nested.effect, AuthorizationEffect::Deny);

    let safe = submit(&app, &shell_event("echo 'rm -rf /'", "tool-lookalike")).await;
    assert_eq!(safe.effect, AuthorizationEffect::Permit);

    let (_, traces) = request(
        &app,
        Method::GET,
        "/v1/traces?limit=20",
        "application/json",
        Body::empty(),
    )
    .await;
    let direct_trace = traces["traces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trace| trace["trace_id"] == direct.trace_id)
        .unwrap();
    assert_eq!(
        direct_trace["payload"]["triggered_policies"][0]["id"],
        "block-system-delete"
    );
    assert_eq!(
        direct_trace["payload"]["event"]["action"]["side_effect"],
        "shell_exec"
    );
}

#[tokio::test]
async fn parameter_policy_and_partial_fact_analysis_are_enforced() {
    let app = app();
    install_policy(
        &app,
        r#"
family: tool
id: block-company-destroy
when: { side_effects: [shell_exec] }
match:
  parameter: { path: /command, regex: '(?i)acme-prod\s+destroy' }
action: deny
reason: Production destroy is prohibited.
"#,
    )
    .await;
    let parameter_match = submit(
        &app,
        &shell_event("acme-prod destroy service", "tool-parameter"),
    )
    .await;
    assert_eq!(parameter_match.effect, AuthorizationEffect::Deny);

    install_policy(
        &app,
        r#"
family: tool
id: block-device-write
when: { side_effects: [shell_exec] }
match:
  fact: { key: shell.risk, equals: disk_overwrite }
action: deny
reason: Device writes are prohibited.
"#,
    )
    .await;
    let partial = submit(&app, &shell_event("bash -c 'dd if=x", "tool-partial")).await;
    assert_eq!(partial.effect, AuthorizationEffect::Defer);
    assert!(partial
        .findings
        .iter()
        .any(|finding| finding.policy_id.as_deref() == Some("block-device-write")));
}

#[tokio::test]
async fn exact_approval_resumes_once_and_completes_the_lease() {
    let (app, workspace_id, owner_id) = app_with_owner().await;
    install_policy_in_workspace(&app, &workspace_id, APPROVAL_POLICY).await;
    let event = shell_event("rm -rf ./build", "tool-approved");

    let pending = submit_in_workspace(&app, &workspace_id, &event).await;
    assert_eq!(pending.effect, AuthorizationEffect::RequireApproval);
    let approval = pending.approval.expect("approval summary");

    let (status, decided) = request_in_workspace(
        &app,
        &workspace_id,
        Some(owner_id),
        Method::POST,
        &format!("/v1/authorization/approvals/{}/decide", approval.id),
        "application/json",
        Body::from(
            serde_json::json!({
                "decision": "approve",
                "mode": "exact_once",
                "envelope_hash": approval.envelope_hash
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "approval response: {decided}");
    let grant_id = decided["grant"]["id"].as_str().unwrap();

    let mut resumed = event.clone();
    resumed["action"]["authorization"] = serde_json::json!({
        "grant_id": grant_id,
        "attempt_id": "00000000-0000-4000-8000-000000000001"
    });
    let permitted = submit_in_workspace(&app, &workspace_id, &resumed).await;
    assert_eq!(permitted.effect, AuthorizationEffect::Permit);
    let lease = permitted.lease.expect("execution lease");

    let mut changed = event;
    changed["action"]["parameters"]["command"] = serde_json::json!("rm -rf ./other");
    changed["action"]["authorization"] = serde_json::json!({
        "grant_id": grant_id,
        "attempt_id": "00000000-0000-4000-8000-000000000002"
    });
    let (changed_status, changed_body) = request_in_workspace(
        &app,
        &workspace_id,
        None,
        Method::POST,
        "/v1/events",
        "application/json",
        Body::from(changed.to_string()),
    )
    .await;
    if changed_status == StatusCode::OK {
        let changed_decision: AuthorizationDecision = serde_json::from_value(changed_body).unwrap();
        assert_ne!(changed_decision.effect, AuthorizationEffect::Permit);
    } else {
        assert_eq!(
            changed_status,
            StatusCode::CONFLICT,
            "response: {changed_body}"
        );
    }

    let (status, completed) = request_in_workspace(
        &app,
        &workspace_id,
        None,
        Method::POST,
        &format!("/v1/authorization/leases/{}/complete", lease.id),
        "application/json",
        Body::from(r#"{"status":"consumed","outcome":{"success":true}}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "lease response: {completed}");
    assert_eq!(completed["status"], "consumed");
}

#[tokio::test]
async fn disabled_tool_policy_does_not_enforce() {
    let app = app();
    install_policy(&app, ROOT_DELETE_DENY).await;
    let (status, body) = request(
        &app,
        Method::PATCH,
        "/v1/policies/block-system-delete/enabled",
        "application/json",
        Body::from(r#"{"enabled":false}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "disable response: {body}");

    let decision = submit(&app, &shell_event("rm -rf /", "tool-disabled")).await;
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
}
