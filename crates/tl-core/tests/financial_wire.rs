use serde_json::json;
use tl_core::financial::{
    ApprovalRequirement, CounterpartyRef, CreateFinancialActionRequest, EvidenceRef,
    FinancialAction, FinancialActionKind, FinancialActionOutcome, FinancialActionOutcomeStatus,
    FinancialActionPrecondition, FinancialActionStatus, FinancialDecision, FinancialRail,
    FinancialReceipt, MandateRef, MoneyAmount, RecoveryStatus, ReversalCapability,
};
use tl_core::{FinancialActionKind as RootFinancialActionKind, Verdict};

#[test]
fn financial_types_are_available_from_named_module_and_root() {
    let _: Option<RootFinancialActionKind> = Some(FinancialActionKind::Refund);
    let _: Option<FinancialDecision> = None;
    let _: Option<FinancialReceipt> = None;
}

#[test]
fn financial_enums_use_snake_case_wire_values() {
    assert_eq!(
        serde_json::to_value(FinancialActionKind::InvoiceApproval).unwrap(),
        "invoice_approval"
    );
    assert_eq!(
        serde_json::to_value(FinancialActionStatus::Authorized).unwrap(),
        "authorized"
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
        serde_json::to_value(FinancialActionPrecondition::AmountLteRefundableBalance).unwrap(),
        "amount_lte_refundable_balance"
    );
    assert_eq!(
        serde_json::to_value(FinancialRail::PaymentHttp).unwrap(),
        "payment_http"
    );
}

#[test]
fn financial_action_request_serializes_canonical_money_and_optional_refs() {
    let request = CreateFinancialActionRequest {
        idempotency_key: "idem-1".into(),
        execute: true,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            principal_id: "agent-refund-bot".into(),
            amount: MoneyAmount {
                amount_minor: 7_500,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: "cust_456".into(),
                display_name: Some("Customer 456".into()),
                kind: "customer".into(),
                country: Some("US".into()),
                metadata: json!({ "segment": "support" }),
            }),
            rail: FinancialRail::PaymentHttp,
            mandate: Some(MandateRef {
                id: "mandate_123".into(),
                version: Some(2),
            }),
            memo: Some("damaged_item".into()),
            metadata: json!({ "order_id": "order_123" }),
        },
        evidence: vec![EvidenceRef {
            source: "customer_backend".into(),
            source_id: "refund_check_789".into(),
            kind: "refundability_snapshot".into(),
            observed_at: Some("2026-07-05T19:00:00Z".into()),
            metadata: json!({ "refundable_balance_minor": 10_000 }),
        }],
    };

    let json = serde_json::to_value(&request).expect("request serializes");
    assert_eq!(json["idempotency_key"], "idem-1");
    assert_eq!(json["execute"], true);
    assert_eq!(json["action"]["kind"], "refund");
    assert_eq!(json["action"]["amount"]["amount_minor"], 7500);
    assert_eq!(json["action"]["amount"]["currency"], "USD");
    assert_eq!(json["action"]["counterparty"]["id"], "cust_456");
    assert_eq!(json["action"]["mandate"]["version"], 2);
    assert_eq!(
        json["evidence"][0]["metadata"]["refundable_balance_minor"],
        10000
    );
}

#[test]
fn financial_decision_carries_verdict_status_approval_and_receipt_refs() {
    let decision = FinancialDecision {
        action_id: "fa_123".into(),
        status: FinancialActionStatus::Held,
        verdict: Verdict::Escalate,
        reason: "approval threshold exceeded".into(),
        approval: Some(ApprovalRequirement {
            required: true,
            approver_roles: vec!["finance_admin".into()],
            reason: "refund above auto-approval threshold".into(),
            expires_at: Some("2026-07-06T19:00:00Z".into()),
        }),
        receipt_id: None,
    };

    let json = serde_json::to_value(&decision).expect("decision serializes");
    assert_eq!(json["action_id"], "fa_123");
    assert_eq!(json["status"], "held");
    assert_eq!(json["verdict"], "escalate");
    assert_eq!(json["approval"]["required"], true);
    assert_eq!(json["receipt_id"], serde_json::Value::Null);
}

#[test]
fn financial_outcome_records_recovery_state_without_accounting_floats() {
    let outcome = FinancialActionOutcome {
        action_id: "fa_123".into(),
        status: FinancialActionOutcomeStatus::LossRecorded,
        reversal_capability: ReversalCapability::ManualRecovery,
        recovery_status: RecoveryStatus::Failed,
        provider_status: Some("refund_succeeded_disputed".into()),
        provider_reference: Some("re_123".into()),
        final_loss_amount: Some(MoneyAmount {
            amount_minor: 7_500,
            currency: "USD".into(),
        }),
        occurred_at: "2026-07-05T20:00:00Z".into(),
        metadata: json!({ "dispute_id": "dp_123" }),
    };

    let json = serde_json::to_value(&outcome).expect("outcome serializes");
    assert_eq!(json["status"], "loss_recorded");
    assert_eq!(json["reversal_capability"], "manual_recovery");
    assert_eq!(json["recovery_status"], "failed");
    assert_eq!(json["final_loss_amount"]["amount_minor"], 7500);
    assert!(json["final_loss_amount"].get("amount").is_none());
}
