use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tl_engine::Engine;
use tl_server::{memory_app_state, router, AuthConfig, MemoryUserStore};
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{header as wire_header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn gateway_owner_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-00000000aa01").unwrap()
}

async fn build_app() -> axum::Router {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let user_store = Arc::new(MemoryUserStore::new());
    user_store
        .insert_approved_for_tests(gateway_owner_id(), "gateway-owner@example.com")
        .await
        .unwrap();
    state.user_store = user_store;
    router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32])
}

async fn read_body(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn read_text(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn json_request(
    method: &str,
    uri: &str,
    token: &str,
    workspace: &str,
    body: Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace)
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn create_workspace_key(app: axum::Router, workspace: &str) -> String {
    let user_id = gateway_owner_id();
    let workspace_name = workspace
        .strip_prefix("ws_")
        .unwrap_or(workspace)
        .replace('_', " ");
    let workspace_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/team/my-workspaces")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-user-id", user_id.to_string())
                .body(Body::from(json!({ "name": workspace_name }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let workspace_status = workspace_resp.status();
    let workspace_body = read_body(workspace_resp).await;
    assert_eq!(
        workspace_status,
        StatusCode::CREATED,
        "workspace creation failed: {workspace_body}"
    );

    let mut create_key_req = json_request(
        "POST",
        "/v1/api-keys",
        "sk-internal",
        workspace,
        json!({ "name": "Gateway runtime" }),
    );
    create_key_req.headers_mut().insert(
        "x-tlg-user-id",
        user_id.to_string().parse().expect("valid user id header"),
    );
    let resp = app.oneshot(create_key_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    read_body(resp).await["plaintext_key"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create_common_gateway_config(
    app: axum::Router,
    workspace: &str,
    base_url: &str,
    kind: &str,
) {
    let provider_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": kind,
                "base_url": base_url,
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(provider_resp.status(), StatusCode::CREATED);
    let provider_body = read_body(provider_resp).await;
    assert!(provider_body.get("provider_api_key").is_none());
    assert!(provider_body.get("encrypted_api_key").is_none());

    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Strict output",
                "input_action": "allow",
                "output_action": "block",
                "fail_mode": "closed",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

    let route_resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/routes",
            "sk-internal",
            workspace,
            json!({
                "id": "route",
                "display_name": "Gateway route",
                "provider_connection_id": "provider",
                "agent_id": "agent",
                "enforcement_profile_id": "profile"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(route_resp.status(), StatusCode::CREATED);
}

/// Flip the common config's enforcement profile into streaming mode so the
/// gateway will emit SSE for `stream:true` requests.
async fn enable_streaming_mode(app: axum::Router, workspace: &str) {
    let resp = app
        .oneshot(json_request(
            "PATCH",
            "/v1/enforcement-profiles/profile",
            "sk-internal",
            workspace,
            json!({ "response_mode": "streaming" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

async fn upsert_block_policy(app: axum::Router, workspace: &str) {
    let policy = r#"
id: block-unsafe-reply
description: Block unsafe provider replies
when:
  channels: [chat]
match:
  literal: unsafe reply
action: block
"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/x-yaml")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", workspace)
                .body(Body::from(policy))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

include!("gateway/provider_flow.rs");

include!("gateway/streaming.rs");

include!("gateway/input_enforcement.rs");

include!("gateway/fail_modes.rs");

include!("gateway/route_validation.rs");

include!("gateway/output_actions.rs");

include!("gateway/regeneration.rs");
