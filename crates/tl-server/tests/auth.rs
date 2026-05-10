//! E2E tests for the bearer-auth middleware. We invoke the router as a
//! tower service via `oneshot` so the tests don't need a real TCP
//! listener — pure in-process Service::call.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::ApiError;
use tl_engine::Engine;
use tl_server::{router, AppState, AuthConfig};
use tower::ServiceExt;

fn build_app(auth: Option<Arc<AuthConfig>>) -> axum::Router {
    let state = AppState {
        engine: Arc::new(Engine::empty()),
    };
    router(state, auth, None)
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
