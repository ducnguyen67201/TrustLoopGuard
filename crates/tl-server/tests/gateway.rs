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
use uuid::Uuid;
use wiremock::matchers::{header as wire_header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn build_app() -> axum::Router {
    router(
        memory_app_state(Arc::new(Engine::empty())),
        Some(AuthConfig::new("sk-internal")),
        [0u8; 32],
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
    let user_id = Uuid::new_v4();
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
    assert_eq!(workspace_resp.status(), StatusCode::CREATED);

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
        .clone()
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

    let runs = app
        .oneshot(json_request(
            "GET",
            "/v1/runs",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(runs.status(), StatusCode::OK);
    let runs_body = read_body(runs).await;
    let run = &runs_body["runs"][0];
    assert_eq!(run["agent_id"], "agent");
    assert_eq!(run["kind"], "chat_session");
    assert_eq!(run["status"], "completed");
    assert_eq!(run["trace_count"], 2);
    assert_eq!(run["blocked_count"], 1);

    provider.verify().await;
}

#[tokio::test]
async fn openai_gateway_streams_guarded_response_as_sse() {
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
                "message": { "role": "assistant", "content": "clean reply" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_openai_stream";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "stream": true,
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let body = read_text(resp).await;
    assert!(body.contains("chat.completion.chunk"));
    assert!(body.contains("clean reply"));
    assert!(body.trim_end().ends_with("data: [DONE]"));

    // The upstream provider must have been called WITHOUT the streaming flag,
    // since we buffer-then-guard before emitting SSE.
    let reqs = provider.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let forwarded: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    assert!(forwarded.get("stream").is_none());

    provider.verify().await;
}

#[tokio::test]
async fn openai_gateway_streams_blocked_output_as_sse() {
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
    let workspace = "ws_gateway_openai_stream_block";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "stream": true,
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        resp.headers().get("x-trustloopguard-verdict").unwrap(),
        "blocked"
    );
    let body = read_text(resp).await;
    assert!(body.contains("content_filter"));
    assert!(body.contains("Blocked by TrustLoopGuard."));
    assert!(body.trim_end().ends_with("data: [DONE]"));

    provider.verify().await;
}

#[tokio::test]
async fn anthropic_gateway_streams_guarded_response_as_sse() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(wire_header("x-api-key", "provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_mock",
            "type": "message",
            "role": "assistant",
            "model": "mock-model",
            "content": [{ "type": "text", "text": "scheduling hours are 9 to 5" }],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": { "input_tokens": 1, "output_tokens": 1 }
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_anthropic_stream";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "anthropic").await;

    let resp = app
        .clone()
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

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let body = read_text(resp).await;
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("scheduling hours are 9 to 5"));
    assert!(body.contains("event: message_stop"));

    let reqs = provider.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let forwarded: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    assert!(forwarded.get("stream").is_none());

    provider.verify().await;
}

#[tokio::test]
async fn workspace_runtime_key_cannot_create_gateway_configuration() {
    let provider = MockServer::start().await;
    let app = build_app();
    let workspace = "ws_gateway_runtime_key_admin";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            &runtime_key,
            "ws_wrong",
            json!({
                "id": "provider",
                "display_name": "Runtime key should not manage config",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn gateway_blocks_input_before_reaching_provider() {
    let provider = MockServer::start().await;
    // No mock registered — any call to the provider will fail the test via verify().
    let app = build_app();
    let workspace = "ws_gateway_input_block";

    // Policy that blocks the incoming user message.
    let policy = r#"
id: block-unsafe-input
description: Block unsafe user messages
when:
  channels: [chat]
match:
  literal: unsafe input
action: block
"#;
    let policy_resp = app
        .clone()
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
    assert_eq!(policy_resp.status(), StatusCode::CREATED);

    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    // Profile with input_action: block so a flagged input is blocked before forwarding.
    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Block on input",
                "input_action": "block",
                "output_action": "block",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by input guard.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

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
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let route_resp = app
        .clone()
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

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "unsafe input" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by input guard."
    );
    // Provider must never have been contacted.
    provider.verify().await;
}

#[tokio::test]
async fn metadata_only_retention_still_enforces_input_policy() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "provider response" },
                "finish_reason": "stop"
            }]
        })))
        .expect(0)
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_metadata_enforces";
    let policy = r#"
id: block-retained-input
description: Block unsafe input even when retention omits body
when:
  channels: [chat]
match:
  literal: unsafe retained input
action: block
"#;
    let policy_resp = app
        .clone()
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
    assert_eq!(policy_resp.status(), StatusCode::CREATED);

    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Metadata only",
                "input_action": "block",
                "output_action": "allow",
                "fail_mode": "open",
                "retention_mode": "metadata_only",
                "fallback_message": "Blocked despite metadata-only retention.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

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
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let route_resp = app
        .clone()
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

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "unsafe retained input" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked despite metadata-only retention."
    );
    provider.verify().await;
}

#[tokio::test]
async fn anthropic_gateway_checks_top_level_system_prompt() {
    let provider = MockServer::start().await;
    let app = build_app();
    let workspace = "ws_gateway_anthropic_system";
    let policy = r#"
id: block-anthropic-system
description: Block unsafe Anthropic system instructions
when:
  channels: [chat]
match:
  literal: unsafe system instruction
action: block
"#;
    let policy_resp = app
        .clone()
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
    assert_eq!(policy_resp.status(), StatusCode::CREATED);

    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Block Anthropic system",
                "input_action": "block",
                "output_action": "allow",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked system instruction.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

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
                "kind": "anthropic",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let route_resp = app
        .clone()
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

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/anthropic/v1/messages",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "system": "unsafe system instruction",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["content"][0]["text"], "Blocked system instruction.");
    provider.verify().await;
}

#[tokio::test]
async fn gateway_fail_mode_open_returns_502_on_provider_error() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_fail_open";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Open fail mode",
                "input_action": "allow",
                "output_action": "allow",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Fallback.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

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
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let route_resp = app
        .clone()
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

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn gateway_fail_mode_closed_returns_safe_response_on_provider_error() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503).set_body_string("service unavailable"))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_fail_closed";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Closed fail mode",
                "input_action": "allow",
                "output_action": "allow",
                "fail_mode": "closed",
                "retention_mode": "full_body",
                "fallback_message": "Service temporarily unavailable.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

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
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let route_resp = app
        .clone()
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

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
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
        "Service temporarily unavailable."
    );
}

#[tokio::test]
async fn gateway_returns_404_for_unknown_route() {
    let app = build_app();
    let workspace = "ws_gateway_unknown_route";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn gateway_returns_400_for_provider_kind_mismatch() {
    let provider = MockServer::start().await;
    let app = build_app();
    let workspace = "ws_gateway_kind_mismatch";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    // Create an Anthropic-kind route but call the OpenAI endpoint.
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "anthropic").await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    // Provider must not have been contacted.
    provider.verify().await;
}

#[tokio::test]
async fn gateway_patch_non_existent_resource_returns_404() {
    let app = build_app();
    for (path, body) in [
        (
            "/v1/gateway/provider-connections/no-such",
            json!({"display_name": "x"}),
        ),
        (
            "/v1/enforcement-profiles/no-such",
            json!({"display_name": "x"}),
        ),
        ("/v1/gateway/routes/no-such", json!({"display_name": "x"})),
    ] {
        let resp = app
            .clone()
            .oneshot(json_request(
                "PATCH",
                path,
                "sk-internal",
                "ws_404_test",
                body,
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "expected 404 for PATCH {path}"
        );
    }
}

#[tokio::test]
async fn gateway_create_rejects_empty_required_fields() {
    let app = build_app();

    // Empty display_name on provider connection
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            "ws_validation",
            json!({
                "display_name": "",
                "kind": "openai_compatible",
                "default_model": "gpt-4",
                "provider_api_key": "sk-test"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Empty provider_api_key
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            "ws_validation",
            json!({
                "display_name": "Test",
                "kind": "openai_compatible",
                "default_model": "gpt-4",
                "provider_api_key": "  "
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Empty fallback_message on enforcement profile
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            "ws_validation",
            json!({
                "display_name": "Test",
                "input_action": "block",
                "output_action": "block",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn gateway_redacts_flagged_input_before_forwarding_to_provider() {
    let provider = MockServer::start().await;
    // Mock accepts any POST — presence of a call proves the request was forwarded (not blocked).
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "safe reply"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_redact_input";

    // Policy that flags the literal "unsafe input" phrase.
    let policy = r#"
id: block-unsafe-input-for-redact
description: Flag unsafe user messages
when:
  channels: [chat]
match:
  literal: unsafe input
action: block
"#;
    let policy_resp = app
        .clone()
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
    assert_eq!(policy_resp.status(), StatusCode::CREATED);

    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    // Provider connection, enforcement profile with input_action=redact, and route.
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Redact on input",
                "input_action": "redact",
                "output_action": "block",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
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

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{"role": "user", "content": "unsafe input"}]
            }),
        ))
        .await
        .unwrap();

    // The redact action rewrites the input to "[redacted]" and forwards it.
    // The provider returns "safe reply" which passes the output check.
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["choices"][0]["message"]["content"], "safe reply");
    // verify() fails if the mock was never called — proves request was forwarded, not blocked.
    provider.verify().await;
}

#[tokio::test]
async fn gateway_rewrite_output_falls_back_to_safe_response_when_no_safe_output() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "unsafe reply"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_rewrite_output";
    // Block policy flags "unsafe reply" on output.
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Rewrite on output",
                "input_action": "allow",
                "output_action": "rewrite",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
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

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{"role": "user", "content": "hello"}]
            }),
        ))
        .await
        .unwrap();

    // Provider returns "unsafe reply" → output flagged → rewrite with no safe_output
    // → falls back to safe_response (fallback_message).
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by TrustLoopGuard."
    );
    provider.verify().await;
}

// ── Signal correctness tests ─────────────────────────────────────────────────

#[tokio::test]
async fn openai_blocked_output_has_content_filter_finish_reason_and_headers() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_sig_openai_output";
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
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let phase = resp
        .headers()
        .get("x-trustloopguard-phase")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let trace_id = resp
        .headers()
        .get("x-trustloopguard-trace-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("blocked"));
    assert_eq!(phase.as_deref(), Some("output"));
    assert!(trace_id.is_some());
}

#[tokio::test]
async fn anthropic_blocked_output_has_content_filter_stop_reason_and_headers() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_mock",
            "type": "message",
            "role": "assistant",
            "model": "mock-model",
            "content": [{ "type": "text", "text": "unsafe reply" }],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": { "input_tokens": 1, "output_tokens": 1 }
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_sig_anthropic_output";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "anthropic").await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/anthropic/v1/messages",
            &runtime_key,
            "ws_wrong",
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let phase = resp
        .headers()
        .get("x-trustloopguard-phase")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["stop_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("blocked"));
    assert_eq!(phase.as_deref(), Some("output"));
}

#[tokio::test]
async fn openai_blocked_input_returns_content_filter_with_input_phase_header() {
    let provider = MockServer::start().await;
    // No mock registered — provider must never be called on an input block.

    let app = build_app();
    let workspace = "ws_sig_input_phase";

    let policy = r#"
id: block-bad-input
description: Block bad input
when:
  channels: [chat]
match:
  literal: forbidden input
action: block
"#;
    let _ = app
        .clone()
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

    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Block on input",
                "input_action": "block",
                "output_action": "block",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by input guard.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/routes",
            "sk-internal",
            workspace,
            json!({
                "id": "route",
                "display_name": "Route",
                "provider_connection_id": "provider",
                "agent_id": "agent",
                "enforcement_profile_id": "profile"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "forbidden input" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let phase = resp
        .headers()
        .get("x-trustloopguard-phase")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(phase.as_deref(), Some("input"));
    assert_eq!(verdict.as_deref(), Some("blocked"));
    provider.verify().await; // assert provider was never called
}

#[tokio::test]
async fn escalate_output_action_returns_escalated_verdict_header() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_escalate_header";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Escalate on output",
                "input_action": "allow",
                "output_action": "escalate",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Under review.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/routes",
            "sk-internal",
            workspace,
            json!({
                "id": "route",
                "display_name": "Route",
                "provider_connection_id": "provider",
                "agent_id": "agent",
                "enforcement_profile_id": "profile"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            "ws_wrong",
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("escalated")); // distinct from "blocked"
}

// ── Max regenerations tests ──────────────────────────────────────────────────

#[tokio::test]
async fn max_regenerations_self_heals_on_second_attempt() {
    let provider = MockServer::start().await;

    // Wiremock uses first-registered = highest priority.
    // Register the unsafe-reply mock first so it handles the initial request.
    // It responds at most once, then the safe-reply mock below takes over for the retry.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_1",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&provider)
        .await;

    // Register the safe-reply mock second (lower priority). It handles the regeneration
    // request once the unsafe mock is exhausted.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_2",
            "object": "chat.completion",
            "created": 2,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "safe reply" }, "finish_reason": "stop" }]
        })))
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_regen_self_heal";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Rewrite with regen",
                "input_action": "allow",
                "output_action": "rewrite",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
                "max_regenerations": 2
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/routes",
            "sk-internal",
            workspace,
            json!({
                "id": "route",
                "display_name": "Route",
                "provider_connection_id": "provider",
                "agent_id": "agent",
                "enforcement_profile_id": "profile"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    // No enforcement headers — the clean response passes through unmodified.
    assert!(resp.headers().get("x-trustloopguard-verdict").is_none());
    let body = read_body(resp).await;
    assert_eq!(body["choices"][0]["message"]["content"], "safe reply");
    assert_eq!(body["choices"][0]["finish_reason"], "stop"); // real finish_reason from provider
    provider.verify().await; // exactly 2 provider calls
}

#[tokio::test]
async fn max_regenerations_exhausted_falls_back_to_safe_response() {
    let provider = MockServer::start().await;

    // Both the original call and the retry return "unsafe reply" (always blocked).
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .expect(2) // original + 1 regeneration attempt
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_regen_exhausted";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Rewrite with regen",
                "input_action": "allow",
                "output_action": "rewrite",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
                "max_regenerations": 1
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/routes",
            "sk-internal",
            workspace,
            json!({
                "id": "route",
                "display_name": "Route",
                "provider_connection_id": "provider",
                "agent_id": "agent",
                "enforcement_profile_id": "profile"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let phase = resp
        .headers()
        .get("x-trustloopguard-phase")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by TrustLoopGuard."
    );
    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("blocked"));
    assert_eq!(phase.as_deref(), Some("output"));
    provider.verify().await; // exactly 2 provider calls
}
