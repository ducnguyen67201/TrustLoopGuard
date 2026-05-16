//! E2E tests for agent CRUD endpoints. Uses `MemoryAgentStore` so no
//! Postgres / testcontainers needed; runs the router as a tower service
//! via `oneshot`.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{AgentProfile, ApiError};
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;

const SAMPLE_YAML: &str = r#"
agent_id: acme-support-v3
display_name: Acme Support Assistant
scope:
  in_scope:
    - billing questions
    - account access
  out_of_scope:
    - legal advice
authority:
  can_promise:
    - we'll respond within 24h
  cannot_promise:
    - refunds
tone:
  target: warm-professional
  forbidden:
    - sarcastic
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

#[tokio::test]
async fn upsert_yaml_then_get_round_trips() {
    let app = build_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(SAMPLE_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/agents/acme-support-v3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let profile: AgentProfile = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(profile.agent_id, "acme-support-v3");
    assert_eq!(profile.display_name, "Acme Support Assistant");
    assert_eq!(profile.tone.target, "warm-professional");
}

#[tokio::test]
async fn upsert_json_body_works() {
    let app = build_app();
    let body = serde_json::json!({
        "agent_id": "json-agent",
        "display_name": "JSON Agent",
        "scope": { "in_scope": ["x"], "out_of_scope": [] },
        "authority": { "can_promise": [], "cannot_promise": [] },
        "tone": { "target": "neutral", "forbidden": [] },
        "knowledge_sources": [],
        "escalation_triggers": []
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn upsert_overwrites_existing() {
    let app = build_app();
    let v1 = SAMPLE_YAML.replace("Acme Support Assistant", "First Display");
    let v2 = SAMPLE_YAML.replace("Acme Support Assistant", "Second Display");
    for body in [v1, v2] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/agents")
                    .header(header::CONTENT_TYPE, "application/yaml")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/agents/acme-support-v3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let profile: AgentProfile = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(profile.display_name, "Second Display");
}

#[tokio::test]
async fn same_agent_id_is_isolated_by_workspace_header() {
    let app = build_app();
    let alpha = SAMPLE_YAML.replace("Acme Support Assistant", "Alpha Agent");
    let beta = SAMPLE_YAML.replace("Acme Support Assistant", "Beta Agent");

    for (workspace_id, body) in [("ws_alpha", alpha), ("ws_beta", beta)] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/agents")
                    .header(header::CONTENT_TYPE, "application/yaml")
                    .header("x-tlg-workspace-id", workspace_id)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    for (workspace_id, display_name) in [("ws_alpha", "Alpha Agent"), ("ws_beta", "Beta Agent")] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/agents/acme-support-v3")
                    .header("x-tlg-workspace-id", workspace_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let profile: AgentProfile = serde_json::from_value(read_body(resp).await).unwrap();
        assert_eq!(profile.display_name, display_name);
    }

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/agents/acme-support-v3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_then_get_returns_404() {
    let app = build_app();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(SAMPLE_YAML))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/agents/acme-support-v3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/agents/acme-support-v3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: ApiError = serde_json::from_value(read_body(resp).await).unwrap();
    assert!(matches!(body.code, tl_core::ApiErrorCode::NotFound));
}

#[tokio::test]
async fn delete_unknown_yields_404() {
    let app = build_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/agents/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_yaml_yields_400() {
    let app = build_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from("not: valid: yaml: ["))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: ApiError = serde_json::from_value(read_body(resp).await).unwrap();
    assert!(matches!(body.code, tl_core::ApiErrorCode::Invalid));
}

#[tokio::test]
async fn yaml_missing_required_fields_yields_400() {
    // Validation in tl_policy::load_agent_str catches this before we
    // hit our own validate_profile check.
    let app = build_app();
    let body = "agent_id: \"\"\ndisplay_name: x\nscope:\n  in_scope: [a]\nauthority: {}\ntone:\n  target: x\n";
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn json_with_empty_agent_id_yields_422() {
    // JSON path skips tl_policy::load_agent_str so our own validate_profile
    // is what catches the empty agent_id — different status (422 vs 400).
    let app = build_app();
    let body = serde_json::json!({
        "agent_id": "",
        "display_name": "x",
        "scope": { "in_scope": ["a"], "out_of_scope": [] },
        "authority": { "can_promise": [], "cannot_promise": [] },
        "tone": { "target": "x", "forbidden": [] },
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn list_returns_all_agents() {
    let app = build_app();
    for id in ["zeta", "alpha", "mid"] {
        let yaml = SAMPLE_YAML.replace("acme-support-v3", id);
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/agents")
                    .header(header::CONTENT_TYPE, "application/yaml")
                    .body(Body::from(yaml))
                    .unwrap(),
            )
            .await
            .unwrap();
    }
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    let agents = body["agents"].as_array().unwrap();
    let ids: Vec<&str> = agents
        .iter()
        .map(|a| a["agent_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["alpha", "mid", "zeta"]);
}

#[tokio::test]
async fn missing_agent_yields_404() {
    let app = build_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/agents/never-registered")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
