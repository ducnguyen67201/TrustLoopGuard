use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tl_engine::Engine;
use tl_server::{memory_app_state, router, AuthConfig};
use tower::ServiceExt;
use wiremock::matchers::{header as wire_header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn build_app() -> axum::Router {
    router(
        memory_app_state(Arc::new(Engine::empty())),
        Some(AuthConfig::new("sk-internal")),
    )
}

async fn read_body(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
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
    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/api-keys",
            "sk-internal",
            workspace,
            json!({ "name": "Gateway runtime" }),
        ))
        .await
        .unwrap();
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

#[tokio::test]
async fn openai_gateway_forwards_with_customer_key_and_blocks_unsafe_output() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "unsafe reply" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_openai";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            "ws_wrong",
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by TrustLoopGuard."
    );
    provider.verify().await;
}

#[tokio::test]
async fn anthropic_gateway_rejects_streaming_before_provider_call() {
    let provider = MockServer::start().await;
    let app = build_app();
    let workspace = "ws_gateway_anthropic";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "anthropic").await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/anthropic/v1/messages",
            &runtime_key,
            workspace,
            json!({
                "model": "claude-3-5-sonnet-latest",
                "stream": true,
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert!(body["message"].as_str().unwrap().contains("streaming"));
    provider.verify().await;
}
