//! E2E tests for the gateway LLM budget gate + token metering. Uses
//! `wiremock` as the OpenAI-compatible upstream (mirroring the
//! `tests/gateway.rs` harness) and drives the real router: workspace
//! key → budget policy → proxy → usage events.

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
    Uuid::parse_str("00000000-0000-0000-0000-00000000bb01").unwrap()
}

async fn build_app() -> axum::Router {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let user_store = Arc::new(MemoryUserStore::new());
    user_store
        .insert_approved_for_tests(gateway_owner_id(), "budget-owner@example.com")
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

/// Create the workspace + a runtime key, optionally bound to a
/// principal (spec: api-key principal binding).
async fn create_workspace_key(
    app: axum::Router,
    workspace: &str,
    principal_id: Option<&str>,
) -> String {
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
    assert_eq!(workspace_resp.status(), StatusCode::CREATED);

    let mut key_body = json!({ "name": "Budget runtime" });
    if let Some(principal_id) = principal_id {
        key_body["principal_id"] = json!(principal_id);
    }
    let mut create_key_req =
        json_request("POST", "/v1/api-keys", "sk-internal", workspace, key_body);
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

async fn create_common_gateway_config(app: axum::Router, workspace: &str, base_url: &str) {
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
                "base_url": base_url,
                "default_model": "deepseek-chat",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Pass-through",
                "input_action": "allow",
                "output_action": "allow",
                "fail_mode": "open",
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

/// Weekly budget targeting the shared llm.chat_completions operation.
async fn create_weekly_llm_budget(app: axum::Router, workspace: &str, weekly_minor: i64) {
    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/financial/policies",
            "sk-internal",
            workspace,
            json!({
                "id": "llm-weekly-budget",
                "description": "Weekly LLM spend cap",
                "when": { "operations": ["llm.chat_completions"] },
                "weekly_minor": weekly_minor
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

fn mount_chat_completion(model: &str, prompt_tokens: i64, completion_tokens: i64) -> Mock {
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": model,
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "safe reply" },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": prompt_tokens + completion_tokens
            }
        })))
}

fn chat_request(runtime_key: &str, workspace: &str, body: Value) -> Request<Body> {
    json_request(
        "POST",
        "/v1/gateway/route/openai/chat/completions",
        runtime_key,
        workspace,
        body,
    )
}

async fn list_usage(app: axum::Router, workspace: &str, query: &str) -> Value {
    let resp = app
        .oneshot(json_request(
            "GET",
            &format!("/v1/llm-usage{query}"),
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    read_body(resp).await
}

#[tokio::test]
async fn under_cap_forwards_and_meters_with_exact_cost() {
    let provider = MockServer::start().await;
    // 1M prompt @ 27/1M + 1M completion @ 110/1M (deepseek-chat
    // built-in prices) = 137 USD-minor.
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_under_cap";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 1_000).await;

    let resp = app
        .clone()
        .oneshot(chat_request(
            &runtime_key,
            workspace,
            json!({
                "model": "deepseek-chat",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["choices"][0]["message"]["content"], "safe reply");

    let usage = list_usage(app.clone(), workspace, "").await;
    let events = usage["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["principal_id"], "user:daniel");
    assert_eq!(events[0]["model"], "deepseek-chat");
    assert_eq!(events[0]["prompt_tokens"], 1_000_000);
    assert_eq!(events[0]["completion_tokens"], 1_000_000);
    assert_eq!(events[0]["cost_minor"], 137);
    assert_eq!(events[0]["currency"], "USD");

    let grouped = list_usage(app, workspace, "?group_by=principal").await;
    let buckets = grouped["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0]["key"], "user:daniel");
    assert_eq!(buckets[0]["cost_minor"], 137);
    assert_eq!(buckets[0]["calls"], 1);

    provider.verify().await;
}

#[tokio::test]
async fn at_cap_denies_without_calling_upstream() {
    let provider = MockServer::start().await;
    // Cap 0 → the very first request is already at cap; the upstream
    // must never be reached.
    mount_chat_completion("deepseek-chat", 1, 1)
        .expect(0)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_at_cap";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 0).await;

    let resp = app
        .clone()
        .oneshot(chat_request(
            &runtime_key,
            workspace,
            json!({
                "model": "deepseek-chat",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = read_body(resp).await;
    assert_eq!(body["error"]["code"], "budget_exceeded");
    assert_eq!(body["error"]["type"], "insufficient_quota");
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains("user:daniel"), "message: {message}");

    // Nothing metered for a denied request.
    let usage = list_usage(app, workspace, "").await;
    assert_eq!(usage["events"].as_array().unwrap().len(), 0);

    provider.verify().await;
}

#[tokio::test]
async fn spend_reaching_cap_denies_the_next_request() {
    let provider = MockServer::start().await;
    // First call lands exactly at the cap (137); the second must be
    // denied — boundary check: spent == cap denies.
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_overshoot";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 137).await;

    let request_body = json!({
        "model": "deepseek-chat",
        "messages": [{ "role": "user", "content": "hello" }]
    });
    let first = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, request_body.clone()))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, request_body))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = read_body(second).await;
    assert_eq!(body["error"]["code"], "budget_exceeded");

    provider.verify().await;
}

#[tokio::test]
async fn unknown_model_forwards_and_meters_cost_zero() {
    let provider = MockServer::start().await;
    mount_chat_completion("mystery-1", 500, 100)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_unknown_model";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 1_000).await;

    let resp = app
        .clone()
        .oneshot(chat_request(
            &runtime_key,
            workspace,
            json!({
                "model": "mystery-1",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let usage = list_usage(app, workspace, "").await;
    let events = usage["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["model"], "mystery-1");
    assert_eq!(events[0]["prompt_tokens"], 500);
    assert_eq!(events[0]["completion_tokens"], 100);
    // Unknown model → tokens metered honestly, cost 0.
    assert_eq!(events[0]["cost_minor"], 0);

    provider.verify().await;
}

#[tokio::test]
async fn streaming_request_gets_sse_and_is_metered() {
    let provider = MockServer::start().await;
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_streaming";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 1_000).await;

    // Flip the profile into streaming mode so stream:true is accepted.
    let patch = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            "/v1/enforcement-profiles/profile",
            "sk-internal",
            workspace,
            json!({ "response_mode": "streaming" }),
        ))
        .await
        .unwrap();
    assert_eq!(patch.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(chat_request(
            &runtime_key,
            workspace,
            json!({
                "model": "deepseek-chat",
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
    let sse = read_text(resp).await;
    assert!(sse.contains("data:"), "expected SSE body, got: {sse}");
    assert!(sse.contains("[DONE]"));

    // Metering reads the buffered upstream response, not the SSE.
    let usage = list_usage(app, workspace, "").await;
    let events = usage["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["cost_minor"], 137);

    provider.verify().await;
}

#[tokio::test]
async fn key_without_principal_budgets_by_api_key_id() {
    let provider = MockServer::start().await;
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_keyed";
    let runtime_key = create_workspace_key(app.clone(), workspace, None).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 1_000).await;

    let resp = app
        .clone()
        .oneshot(chat_request(
            &runtime_key,
            workspace,
            json!({
                "model": "deepseek-chat",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // The key id is the principal when the key has no bound principal.
    let mut list_keys_req =
        json_request("GET", "/v1/api-keys", "sk-internal", workspace, json!({}));
    list_keys_req.headers_mut().insert(
        "x-tlg-user-id",
        gateway_owner_id()
            .to_string()
            .parse()
            .expect("valid user id header"),
    );
    let keys = app.clone().oneshot(list_keys_req).await.unwrap();
    assert_eq!(keys.status(), StatusCode::OK);
    let key_id = read_body(keys).await["api_keys"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let usage = list_usage(app, workspace, "").await;
    let events = usage["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["principal_id"], key_id);
    assert_eq!(events[0]["api_key_id"], key_id);

    provider.verify().await;
}
