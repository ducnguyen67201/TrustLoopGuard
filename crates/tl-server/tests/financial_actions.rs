use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tl_engine::Engine;
use tl_server::{memory_app_state, router, AppState};
use tower::ServiceExt;
use wiremock::matchers::{header as header_matcher, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEAL_KEY: [u8; 32] = [0u8; 32];

fn app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, SEAL_KEY)
}

fn app_for(state: AppState) -> axum::Router {
    router(state, None, SEAL_KEY)
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

fn payment_http_body(idempotency_key: &str, amount_minor: i64, execute: bool) -> Value {
    json!({
        "idempotency_key": idempotency_key,
        "execute": execute,
        "action": {
            "kind": "payment",
            "principal_id": "payment-bot",
            "amount": { "amount_minor": amount_minor, "currency": "USD" },
            "counterparty": {
                "id": "merchant_123",
                "display_name": "Demo Merchant",
                "kind": "merchant",
                "country": "US",
                "metadata": {}
            },
            "rail": "payment_http",
            "memo": "provider-backed payment",
            "metadata": { "invoice_id": "inv_123" }
        },
        "evidence": []
    })
}

fn financial_policy_body(id: &str) -> Value {
    json!({
        "id": id,
        "description": "Refund controls for support agents",
        "severity": "high",
        "when": {
            "agents": ["refund-bot"],
            "action_kinds": ["refund"],
            "operations": ["issue_refund"],
            "currencies": ["USD"],
            "rails": ["payment_http"]
        },
        "per_transaction_minor": 10000,
        "hold_above_minor": 5000,
        "daily_minor": 50000,
        "monthly_minor": 500000,
        "required_preconditions": [
            "order_exists",
            "amount_lte_refundable_balance"
        ],
        "missing_evidence_action": "escalate",
        "failed_precondition_action": "block",
        "on_breach": "block"
    })
}

async fn create_payment_connection(state: &AppState, base_url: &str) {
    let response = app_for(state.clone())
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            json!({
                "display_name": "Test payments",
                "kind": "payment_http",
                "base_url": base_url,
                "provider_api_key": "test-provider-key"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

fn mandate_body() -> Value {
    json!({
        "id": "mandate_refund_bot",
        "version": 1,
        "principal_id": "refund-bot",
        "scope": {
            "action_kinds": ["refund"],
            "max_amount_minor": 10000,
            "currency": "USD"
        },
        "metadata": { "source": "router_test" },
        "expires_at": "2026-08-05T19:00:00Z"
    })
}

fn outcome_body(action_id: &str, status: &str) -> Value {
    json!({
        "action_id": action_id,
        "status": status,
        "reversal_capability": "manual_recovery",
        "recovery_status": "manual_required",
        "provider_status": "provider_status",
        "provider_reference": "provider_ref_123",
        "occurred_at": "2026-07-05T20:00:00Z",
        "metadata": { "source": "router_test" }
    })
}

#[tokio::test]
async fn financial_mandates_create_list_and_revoke() {
    let app = app();
    let created = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/mandates",
            mandate_body(),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = json_body(created).await;
    assert_eq!(created["status"], "active");
    assert_eq!(created["principal_id"], "refund-bot");
    let mandate_id = created["id"].as_str().unwrap();

    let listed = app
        .clone()
        .oneshot(json_request("GET", "/v1/financial/mandates", json!({})))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = json_body(listed).await;
    assert_eq!(listed["mandates"].as_array().unwrap().len(), 1);
    assert_eq!(listed["mandates"][0]["id"], mandate_id);

    let revoked = app
        .oneshot(json_request(
            "POST",
            &format!("/v1/financial/mandates/{mandate_id}/revoke"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(revoked.status(), StatusCode::OK);
    let revoked = json_body(revoked).await;
    assert_eq!(revoked["status"], "revoked");
}

#[tokio::test]
async fn financial_policies_create_and_list() {
    let app = app();
    let created = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/policies",
            financial_policy_body("refund-controls"),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = json_body(created).await;
    assert_eq!(created["id"], "refund-controls");
    assert_eq!(created["when"]["agents"][0], "refund-bot");
    assert_eq!(created["per_transaction_minor"], 10000);
    assert_eq!(created["enabled"], true);

    let listed = app
        .oneshot(json_request("GET", "/v1/financial/policies", json!({})))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = json_body(listed).await;
    assert_eq!(listed["policies"].as_array().unwrap().len(), 1);
    assert_eq!(listed["policies"][0]["id"], "refund-controls");
}

#[tokio::test]
async fn financial_policy_creation_rejects_non_enforcing_actions() {
    let app = app();
    let mut body = financial_policy_body("bad-refund-controls");
    body["on_breach"] = json!("rewrite");
    let created = app
        .oneshot(json_request("POST", "/v1/financial/policies", body))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::BAD_REQUEST);
    let body = json_body(created).await;
    assert!(body["message"].as_str().unwrap().contains("on_breach"));
}

#[tokio::test]
async fn financial_action_outcomes_record_and_list() {
    let app = app();
    let created = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            refund_body("idem-outcome", 7_500),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = json_body(created).await;
    let action_id = created["id"].as_str().unwrap();

    let recorded = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/v1/financial/actions/{action_id}/outcomes"),
            outcome_body(action_id, "succeeded"),
        ))
        .await
        .unwrap();
    assert_eq!(recorded.status(), StatusCode::CREATED);
    let recorded = json_body(recorded).await;
    assert_eq!(recorded["status"], "succeeded");
    assert_eq!(recorded["provider_reference"], "provider_ref_123");

    let listed = app
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/actions/{action_id}/outcomes"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = json_body(listed).await;
    assert_eq!(listed["outcomes"].as_array().unwrap().len(), 1);
    assert_eq!(listed["outcomes"][0]["status"], "succeeded");
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

    let receipt = app
        .clone()
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/receipts/{action_id}"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(receipt.status(), StatusCode::OK);
    let receipt = json_body(receipt).await;
    assert_eq!(receipt["id"], action_id);
    assert_eq!(receipt["action_id"], action_id);
    assert_eq!(receipt["proof"]["action_status"], "executed");

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
async fn payment_http_execute_uses_vaulted_provider_and_records_proof() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .and(header_matcher("authorization", "Bearer test-provider-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "succeeded",
            "provider_reference": "pay_123",
            "reversal_capability": "provider_reversal",
            "recovery_status": "not_needed"
        })))
        .expect(1)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;

    let created = app_for(state.clone())
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            payment_http_body("idem-provider-success", 4_000, true),
        ))
        .await
        .unwrap();

    assert_eq!(created.status(), StatusCode::CREATED);
    let created = json_body(created).await;
    assert_eq!(created["status"], "executed");
    let action_id = created["id"].as_str().unwrap();
    let requests = provider.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get("idempotency-key").unwrap(),
        action_id
    );
    let provider_body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(provider_body["amount"], 4_000);
    assert_eq!(provider_body["merchant"], "Demo Merchant");

    let receipt = app_for(state.clone())
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/receipts/{action_id}"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(receipt.status(), StatusCode::OK);
    let receipt = json_body(receipt).await;
    assert_eq!(receipt["ledger_event_ids"].as_array().unwrap().len(), 1);
    assert_eq!(receipt["proof"]["provider"]["reference"], json!("pay_123"));
    assert_eq!(
        receipt["proof"]["provider"]["response"]["status"],
        json!("succeeded")
    );

    let outcomes = app_for(state)
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/actions/{action_id}/outcomes"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(outcomes.status(), StatusCode::OK);
    let outcomes = json_body(outcomes).await;
    assert_eq!(outcomes["outcomes"][0]["status"], "succeeded");
    assert_eq!(
        outcomes["outcomes"][0]["provider_reference"],
        json!("pay_123")
    );
}

#[tokio::test]
async fn payment_http_execute_without_provider_fails_honestly() {
    let state = memory_app_state(Arc::new(Engine::empty()));

    let created = app_for(state.clone())
        .oneshot(json_request(
            "POST",
            "/v1/financial/actions",
            payment_http_body("idem-provider-missing", 4_000, true),
        ))
        .await
        .unwrap();

    assert_eq!(created.status(), StatusCode::CREATED);
    let created = json_body(created).await;
    assert_eq!(created["status"], "failed");
    let action_id = created["id"].as_str().unwrap();
    let receipt = app_for(state.clone())
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/receipts/{action_id}"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(receipt.status(), StatusCode::NOT_FOUND);

    let outcomes = app_for(state)
        .oneshot(json_request(
            "GET",
            &format!("/v1/financial/actions/{action_id}/outcomes"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(outcomes.status(), StatusCode::OK);
    let outcomes = json_body(outcomes).await;
    assert_eq!(outcomes["outcomes"][0]["status"], "failed");
    assert_eq!(
        outcomes["outcomes"][0]["metadata"]["reason"],
        "no payment_http provider connection configured"
    );
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
