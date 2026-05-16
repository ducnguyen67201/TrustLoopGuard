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
action: block
severity: high
"#;

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
            .body(body.into())
            .unwrap(),
    )
    .await
    .unwrap()
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
        "action": "block",
        "severity": "medium"
    });
    let resp = app
        .oneshot(
            Request::builder()
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

#[tokio::test]
async fn disable_policy_updates_document_but_get_still_works() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["enabled"], false);

    let resp = request(
        app,
        Method::GET,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["enabled"], false);
}

#[tokio::test]
async fn disable_policy_with_malformed_json_returns_api_error() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":"no"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "invalid");
    assert_eq!(body["retriable"], false);
}

#[tokio::test]
async fn delete_policy_makes_get_return_404() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = request(
        app.clone(),
        Method::DELETE,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

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
async fn create_invalid_policy_returns_422_with_validation_details() {
    let app = build_app();
    let resp = request(
        app,
        Method::POST,
        "/v1/policies",
        Body::from(
            r#"
id: "Refund Guarantee"
match:
  regex: "["
action: rewrite
"#,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "unprocessable");
    assert!(body["details"].as_array().unwrap().len() >= 2);
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
