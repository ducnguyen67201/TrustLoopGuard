//! Integration coverage for Rust SDK financial action helpers.

use std::time::Duration;

use tl_sdk_rust::{
    Client, CounterpartyRef, CreateFinancialActionRequest, FinancialAction, FinancialActionKind,
    FinancialActionStatus, FinancialRail, MoneyAmount, RetryConfig,
};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn one_shot_retry() -> RetryConfig {
    RetryConfig {
        max_attempts: 1,
        total_budget: Duration::from_millis(50),
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    }
}

fn refund_request() -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: "idem-refund-75".into(),
        execute: false,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            principal_id: "refund-bot".into(),
            amount: MoneyAmount {
                amount_minor: 7_500,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: "cust_456".into(),
                display_name: Some("Ada Customer".into()),
                kind: "customer".into(),
                country: Some("US".into()),
                metadata: serde_json::json!({}),
            }),
            rail: FinancialRail::PaymentHttp,
            mandate: None,
            memo: Some("refund damaged item".into()),
            metadata: serde_json::json!({
                "order_id": "order_123",
                "reason": "damaged_item"
            }),
        },
        evidence: vec![],
    }
}

fn action_body(id: &str, status: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "workspace_id": "default",
        "status": status,
        "action": {
            "id": id,
            "kind": "refund",
            "principal_id": "refund-bot",
            "amount": { "amount_minor": 7500, "currency": "USD" },
            "counterparty": {
                "id": "cust_456",
                "display_name": "Ada Customer",
                "kind": "customer",
                "country": "US",
                "metadata": {}
            },
            "rail": "payment_http",
            "memo": "refund damaged item",
            "metadata": {
                "order_id": "order_123",
                "reason": "damaged_item"
            }
        },
        "evidence": [],
        "created_at": "2026-05-17T00:00:00Z",
        "updated_at": "2026-05-17T00:00:00Z"
    })
}

#[tokio::test]
async fn verify_action_posts_typed_request_with_bearer_auth() {
    let server = MockServer::start().await;
    let request = refund_request();
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions"))
        .and(header("authorization", "Bearer secret"))
        .and(body_json(&request))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(action_body("act_refund_75", "proposed")),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri())
        .with_api_key("secret")
        .with_retry(one_shot_retry());
    let action = client.verify_action(&request).await.unwrap();

    assert_eq!(action.id, "act_refund_75");
    assert_eq!(action.status, FinancialActionStatus::Proposed);
    assert_eq!(action.action.kind, FinancialActionKind::Refund);
}

#[tokio::test]
async fn guard_payment_aliases_verify_action_for_payment_ergonomics() {
    let server = MockServer::start().await;
    let request = refund_request();
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions"))
        .and(body_json(&request))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(action_body("act_refund_75", "proposed")),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let action = client.guard_payment(&request).await.unwrap();

    assert_eq!(action.id, "act_refund_75");
}

#[tokio::test]
async fn financial_action_helpers_encode_ids_and_parse_statuses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/financial/actions/action%2Fone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(action_body("action/one", "held")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions/action%2Fone/approve"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(action_body("action/one", "authorized")),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions/action%2Fone/deny"))
        .respond_with(ResponseTemplate::new(200).set_body_json(action_body("action/one", "denied")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions/action%2Fone/execute"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(action_body("action/one", "executed")),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());

    let action = client.get_financial_action("action/one").await.unwrap();
    assert_eq!(action.status, FinancialActionStatus::Held);

    let approved = client.approve_action("action/one").await.unwrap();
    assert_eq!(approved.status, FinancialActionStatus::Authorized);

    let denied = client.deny_action("action/one").await.unwrap();
    assert_eq!(denied.status, FinancialActionStatus::Denied);

    let executed = client.execute_action("action/one").await.unwrap();
    assert_eq!(executed.status, FinancialActionStatus::Executed);
}
