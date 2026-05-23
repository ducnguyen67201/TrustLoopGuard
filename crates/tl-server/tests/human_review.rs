use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_engine::Engine;
use tl_server::{memory_app_state, router, AuthConfig};
use tower::ServiceExt;

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

#[tokio::test]
async fn review_event_endpoints_append_and_list_outcomes() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);
    let trace_id = uuid::Uuid::now_v7().to_string();

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/traces/{trace_id}/review-events"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_review")
                .header("x-tlg-user-id", "reviewer-1")
                .body(Body::from(
                    serde_json::json!({
                        "outcome": "corrected",
                        "reason_codes": ["field_mismatch"],
                        "note": "Reviewer corrected the extracted amount.",
                        "metadata": { "workflow_step": "document_extraction" }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = read_body(created).await;
    assert_eq!(created_body["trace_id"], trace_id);
    assert_eq!(created_body["outcome"], "corrected");
    assert_eq!(
        created_body["reason_codes"],
        serde_json::json!(["field_mismatch"])
    );

    let listed = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/traces/{trace_id}/review-events"))
                .header("x-tlg-workspace-id", "ws_review")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = read_body(listed).await;
    assert_eq!(listed_body["review_events"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn human_review_analytics_route_returns_guardrail_and_human_summary() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let analytics = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/analytics/human-review?agent_id=tax-agent&workflow_step=document_extraction")
                .header("x-tlg-workspace-id", "ws_review")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(analytics.status(), StatusCode::OK);
    let body = read_body(analytics).await;
    assert_eq!(body["summary"]["trace_count"], 0);
    assert!(body["by_workflow_step"].as_array().is_some());
    assert!(body["top_reasons"].as_array().is_some());
}

#[tokio::test]
async fn review_endpoints_are_protected_by_bearer_auth() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, Some(AuthConfig::new("sk-correct")), [0u8; 32]);
    let trace_id = uuid::Uuid::now_v7();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/traces/{trace_id}/review-events"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
