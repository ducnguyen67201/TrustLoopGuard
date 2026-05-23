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
async fn analytics_catalog_query_and_saved_views_round_trip() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

    let catalog = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/analytics/catalog")
                .header("x-tlg-workspace-id", "ws_analytics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(catalog.status(), StatusCode::OK);
    let catalog_body = read_body(catalog).await;
    assert!(catalog_body["metrics"].as_array().unwrap().len() >= 3);

    let query = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/analytics/query")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_analytics")
                .body(Body::from(
                    serde_json::json!({
                        "metric": "trace_count",
                        "group_by": "decision",
                        "filters": []
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(query.status(), StatusCode::OK);
    let query_body = read_body(query).await;
    assert_eq!(query_body["metric"], "trace_count");

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/analytics/views")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_analytics")
                .body(Body::from(
                    serde_json::json!({
                        "name": "Ops view",
                        "is_default": true,
                        "config": {
                            "filters": [],
                            "widgets": [{
                                "id": "trace-volume",
                                "title": "Trace volume",
                                "metric": "trace_count",
                                "chart_type": "bar",
                                "group_by": "decision"
                            }]
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = read_body(created).await;
    assert_eq!(created_body["name"], "Ops view");
    assert_eq!(created_body["config"]["widgets"][0]["layout"]["w"], 6);
    let view_id = created_body["id"].as_str().unwrap();

    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/v1/analytics/views/{view_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_analytics")
                .body(Body::from(
                    serde_json::json!({ "name": "Ops view v2" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_body = read_body(updated).await;
    assert_eq!(updated_body["name"], "Ops view v2");
}

#[tokio::test]
async fn analytics_endpoints_are_protected_by_bearer_auth() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, Some(AuthConfig::new("sk-correct")));

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/analytics/catalog")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
