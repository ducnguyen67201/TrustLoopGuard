//! Service-level coverage for financial action orchestration.

use std::sync::Arc;

use tl_core::{
    CounterpartyRef, CreateFinancialActionRequest, FinancialAction, FinancialActionKind,
    FinancialActionStatus, FinancialRail, MoneyAmount,
};
use tl_server::{FinancialAuthorizationService, FinancialStoreError, MemoryFinancialStore};

fn refund_request(idempotency_key: &str, amount_minor: i64) -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: idempotency_key.into(),
        execute: false,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            principal_id: "refund-bot".into(),
            amount: MoneyAmount {
                amount_minor,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: "cust_456".into(),
                display_name: Some("Casey Customer".into()),
                kind: "customer".into(),
                country: Some("US".into()),
                metadata: serde_json::json!({}),
            }),
            rail: FinancialRail::Card,
            mandate: None,
            memo: Some("refund damaged item".into()),
            metadata: serde_json::json!({ "order_id": "order_123" }),
        },
        evidence: vec![],
    }
}

fn service() -> FinancialAuthorizationService {
    FinancialAuthorizationService::new(Arc::new(MemoryFinancialStore::new()))
}

#[tokio::test]
async fn service_creates_idempotent_action_and_advances_status() {
    let service = service();
    let created = service
        .create_action("ws_finance", refund_request("idem-refund-75", 7_500))
        .await
        .unwrap();
    assert_eq!(created.status, FinancialActionStatus::Proposed);
    assert_eq!(created.action.id.as_deref(), Some(created.id.as_str()));

    let duplicate = service
        .create_action("ws_finance", refund_request("idem-refund-75", 7_500))
        .await
        .unwrap();
    assert_eq!(duplicate.id, created.id);

    let authorized = service
        .approve_action("ws_finance", &created.id)
        .await
        .unwrap();
    assert_eq!(authorized.status, FinancialActionStatus::Authorized);

    let executed = service
        .execute_action("ws_finance", &created.id)
        .await
        .unwrap();
    assert_eq!(executed.status, FinancialActionStatus::Executed);

    let fetched = service.get_action("ws_finance", &created.id).await.unwrap();
    assert_eq!(fetched.status, FinancialActionStatus::Executed);
}

#[tokio::test]
async fn service_denies_pending_action() {
    let service = service();
    let created = service
        .create_action("ws_finance", refund_request("idem-deny", 7_500))
        .await
        .unwrap();

    let denied = service
        .deny_action("ws_finance", &created.id)
        .await
        .unwrap();

    assert_eq!(denied.status, FinancialActionStatus::Denied);
}

#[tokio::test]
async fn service_lists_workspace_actions_newest_first() {
    let service = service();
    let first = service
        .create_action("ws_finance", refund_request("idem-first", 7_500))
        .await
        .unwrap();
    let second = service
        .create_action("ws_finance", refund_request("idem-second", 8_500))
        .await
        .unwrap();
    service
        .create_action("ws_other", refund_request("idem-other", 9_500))
        .await
        .unwrap();

    let listed = service.list_actions("ws_finance").await.unwrap();

    assert_eq!(listed.actions.len(), 2);
    assert_eq!(listed.actions[0].id, second.id);
    assert_eq!(listed.actions[1].id, first.id);
}

#[tokio::test]
async fn service_validates_action_before_storage_transition() {
    let error = service()
        .create_action("ws_finance", refund_request("idem-invalid", 0))
        .await
        .unwrap_err();

    assert!(matches!(error, FinancialStoreError::Validation(_)));
}
