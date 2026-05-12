//! E2E tests for policy authoring endpoints.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;

fn build_app() -> axum::Router {
    let state = memory_app_state(Arc::new(Engine::empty()));
    router(state, None)
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
async fn validate_policy_yaml_returns_valid_true() {
    let app = build_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies/validate")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(
                    r#"
id: refund-guarantee
match:
  literal: "guaranteed refund"
action: block
"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["valid"], true);
    assert_eq!(body["policy_id"], "refund-guarantee");
    assert_eq!(body["errors"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn validate_policy_yaml_returns_structured_errors() {
    let app = build_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies/validate")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(
                    r#"
id: "Refund Guarantee"
match:
  regex: "["
action: rewrite
"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["valid"], false);
    let paths: Vec<_> = body["errors"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["path"].as_str())
        .collect();
    assert!(paths.contains(&"id"));
    assert!(paths.contains(&"match.regex"));
    assert!(paths.contains(&"rewrite"));
}

#[tokio::test]
async fn validate_policy_json_works() {
    let app = build_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies/validate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"id":"json-policy","match":{"literal":"refund"},"action":"block"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["valid"], true);
    assert_eq!(body["policy_id"], "json-policy");
}

#[tokio::test]
async fn validate_policy_rejects_non_utf8_body() {
    let app = build_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies/validate")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(vec![0xff, 0xfe, 0xfd]))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "invalid");
}
