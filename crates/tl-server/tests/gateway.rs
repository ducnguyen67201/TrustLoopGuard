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
    // Ensure a seal key is available for credential encryption in tests.
    if std::env::var("TL_GATEWAY_CREDENTIAL_KEY").is_err() && std::env::var("TL_API_KEY").is_err() {
        // SAFETY: test process only; no other threads are reading this var.
        unsafe { std::env::set_var("TL_API_KEY", "test-gateway-seal-key") };
    }
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
