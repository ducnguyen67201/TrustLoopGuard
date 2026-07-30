//! E2E tests for policy authoring endpoints.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use tl_core::WorkspaceRole;
use tl_engine::Engine;
use tl_server::dashboard_admin::NewApiKey;
use tl_server::{memory_app_state, router, AuthConfig};
use tower::ServiceExt;
use uuid::Uuid;

const SAMPLE_POLICY_YAML: &str = r#"
id: refund-guarantee
description: Prevents guaranteed refund promises.
match:
  literal: guaranteed refund
action: deny
severity: high
"#;

fn owner_id() -> Uuid {
    Uuid::from_u128(1)
}

async fn build_app() -> axum::Router {
    let state = memory_app_state(Arc::new(Engine::empty()));
    for name in ["ws", "alpha", "beta"] {
        state
            .team_store
            .create_workspace(owner_id(), name)
            .await
            .unwrap();
    }
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
            .header("x-tlg-workspace-id", "ws_ws")
            .header("x-tlg-user-id", owner_id().to_string())
            .body(body.into())
            .unwrap(),
    )
    .await
    .unwrap()
}

fn workspace_request() -> axum::http::request::Builder {
    Request::builder()
        .header("x-tlg-workspace-id", "ws_ws")
        .header("x-tlg-user-id", owner_id().to_string())
}

include!("policies/validation.rs");

#[tokio::test]
async fn create_then_get_policy_round_trips_source_yaml() {
    let app = build_app().await;
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
    let app = build_app().await;
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
    let app = build_app().await;
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
                    .header("x-tlg-user-id", owner_id().to_string())
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
                    .header("x-tlg-user-id", owner_id().to_string())
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
    let app = build_app().await;
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

#[tokio::test]
async fn runtime_key_cannot_mutate_policies() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Policy Runtime Key")
        .await
        .unwrap();
    let runtime_key = "tl_live_policy_mutation_test";
    let key_hash = Sha256::digest(runtime_key.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    state
        .api_key_store
        .create(NewApiKey {
            id: "key_policy_mutation_test".into(),
            workspace_id: workspace.id,
            environment_id: "production".into(),
            name: "Policy mutation test".into(),
            key_prefix: "tl_live_policy".into(),
            key_hash,
            created_by_user_id: Some(owner_id),
            principal_id: Some("agent:test".into()),
        })
        .await
        .unwrap();
    let app = router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]);

    let mutations = [
        (
            Method::POST,
            "/v1/policies",
            "application/yaml",
            SAMPLE_POLICY_YAML,
        ),
        (
            Method::PATCH,
            "/v1/policies/refund-guarantee/enabled",
            "application/json",
            r#"{"enabled":false}"#,
        ),
        (
            Method::PATCH,
            "/v1/policies/batch/enabled",
            "application/json",
            r#"{"ids":["refund-guarantee"],"enabled":false}"#,
        ),
        (
            Method::DELETE,
            "/v1/policies/refund-guarantee",
            "application/json",
            "",
        ),
    ];

    for (method, uri, content_type, body) in mutations {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::AUTHORIZATION, format!("Bearer {runtime_key}"))
                    .header("x-tlg-user-id", owner_id.to_string())
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{uri} must reject workspace runtime keys"
        );
    }
}

#[tokio::test]
async fn viewer_cannot_mutate_policies() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let owner_id = Uuid::new_v4();
    let viewer_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Policy Viewer")
        .await
        .unwrap();
    state
        .team_store
        .add_member_or_invite(
            &workspace.id,
            "viewer@example.com",
            WorkspaceRole::Viewer,
            Some(owner_id),
        )
        .await
        .unwrap();
    state
        .team_store
        .accept_pending_invites_for_email("viewer@example.com", viewer_id)
        .await
        .unwrap();
    let app = router(state, None, [0u8; 32]);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .header("x-tlg-workspace-id", workspace.id)
                .header("x-tlg-user-id", viewer_id.to_string())
                .body(Body::from(SAMPLE_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
