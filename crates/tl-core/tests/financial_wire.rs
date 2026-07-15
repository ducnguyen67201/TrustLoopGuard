use serde_json::json;
use tl_core::{
    AuthorizationClaim, AuthorizationEffect, AuthorizationIntentStatus, CounterpartyRef,
    CreateFinancialActionRequest, EvidenceRef, FinancialAction, FinancialActionKind,
    FinancialActionOutcomeStatus, FinancialActionRecord, FinancialActionState,
    FinancialExecutionStatus, FinancialRail, MoneyAmount, RecoveryStatus, ReversalCapability,
};

#[test]
fn financial_enums_use_canonical_wire_values() {
    assert_eq!(
        serde_json::to_value(FinancialActionKind::InvoiceApproval).unwrap(),
        "invoice_approval"
    );
    assert_eq!(
        serde_json::to_value(FinancialExecutionStatus::Succeeded).unwrap(),
        "succeeded"
    );
    assert_eq!(
        serde_json::to_value(FinancialActionOutcomeStatus::RecoveryStarted).unwrap(),
        "recovery_started"
    );
    assert_eq!(
        serde_json::to_value(ReversalCapability::CancelPendingRefund).unwrap(),
        "cancel_pending_refund"
    );
    assert_eq!(
        serde_json::to_value(RecoveryStatus::ManualRequired).unwrap(),
        "manual_required"
    );
    assert_eq!(
        serde_json::to_value(FinancialActionState::NotExecutable).unwrap(),
        "not_executable"
    );
}

#[test]
fn action_request_uses_the_common_authorization_claim() {
    let request = CreateFinancialActionRequest {
        idempotency_key: "idem-1".into(),
        execute: true,
        authorization: Some(AuthorizationClaim {
            grant_id: "grant-1".into(),
            attempt_id: "attempt-1".into(),
        }),
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            operation: "issue_refund".into(),
            principal_id: "refund-bot".into(),
            amount: MoneyAmount {
                amount_minor: 7_500,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: "cust-1".into(),
                display_name: None,
                kind: "customer".into(),
                country: None,
                metadata: json!({}),
            }),
            rail: FinancialRail::PaymentHttp,
            memo: None,
            metadata: json!({}),
        },
        evidence: vec![EvidenceRef {
            source: "orders".into(),
            source_id: "order-1".into(),
            kind: "refundability".into(),
            observed_at: None,
            metadata: json!({ "payment_captured": true }),
        }],
    };

    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["authorization"]["grant_id"], "grant-1");
    assert!(value["action"].get("mandate").is_none());
}

#[test]
fn action_record_separates_authorization_from_execution() {
    let record = FinancialActionRecord {
        id: "action-1".into(),
        workspace_id: "workspace-1".into(),
        environment_id: "production".into(),
        authorization_intent_id: Some("intent-1".into()),
        authorization_receipt_id: Some("receipt-1".into()),
        authorization_effect: AuthorizationEffect::RequireApproval,
        authorization_status: AuthorizationIntentStatus::PendingApproval,
        authorization: None,
        execution_status: FinancialExecutionStatus::NotStarted,
        status_reason: None,
        state: FinancialActionState::HeldForApproval,
        state_reason: Some("Human authorization required".into()),
        action: FinancialAction {
            id: Some("action-1".into()),
            kind: FinancialActionKind::Payment,
            operation: "pay".into(),
            principal_id: "agent-1".into(),
            amount: MoneyAmount {
                amount_minor: 100,
                currency: "USD".into(),
            },
            counterparty: None,
            rail: FinancialRail::Internal,
            memo: None,
            metadata: json!({}),
        },
        evidence: Vec::new(),
        created_at: "2026-07-14T00:00:00Z".into(),
        updated_at: "2026-07-14T00:00:00Z".into(),
    };

    let mut value = serde_json::to_value(record).unwrap();
    assert_eq!(value["authorization_effect"], "require_approval");
    assert_eq!(value["authorization_status"], "pending_approval");
    assert_eq!(value["execution_status"], "not_started");
    assert_eq!(value["state"], "held_for_approval");
    assert_eq!(value["state_reason"], "Human authorization required");

    value.as_object_mut().unwrap().remove("state");
    value.as_object_mut().unwrap().remove("state_reason");
    let legacy: FinancialActionRecord = serde_json::from_value(value).unwrap();
    assert_eq!(legacy.state, FinancialActionState::Evaluating);
    assert_eq!(legacy.state_reason, None);
}
