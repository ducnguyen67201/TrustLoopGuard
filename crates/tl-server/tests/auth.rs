//! E2E tests for the bearer-auth middleware. We invoke the router as a
//! tower service via `oneshot` so the tests don't need a real TCP
//! listener — pure in-process Service::call.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{ApiError, Verdict};
use tl_engine::Engine;
use tl_server::{memory_app_state, router, AuthConfig};
use tower::ServiceExt;

fn build_app(auth: Option<Arc<AuthConfig>>) -> axum::Router {
    let state = memory_app_state(Arc::new(Engine::empty()));
    router(state, auth)
}

fn check_request(token: Option<&str>) -> Request<Body> {
    let body = serde_json::json!({
        "agent_id": "a",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello"
    });
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/check")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

fn policy_validate_request(token: Option<&str>) -> Request<Body> {
    let body = r#"
id: pii-block
description: Block obvious PII
when:
  channels: [chat]
match:
  regex: "\\b\\d{3}-\\d{2}-\\d{4}\\b"
action:
  verdict: block
"#;
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/policies/validate")
        .header(header::CONTENT_TYPE, "application/x-yaml");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body)).unwrap()
}

fn create_api_key_request(token: &str, workspace_id: &str, name: &str) -> Request<Body> {
    let body = serde_json::json!({ "name": name });
    Request::builder()
        .method("POST")
        .uri("/v1/api-keys")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }
}

#[tokio::test]
async fn missing_bearer_returns_401_with_api_error_envelope() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app.oneshot(check_request(None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(matches!(body.code, tl_core::ApiErrorCode::Unauthorized));
    assert!(body.message.contains("missing"));
    assert!(!body.retriable);
}

#[tokio::test]
async fn wrong_bearer_returns_401() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app.oneshot(check_request(Some("sk-wrong"))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(body.message.contains("invalid"));
}

#[tokio::test]
async fn correct_bearer_returns_200() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app
        .oneshot(check_request(Some("sk-correct")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["verdict"], "allow");
}

#[tokio::test]
async fn correct_bearer_can_call_policy_authoring_routes() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));

    let resp = app
        .oneshot(policy_validate_request(Some("sk-correct")))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn internal_bearer_can_issue_workspace_key_used_by_sdk_runtime() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request(
            "sk-internal",
            "ws_runtime",
            "SDK integration",
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let plaintext = created["plaintext_key"]
        .as_str()
        .expect("plaintext key is returned once");
    assert!(plaintext.starts_with("tl_live_"));
    assert_eq!(created["api_key"]["name"], "SDK integration");
    assert_eq!(created["api_key"]["status"], "active");
    assert_eq!(created["api_key"]["last_used_at"], serde_json::Value::Null);
    assert!(created["api_key"]["prefix"]
        .as_str()
        .unwrap()
        .starts_with("tl_live_"));
    assert!(created.get("key_hash").is_none());

    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/api-keys")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", "ws_runtime")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let listed = read_body(list_resp).await;
    assert_eq!(listed["api_keys"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed["api_keys"][0]["prefix"],
        created["api_key"]["prefix"]
    );
    assert!(!listed.to_string().contains(plaintext));

    let other_workspace_policy = r#"
id: wrong-workspace-block
description: Would block if caller-controlled workspace won
when:
  channels: [chat]
match:
  literal: deny me
action: block
"#;
    let upsert_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/x-yaml")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", "ws_wrong")
                .body(Body::from(other_workspace_policy))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upsert_resp.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "a",
        "channel": "chat",
        "input": "deny me",
        "proposed_output": "deny me",
        "workspace_id": "ws_wrong"
    });
    let check_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {plaintext}"))
                .header("x-tlg-workspace-id", "ws_wrong")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(check_resp.status(), StatusCode::OK);
    let decision = read_body(check_resp).await;
    assert_eq!(decision["verdict"], serde_json::json!(Verdict::Allow));
}

#[tokio::test]
async fn health_endpoint_works_without_token() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_endpoint_works_with_random_token_too() {
    // Health bypasses auth entirely — extra credential doesn't break it.
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .header(header::AUTHORIZATION, "Bearer literally-anything")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn no_auth_config_disables_middleware() {
    // Local-dev / test mode: auth=None means /v1/check accepts any
    // request (or no Authorization header at all).
    let app = build_app(None);
    let resp = app.oneshot(check_request(None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn malformed_authorization_header_yields_401() {
    // Bare "Bearer" with no token, or a non-Bearer scheme — both should
    // be rejected as missing.
    let app = build_app(Some(AuthConfig::new("sk-correct")));

    for header_value in ["", "Bearer", "Basic Zm9vOmJhcg==", "Token sk-correct"] {
        let req = Request::builder()
            .method("POST")
            .uri("/v1/check")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, header_value)
            .body(Body::from(
                r#"{"agent_id":"a","channel":"chat","input":"x","proposed_output":"y"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "expected 401 for header `{header_value}`"
        );
    }
}
