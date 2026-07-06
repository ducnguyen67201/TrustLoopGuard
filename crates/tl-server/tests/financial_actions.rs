use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;

fn app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32])
}

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-tlg-workspace-id", "ws_finance")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn refund_body(idempotency_key: &str, amount_minor: i64) -> Value {
    json!({
        "idempotency_key": idempotency_key,
        "execute": false,
        "action": {
            "kind": "refund",
            "principal_id": "refund-bot",
            "amount": { "amount_minor": amount_minor, "currency": "USD" },
            "counterparty": {
                "id": "cust_456",
                "display_name": "Casey Customer",
                "kind": "customer",
                "country": "US",
                "metadata": {}
            },
            "rail": "card",
            "memo": "refund damaged item",
            "metadata": { "order_id": "order_123" }
        },
        "evidence": []
    })
}

#[tokio::test]
async fn financial_actions_create_get_and_transition() {
    let app = app();
    let created = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            refund_body("idem-refund-75", 7_500),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = json_body(created).await;
    assert_eq!(created["status"], "proposed");
    assert_eq!(created["action"]["kind"], "refund");
    assert_eq!(created["action"]["principal_id"], "refund-bot");
    let action_id = created["id"].as_str().unwrap();

    let duplicate = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            refund_body("idem-refund-75", 7_500),
        ))
        .await
        .unwrap();
    assert_eq!(duplicate.status(), StatusCode::CREATED);
    let duplicate = json_body(duplicate).await;
    assert_eq!(duplicate["id"], action_id);

    let held = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/v1/financial/actions/{action_id}/approve"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(held.status(), StatusCode::OK);
    let held = json_body(held).await;
    assert_eq!(held["status"], "authorized");

    let executed = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/v1/financial/actions/{action_id}/execute"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(executed.status(), StatusCode::OK);
    let executed = json_body(executed).await;
    assert_eq!(executed["status"], "executed");

    let fetched = app
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/actions/{action_id}"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    let fetched = json_body(fetched).await;
    assert_eq!(fetched["status"], "executed");
}

#[tokio::test]
async fn financial_actions_list_workspace_actions() {
    let app = app();
    let first = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            refund_body("idem-list-first", 7_500),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = json_body(first).await;

    let second = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            refund_body("idem-list-second", 8_500),
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::CREATED);
    let second = json_body(second).await;

    let listed = app
        .oneshot(json_request("GET", "/v1/financial/actions", json!({})))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = json_body(listed).await;

    assert_eq!(listed["actions"].as_array().unwrap().len(), 2);
    assert_eq!(listed["actions"][0]["id"], second["id"]);
    assert_eq!(listed["actions"][1]["id"], first["id"]);
}

#[tokio::test]
async fn financial_approval_requests_list_empty_queue() {
    let response = app()
        .oneshot(json_request(
            "GET",
            "/v1/financial/approval-requests",
            json!({}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["approval_requests"], json!([]));
}

#[tokio::test]
async fn financial_actions_validate_missing_amount() {
    let response = app()
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            refund_body("idem-missing-amount", 0),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert_eq!(body["code"], "invalid");
}
