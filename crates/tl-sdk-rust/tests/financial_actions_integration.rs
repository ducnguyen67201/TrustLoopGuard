use std::time::Duration;

use tl_sdk_rust::{
    AuthorizationEffect, AuthorizationIntentStatus, Client, FinancialActionKind,
    FinancialExecutionStatus, FinancialOperation, FinancialRail, MoneyAmount, RetryConfig,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> Client {
    Client::new(server.uri()).with_retry(RetryConfig {
        max_attempts: 1,
        total_budget: Duration::from_millis(50),
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    })
}

fn record() -> serde_json::Value {
    serde_json::json!({
        "id": "action-1",
        "workspace_id": "workspace-1",
        "environment_id": "production",
        "authorization_intent_id": "intent-1",
        "authorization_receipt_id": "receipt-1",
        "authorization_effect": "permit",
        "authorization_status": "authorized",
        "execution_status": "not_started",
        "action": {
            "id": "action-1",
            "kind": "payment",
            "operation": "pay",
            "principal_id": "agent-1",
            "amount": { "amount_minor": 100, "currency": "USD" },
            "rail": "internal",
            "metadata": null
        },
        "evidence": [],
        "created_at": "2026-07-14T00:00:00Z",
        "updated_at": "2026-07-14T00:00:00Z"
    })
}

#[test]
fn operation_builder_has_no_private_mandate_contract() {
    let request = FinancialOperation::new(
        "pay",
        FinancialActionKind::Payment,
        "agent-1",
        FinancialRail::Internal,
    )
    .build_request(
        "idem-1",
        MoneyAmount {
            amount_minor: 100,
            currency: "USD".into(),
        },
        None,
        None,
        serde_json::json!({}),
        Vec::new(),
        false,
    );
    let value = serde_json::to_value(request).unwrap();
    assert!(value["action"].get("mandate").is_none());
}

#[tokio::test]
async fn verify_action_decodes_unified_projection() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/financial/actions"))
        .respond_with(ResponseTemplate::new(201).set_body_json(record()))
        .mount(&server)
        .await;
    let request = FinancialOperation::new(
        "pay",
        FinancialActionKind::Payment,
        "agent-1",
        FinancialRail::Internal,
    )
    .build_request(
        "idem-1",
        MoneyAmount {
            amount_minor: 100,
            currency: "USD".into(),
        },
        None,
        None,
        serde_json::json!({}),
        Vec::new(),
        false,
    );
    let result = client(&server).verify_action(&request).await.unwrap();
    assert_eq!(result.authorization_effect, AuthorizationEffect::Permit);
    assert_eq!(
        result.authorization_status,
        AuthorizationIntentStatus::Authorized
    );
    assert_eq!(
        result.execution_status,
        FinancialExecutionStatus::NotStarted
    );
}
