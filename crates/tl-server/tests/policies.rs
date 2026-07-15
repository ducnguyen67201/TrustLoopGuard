//! E2E tests for policy authoring endpoints.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;

const SAMPLE_POLICY_YAML: &str = r#"
id: refund-guarantee
description: Prevents guaranteed refund promises.
match:
  literal: guaranteed refund
action: deny
severity: high
"#;

fn build_app() -> axum::Router {
    let state = memory_app_state(Arc::new(Engine::empty()));
    router(state, None, [0u8; 32])
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }
}

async fn request(
    app: axum::Router,
    method: Method,
    uri: &str,
    body: impl Into<Body>,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/yaml")
            .header("x-tlg-workspace-id", "ws")
            .body(body.into())
            .unwrap(),
    )
    .await
    .unwrap()
}

fn workspace_request() -> axum::http::request::Builder {
    Request::builder().header("x-tlg-workspace-id", "ws")
}

include!("policies/validation.rs");

#[tokio::test]
async fn create_then_get_policy_round_trips_source_yaml() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = read_body(resp).await;
    assert_eq!(body["id"], "refund-guarantee");
    assert_eq!(body["enabled"], true);
    assert_eq!(body["description"], "Prevents guaranteed refund promises.");
    assert!(body["source_yaml"]
        .as_str()
        .unwrap()
        .contains("refund-guarantee"));

    let resp = request(
        app,
        Method::GET,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["id"], "refund-guarantee");
    assert_eq!(body["severity"], "high");
}

#[tokio::test]
async fn list_policies_returns_summaries() {
    let app = build_app();
    for id in ["zeta", "alpha"] {
        let yaml = SAMPLE_POLICY_YAML.replace("refund-guarantee", id);
        let resp = request(app.clone(), Method::POST, "/v1/policies", Body::from(yaml)).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = request(app, Method::GET, "/v1/policies", Body::empty()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    let ids: Vec<_> = body["policies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|policy| policy["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["alpha", "zeta"]);
}

#[tokio::test]
async fn same_policy_id_is_isolated_by_workspace_header() {
    let app = build_app();
    let alpha = SAMPLE_POLICY_YAML.replace("Prevents guaranteed refund promises.", "Alpha policy.");
    let beta = SAMPLE_POLICY_YAML.replace("Prevents guaranteed refund promises.", "Beta policy.");

    for (workspace_id, body) in [("ws_alpha", alpha), ("ws_beta", beta)] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/policies")
                    .header(header::CONTENT_TYPE, "application/yaml")
                    .header("x-tlg-workspace-id", workspace_id)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    for (workspace_id, description) in [("ws_alpha", "Alpha policy."), ("ws_beta", "Beta policy.")]
    {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/policies/refund-guarantee")
                    .header("x-tlg-workspace-id", workspace_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_body(resp).await;
        assert_eq!(body["description"], description);
    }

    let resp = request(
        app,
        Method::GET,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_json_policy_canonicalizes_source_yaml() {
    let app = build_app();
    let body = serde_json::json!({
        "id": "json-policy",
        "description": "JSON policy",
        "match": { "literal": "refund" },
        "action": "deny",
        "severity": "medium"
    });
    let resp = app
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = read_body(resp).await;
    assert_eq!(body["id"], "json-policy");
    assert!(body["source_yaml"]
        .as_str()
        .unwrap()
        .contains("json-policy"));
}

include!("policies/lifecycle.rs");
