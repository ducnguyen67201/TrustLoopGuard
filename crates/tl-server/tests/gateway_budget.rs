//! E2E tests for the gateway LLM budget gate + token metering. Uses
//! `wiremock` as the OpenAI-compatible upstream (mirroring the
//! `tests/gateway.rs` harness) and drives the real router: workspace
//! key → budget policy → proxy → usage events.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

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
                "agent_id": "agent"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(route_resp.status(), StatusCode::CREATED);
}

/// Budget on the `llm_usage` spend meter. `window` is the policy cap
/// field: `daily_minor`, `weekly_minor`, or `monthly_minor`.
async fn create_llm_budget(app: axum::Router, workspace: &str, window: &str, cap_minor: i64) {
    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/financial/policies",
            "sk-internal",
            workspace,
            json!({
                "id": format!("llm-{window}-budget"),
                "description": "LLM spend cap",
                "meter": "llm_usage",
                window: cap_minor
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

/// Weekly budget on the `llm_usage` spend meter.
async fn create_weekly_llm_budget(app: axum::Router, workspace: &str, weekly_minor: i64) {
    create_llm_budget(app, workspace, "weekly_minor", weekly_minor).await;
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

fn mount_delayed_chat_completion(
    model: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
    delay: Duration,
) -> Mock {
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(delay)
                .set_body_json(json!({
                    "id": "chatcmpl_delayed",
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
                })),
        )
}

fn chat_request(runtime_key: &str, workspace: &str, mut body: Value) -> Request<Body> {
    if body.get("max_tokens").is_none() && body.get("max_completion_tokens").is_none() {
        body["max_tokens"] = json!(1_000_000);
    }
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
async fn next_request_is_denied_when_its_maximum_cost_exceeds_remaining_budget() {
    let provider = MockServer::start().await;
    // First call costs just over 499 cents at the built-in output price
    // (110 cents / 1M tokens). The second request can cost more than the
    // remaining cent, so it must be denied before the provider sees it.
    mount_chat_completion("deepseek-chat", 0, 4_536_364)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_overshoot";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 500).await;

    let first_body = json!({
        "model": "deepseek-chat",
        "messages": [{ "role": "user", "content": "hello" }],
        "max_tokens": 4_536_364
    });
    let first = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, first_body))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .clone()
        .oneshot(chat_request(
            &runtime_key,
            workspace,
            json!({
                "model": "deepseek-chat",
                "messages": [{ "role": "user", "content": "next" }],
                "max_tokens": 20_000
            }),
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = read_body(second).await;
    assert_eq!(body["error"]["code"], "budget_exceeded");
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains("maximum"), "message: {message}");
    assert!(message.contains("remaining"), "message: {message}");

    provider.verify().await;
}

#[tokio::test]
async fn unknown_model_fails_closed_when_a_budget_is_active() {
    let provider = MockServer::start().await;
    mount_chat_completion("mystery-1", 500, 100)
        .expect(0)
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
                "messages": [{ "role": "user", "content": "hello" }],
                "max_tokens": 100
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = read_body(resp).await;
    assert_eq!(body["error"]["code"], "pricing_unavailable");

    let usage = list_usage(app, workspace, "").await;
    assert_eq!(usage["events"].as_array().unwrap().len(), 0);

    provider.verify().await;
}

#[tokio::test]
async fn active_budget_requires_a_bounded_output_token_count() {
    let provider = MockServer::start().await;
    mount_chat_completion("deepseek-chat", 10, 10)
        .expect(0)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_requires_max_tokens";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 500).await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "deepseek-chat",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["error"]["code"], "budget_max_tokens_required");

    provider.verify().await;
}

#[tokio::test]
async fn concurrent_requests_cannot_reserve_the_same_remaining_budget() {
    let provider = MockServer::start().await;
    mount_delayed_chat_completion("deepseek-chat", 0, 1_000, Duration::from_millis(150))
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_concurrent_reservation";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    // Each request reserves slightly more than 1.1 cents at the
    // built-in output price. Two reservations cannot fit under 2 cents.
    create_weekly_llm_budget(app.clone(), workspace, 2).await;

    let body = json!({
        "model": "deepseek-chat",
        "messages": [{ "role": "user", "content": "hello" }],
        "max_tokens": 10_000
    });
    let first = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, body.clone()));
    let second = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, body));
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.unwrap().status(), second.unwrap().status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1,
        "statuses: {statuses:?}"
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::TOO_MANY_REQUESTS)
            .count(),
        1,
        "statuses: {statuses:?}"
    );

    provider.verify().await;
}

#[tokio::test]
async fn provider_failure_releases_the_unused_reservation() {
    let provider = MockServer::start().await;
    let attempts = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with({
            let attempts = attempts.clone();
            move |_request: &wiremock::Request| {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    ResponseTemplate::new(500)
                } else {
                    ResponseTemplate::new(200).set_body_json(json!({
                        "id": "chatcmpl_retry",
                        "object": "chat.completion",
                        "created": 1,
                        "model": "deepseek-chat",
                        "choices": [{
                            "index": 0,
                            "message": { "role": "assistant", "content": "safe reply" },
                            "finish_reason": "stop"
                        }],
                        "usage": {
                            "prompt_tokens": 0,
                            "completion_tokens": 1_000,
                            "total_tokens": 1_000
                        }
                    }))
                }
            }
        })
        .expect(2)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_release_on_provider_failure";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_weekly_llm_budget(app.clone(), workspace, 2).await;
    let request = json!({
        "model": "deepseek-chat",
        "messages": [{ "role": "user", "content": "hello" }],
        "max_tokens": 10_000
    });

    let failed = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, request.clone()))
        .await
        .unwrap();
    assert_eq!(failed.status(), StatusCode::BAD_GATEWAY);

    let retried = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, request))
        .await
        .unwrap();
    assert_eq!(retried.status(), StatusCode::OK);

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

/// Daily and monthly caps drive the same verdict path as weekly but
/// through their own window sums (spec: budget windows).
#[tokio::test]
async fn daily_budget_denies_after_spend_reaches_cap() {
    let provider = MockServer::start().await;
    // One call at exactly the cap (137), then the day window is full.
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_daily";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_llm_budget(app.clone(), workspace, "daily_minor", 137).await;

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
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains("daily"), "message: {message}");

    provider.verify().await;
}

#[tokio::test]
async fn monthly_budget_denies_after_spend_reaches_cap() {
    let provider = MockServer::start().await;
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_monthly";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    create_llm_budget(app.clone(), workspace, "monthly_minor", 137).await;

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
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains("monthly"), "message: {message}");

    provider.verify().await;
}

/// Add a second runtime key to an existing workspace.
async fn create_extra_runtime_key(
    app: axum::Router,
    workspace: &str,
    principal_id: &str,
) -> String {
    let mut create_key_req = json_request(
        "POST",
        "/v1/api-keys",
        "sk-internal",
        workspace,
        json!({ "name": format!("Key for {principal_id}"), "principal_id": principal_id }),
    );
    create_key_req.headers_mut().insert(
        "x-tlg-user-id",
        gateway_owner_id()
            .to_string()
            .parse()
            .expect("valid user id header"),
    );
    let resp = app.oneshot(create_key_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    read_body(resp).await["plaintext_key"]
        .as_str()
        .unwrap()
        .to_string()
}

/// A `tl_live_` runtime key must only ever read its own principal's
/// usage — even when it asks for another principal explicitly (spec:
/// runtime keys cannot enumerate workspace-wide spend).
#[tokio::test]
async fn runtime_key_usage_reads_are_scoped_to_its_own_principal() {
    let provider = MockServer::start().await;
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(2)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_usage_read_scope";
    let alice_key = create_workspace_key(app.clone(), workspace, Some("user:alice")).await;
    let bob_key = create_extra_runtime_key(app.clone(), workspace, "user:bob").await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;

    let request_body = json!({
        "model": "deepseek-chat",
        "messages": [{ "role": "user", "content": "hello" }]
    });
    for key in [&alice_key, &bob_key] {
        let resp = app
            .clone()
            .oneshot(chat_request(key, workspace, request_body.clone()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // The internal key sees the whole workspace.
    let all = list_usage(app.clone(), workspace, "").await;
    assert_eq!(all["events"].as_array().unwrap().len(), 2);

    // Alice's runtime key sees only Alice — even asking for Bob.
    for query in ["", "?principal_id=user:bob"] {
        let resp = app
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/v1/llm-usage{query}"),
                &alice_key,
                workspace,
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_body(resp).await;
        let events = body["events"].as_array().unwrap();
        assert_eq!(events.len(), 1, "query {query:?}");
        assert_eq!(events[0]["principal_id"], "user:alice");
    }

    // Grouped rollups are scoped the same way.
    let resp = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/v1/llm-usage?group_by=principal",
            &alice_key,
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let grouped = read_body(resp).await;
    let buckets = grouped["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0]["key"], "user:alice");

    provider.verify().await;
}

/// A typo'd meter value must fail loudly at the API boundary instead of
/// silently creating a policy that never matches (spec: typed meter).
#[tokio::test]
async fn meter_typo_is_rejected_at_policy_creation() {
    let app = build_app().await;
    let workspace = "ws_budget_meter_typo";
    create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/financial/policies",
            "sk-internal",
            workspace,
            json!({
                "id": "llm-budget-typo",
                "meter": "llm_usag",
                "weekly_minor": 1_000
            }),
        ))
        .await
        .unwrap();
    assert!(
        resp.status().is_client_error(),
        "expected 4xx for unknown meter, got {}",
        resp.status()
    );
}

/// `action_kinds`/`rails` describe typed payment actions; on the
/// llm_usage meter they could never match a gateway call, so creation
/// rejects them with a pointed message.
#[tokio::test]
async fn llm_usage_meter_rejects_action_only_selectors() {
    let app = build_app().await;
    let workspace = "ws_budget_meter_selectors";
    create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/financial/policies",
            "sk-internal",
            workspace,
            json!({
                "id": "llm-budget-bad-selectors",
                "meter": "llm_usage",
                "when": { "action_kinds": ["refund"], "rails": ["card"] },
                "weekly_minor": 1_000
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    let message = body["message"].as_str().unwrap().to_string();
    assert!(
        message.contains("action_kinds do not apply to the llm_usage meter"),
        "message: {message}"
    );
    assert!(
        message.contains("rails do not apply to the llm_usage meter"),
        "message: {message}"
    );
}

/// Meter isolation, gateway side: an `actions` policy — even one whose
/// `when.operations` names the old llm.chat_completions string — never
/// gates LLM calls. Only `meter: llm_usage` policies do.
#[tokio::test]
async fn actions_meter_policy_does_not_gate_llm_calls() {
    let provider = MockServer::start().await;
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_meter_isolation";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;

    // Default meter (actions) with a zero cap; under the old
    // string-matching selection this would have denied the request.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/policies",
            "sk-internal",
            workspace,
            json!({
                "id": "actions-zero-cap",
                "when": { "operations": ["llm.chat_completions"] },
                "daily_minor": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

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

#[tokio::test]
async fn metered_llm_spend_crossing_threshold_fires_budget_alert() {
    let provider = MockServer::start().await;
    // deepseek-chat built-in prices: 1M + 1M tokens = 137 USD-minor.
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/alerts"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&receiver)
        .await;

    // build_app + the budget alert delivery worker.
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let user_store = Arc::new(MemoryUserStore::new());
    user_store
        .insert_approved_for_tests(gateway_owner_id(), "budget-owner@example.com")
        .await
        .unwrap();
    state.user_store = user_store;
    #[cfg(feature = "postgres")]
    let (alert_tx, _handle) = tl_server::spawn_webhook_delivery_worker(
        tl_server::RetryPolicy { delays: vec![] },
        16,
        None,
    );
    #[cfg(not(feature = "postgres"))]
    let (alert_tx, _handle) =
        tl_server::spawn_webhook_delivery_worker(tl_server::RetryPolicy { delays: vec![] }, 16);
    state.budget_alert_tx = Some(alert_tx);
    let app = router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]);

    let workspace = "ws_budget_alerting";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;
    // Weekly cap 150: one 137-minor call = 91% ≥ the 80% threshold.
    create_weekly_llm_budget(app.clone(), workspace, 150).await;
    let mut alert_req = json_request(
        "POST",
        "/v1/financial/budget-alerts",
        "sk-internal",
        workspace,
        json!({
            "name": "llm-weekly-80",
            "window": "week",
            "threshold_type": "percent",
            "threshold_value": 80,
            "webhook_url": format!("{}/alerts", receiver.uri())
        }),
    );
    alert_req.headers_mut().insert(
        "x-tlg-user-id",
        gateway_owner_id()
            .to_string()
            .parse()
            .expect("valid user id header"),
    );
    let resp = app.clone().oneshot(alert_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let config = read_body(resp).await;
    let config_id = config["id"].as_str().unwrap().to_string();

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

    // The alert webhook fires with the metered spend.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while receiver
        .received_requests()
        .await
        .unwrap_or_default()
        .is_empty()
    {
        if std::time::Instant::now() > deadline {
            panic!("budget alert webhook never delivered");
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let received = receiver.received_requests().await.unwrap();
    let payload: Value = serde_json::from_slice(&received[0].body).unwrap();
    assert_eq!(payload["type"], "budget_alert");
    assert_eq!(payload["principal_id"], "user:daniel");
    assert_eq!(payload["window"], "week");
    assert_eq!(payload["cap_minor"], 150);
    assert_eq!(payload["spent_minor"], 137);
    assert_eq!(payload["currency"], "USD");

    // Firing history is queryable per config.
    let resp = app
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/budget-alerts/{config_id}/firings"),
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let firings = read_body(resp).await;
    assert_eq!(firings["firings"].as_array().unwrap().len(), 1);

    provider.verify().await;
    receiver.verify().await;
}

/// Internal-key request carrying the owner's forwarded user id — the
/// shape admin-gated surfaces (API keys, settings, LLM pricing) expect.
fn admin_request(method: &str, uri: &str, workspace: &str, body: Value) -> Request<Body> {
    let mut request = json_request(method, uri, "sk-internal", workspace, body);
    request.headers_mut().insert(
        "x-tlg-user-id",
        gateway_owner_id()
            .to_string()
            .parse()
            .expect("valid user id header"),
    );
    request
}

/// Workspace price rows override the built-in defaults for metering,
/// and deleting the row restores the built-in fallback (spec:
/// workspace-editable pricing).
#[tokio::test]
async fn workspace_price_overrides_builtin_and_delete_restores_fallback() {
    let provider = MockServer::start().await;
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .expect(2)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_pricing_override";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;

    // Override deepseek-chat (built-in 27/110 → 40/160).
    let put = app
        .clone()
        .oneshot(admin_request(
            "PUT",
            "/v1/llm-pricing/deepseek-chat",
            workspace,
            json!({ "input_per_million_minor": 40, "output_per_million_minor": 160 }),
        ))
        .await
        .unwrap();
    assert_eq!(put.status(), StatusCode::OK);
    let body = read_body(put).await;
    assert_eq!(body["model"], "deepseek-chat");
    assert_eq!(body["source"], "workspace");
    assert_eq!(body["currency"], "USD");

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

    // Remove the override; the built-in default applies again.
    let delete = app
        .clone()
        .oneshot(admin_request(
            "DELETE",
            "/v1/llm-pricing/deepseek-chat",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);

    let second = app
        .clone()
        .oneshot(chat_request(&runtime_key, workspace, request_body))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    let usage = list_usage(app.clone(), workspace, "").await;
    let events = usage["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    // Newest first: the post-delete call priced at built-in 27+110.
    assert_eq!(events[0]["cost_minor"], 137);
    // The first call priced at the workspace override 40+160.
    assert_eq!(events[1]["cost_minor"], 200);

    // Deleting again is a 404 — no workspace row left.
    let missing = app
        .oneshot(admin_request(
            "DELETE",
            "/v1/llm-pricing/deepseek-chat",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    provider.verify().await;
}

/// A workspace price for a model the built-in table has never heard of
/// meters at the workspace price instead of cost 0.
#[tokio::test]
async fn workspace_price_covers_model_unknown_to_builtins() {
    let provider = MockServer::start().await;
    mount_chat_completion("mystery-1", 1_000_000, 1_000_000)
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_pricing_unknown_model";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;

    let put = app
        .clone()
        .oneshot(admin_request(
            "PUT",
            "/v1/llm-pricing/mystery-1",
            workspace,
            json!({ "input_per_million_minor": 100, "output_per_million_minor": 300 }),
        ))
        .await
        .unwrap();
    assert_eq!(put.status(), StatusCode::OK);

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
    assert_eq!(events[0]["cost_minor"], 400);

    provider.verify().await;
}

/// Negative prices are rejected at the API boundary — a negative price
/// would subtract from accumulated spend and defeat the budget gate.
#[tokio::test]
async fn negative_price_is_rejected_with_400() {
    let app = build_app().await;
    let workspace = "ws_pricing_negative";
    create_workspace_key(app.clone(), workspace, None).await;

    for body in [
        json!({ "input_per_million_minor": -1, "output_per_million_minor": 100 }),
        json!({ "input_per_million_minor": 100, "output_per_million_minor": -1 }),
    ] {
        let resp = app
            .clone()
            .oneshot(admin_request(
                "PUT",
                "/v1/llm-pricing/gpt-4o",
                workspace,
                body,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = read_body(resp).await;
        assert_eq!(body["code"], "invalid");
    }
}

/// Pricing writes are admin-gated like settings: a runtime key must
/// never be able to reprice the spend it is billed under.
#[tokio::test]
async fn runtime_key_cannot_modify_pricing() {
    let app = build_app().await;
    let workspace = "ws_pricing_runtime_key";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;

    let put = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/v1/llm-pricing/gpt-4o",
            &runtime_key,
            workspace,
            json!({ "input_per_million_minor": 0, "output_per_million_minor": 0 }),
        ))
        .await
        .unwrap();
    assert_eq!(put.status(), StatusCode::FORBIDDEN);

    let delete = app
        .oneshot(json_request(
            "DELETE",
            "/v1/llm-pricing/gpt-4o",
            &runtime_key,
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::FORBIDDEN);
}

/// `GET /v1/llm-pricing` returns the effective table: workspace rows
/// merged over built-in defaults, each flagged with its source. The
/// model key is normalized (trimmed lowercase) on write.
#[tokio::test]
async fn effective_pricing_list_flags_sources() {
    let app = build_app().await;
    let workspace = "ws_pricing_list";
    create_workspace_key(app.clone(), workspace, None).await;

    // Mixed case normalizes onto the built-in gpt-4o key.
    let put = app
        .clone()
        .oneshot(admin_request(
            "PUT",
            "/v1/llm-pricing/GPT-4o",
            workspace,
            json!({ "input_per_million_minor": 500, "output_per_million_minor": 2000 }),
        ))
        .await
        .unwrap();
    assert_eq!(put.status(), StatusCode::OK);

    let resp = app
        .oneshot(json_request(
            "GET",
            "/v1/llm-pricing",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    let prices = body["prices"].as_array().unwrap();

    let gpt_4o = prices
        .iter()
        .find(|price| price["model"] == "gpt-4o")
        .expect("gpt-4o row");
    assert_eq!(gpt_4o["source"], "workspace");
    assert_eq!(gpt_4o["input_per_million_minor"], 500);
    assert_eq!(gpt_4o["output_per_million_minor"], 2000);

    let deepseek = prices
        .iter()
        .find(|price| price["model"] == "deepseek-chat")
        .expect("deepseek-chat row");
    assert_eq!(deepseek["source"], "default");
    assert_eq!(deepseek["input_per_million_minor"], 27);

    // No duplicate row for the overridden model.
    assert_eq!(
        prices
            .iter()
            .filter(|price| price["model"] == "gpt-4o")
            .count(),
        1
    );
}

#[tokio::test]
async fn unknown_model_buckets_are_flagged_unpriced() {
    let provider = MockServer::start().await;
    // First call answers with a model no price table knows; the mock is
    // consumed after one use so the second call falls through to the
    // priced deepseek-chat mock below.
    mount_chat_completion("totally-unknown-model", 1_000, 1_000)
        .up_to_n_times(1)
        .mount(&provider)
        .await;
    mount_chat_completion("deepseek-chat", 1_000_000, 1_000_000)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_budget_unpriced";
    let runtime_key = create_workspace_key(app.clone(), workspace, Some("user:daniel")).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri()).await;

    for _ in 0..2 {
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
    }

    // Adding a price after the zero-cost call must not erase the
    // historical undercount signal for this window.
    let put = app
        .clone()
        .oneshot(admin_request(
            "PUT",
            "/v1/llm-pricing/totally-unknown-model",
            workspace,
            json!({ "input_per_million_minor": 100, "output_per_million_minor": 300 }),
        ))
        .await
        .unwrap();
    assert_eq!(put.status(), StatusCode::OK);

    let grouped = list_usage(app, workspace, "?group_by=model").await;
    let buckets = grouped["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 2);

    let unknown = buckets
        .iter()
        .find(|bucket| bucket["key"] == "totally-unknown-model")
        .expect("unknown-model bucket");
    assert_eq!(unknown["unpriced"], true);
    assert_eq!(unknown["cost_minor"], 0);

    let priced = buckets
        .iter()
        .find(|bucket| bucket["key"] == "deepseek-chat")
        .expect("priced bucket");
    // Priced buckets omit the key entirely (skip_serializing_if), they
    // do not carry `unpriced: false`.
    assert!(priced.get("unpriced").is_none());
    assert_eq!(priced["cost_minor"], 137);
}
