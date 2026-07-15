use std::sync::Arc;

use serde_json::json;
use tl_core::{
    AuthorizationEffect, AuthorizationIntentStatus, CreateFinancialActionRequest,
    CreateFinancialPolicyRequest, EvidenceRef, FinancialAction, FinancialActionKind,
    FinancialActionPrecondition, FinancialActionState, FinancialExecutionStatus,
    FinancialPolicySelector, FinancialRail, MoneyAmount, SpendMeter,
};
use tl_server::financial::{FinancialAuthorizationService, FinancialStore, MemoryFinancialStore};

fn action(idempotency_key: &str) -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: idempotency_key.into(),
        execute: false,
        authorization: None,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Payment,
            operation: "pay_vendor".into(),
            principal_id: "buyer-agent".into(),
            amount: MoneyAmount {
                amount_minor: 2_500,
                currency: "USD".into(),
            },
            counterparty: None,
            rail: FinancialRail::Internal,
            memo: None,
            metadata: json!({}),
        },
        evidence: Vec::new(),
    }
}

fn ineligible_refund(idempotency_key: &str) -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: idempotency_key.into(),
        execute: true,
        authorization: None,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            operation: "issue_refund".into(),
            principal_id: "refund-bot".into(),
            amount: MoneyAmount {
                amount_minor: 12_500,
                currency: "USD".into(),
            },
            counterparty: None,
            rail: FinancialRail::PaymentHttp,
            memo: None,
            metadata: json!({ "reason": "item_arrived_damaged" }),
        },
        evidence: vec![EvidenceRef {
            source: "customer_backend".into(),
            source_id: "refund_eligibility_order-1".into(),
            kind: "refund_eligibility".into(),
            observed_at: None,
            metadata: json!({
                "amount_lte_refundable_balance": false,
                "refundable_balance_minor": 10_000,
            }),
        }],
    }
}

#[tokio::test]
async fn no_policy_projects_common_permit_without_starting_execution() {
    let service = FinancialAuthorizationService::new(Arc::new(MemoryFinancialStore::new()));
    let record = service
        .create_action_in_environment("workspace-1", "production", action("idem-1"))
        .await
        .unwrap();

    assert_eq!(record.authorization_effect, AuthorizationEffect::Permit);
    assert_eq!(
        record.authorization_status,
        AuthorizationIntentStatus::Authorized
    );
    assert_eq!(
        record.execution_status,
        FinancialExecutionStatus::NotStarted
    );
    assert_eq!(record.state, FinancialActionState::Authorized);
    assert_eq!(record.state_reason, None);
    assert!(record.authorization_intent_id.is_some());
    assert!(record.authorization_receipt_id.is_some());
}

#[tokio::test]
async fn financial_policy_creates_a_common_approval_intent() {
    let service = FinancialAuthorizationService::new(Arc::new(MemoryFinancialStore::new()));
    service
        .create_financial_policy(
            "workspace-1",
            "production",
            CreateFinancialPolicyRequest {
                id: "vendor-authority".into(),
                description: None,
                severity: None,
                when: FinancialPolicySelector {
                    operations: vec!["pay_vendor".into()],
                    ..FinancialPolicySelector::default()
                },
                meter: SpendMeter::Actions,
                per_transaction_minor: None,
                daily_minor: None,
                weekly_minor: None,
                monthly_minor: None,
                allowed_counterparty_ids: Vec::new(),
                denied_counterparty_ids: Vec::new(),
                require_approval_for_new_counterparty: false,
                grant_required: true,
                approval_threshold_minor: None,
                approver_roles: vec!["finance_owner".into()],
                refund_original_method_only: false,
                required_preconditions: Vec::new(),
                missing_evidence_effect: None,
                failed_precondition_effect: None,
                on_breach: None,
            },
        )
        .await
        .unwrap();

    let record = service
        .create_action_in_environment("workspace-1", "production", action("idem-2"))
        .await
        .unwrap();

    assert_eq!(
        record.authorization_effect,
        AuthorizationEffect::RequireApproval
    );
    assert_eq!(
        record.authorization_status,
        AuthorizationIntentStatus::PendingApproval
    );
    assert_eq!(
        record.execution_status,
        FinancialExecutionStatus::NotStarted
    );
    assert_eq!(record.state, FinancialActionState::HeldForApproval);
    assert_eq!(
        record.state_reason.as_deref(),
        Some("policy `vendor-authority` requires delegated authority")
    );
}

#[tokio::test]
async fn failed_evidence_without_authorization_is_not_executable() {
    let store = MemoryFinancialStore::new();
    let record = store
        .create_action(
            "workspace-1",
            "production",
            ineligible_refund("idem-ineligible-orphan"),
        )
        .await
        .unwrap();

    assert_eq!(record.authorization_intent_id, None);
    assert_eq!(
        record.execution_status,
        FinancialExecutionStatus::NotStarted
    );
    assert_eq!(record.state, FinancialActionState::NotExecutable);
    assert_eq!(
        record.state_reason.as_deref(),
        Some("Amount exceeds refundable balance")
    );
}

#[tokio::test]
async fn failed_refund_precondition_is_blocked_without_execution() {
    let service = FinancialAuthorizationService::new(Arc::new(MemoryFinancialStore::new()));
    service
        .create_financial_policy(
            "workspace-1",
            "production",
            CreateFinancialPolicyRequest {
                id: "refund-eligibility".into(),
                description: None,
                severity: None,
                when: FinancialPolicySelector {
                    action_kinds: vec![FinancialActionKind::Refund],
                    operations: vec!["issue_refund".into()],
                    ..FinancialPolicySelector::default()
                },
                meter: SpendMeter::Actions,
                per_transaction_minor: None,
                daily_minor: None,
                weekly_minor: None,
                monthly_minor: None,
                allowed_counterparty_ids: Vec::new(),
                denied_counterparty_ids: Vec::new(),
                require_approval_for_new_counterparty: false,
                grant_required: false,
                approval_threshold_minor: None,
                approver_roles: Vec::new(),
                refund_original_method_only: false,
                required_preconditions: vec![
                    FinancialActionPrecondition::AmountLteRefundableBalance,
                ],
                missing_evidence_effect: None,
                failed_precondition_effect: Some(AuthorizationEffect::Deny),
                on_breach: None,
            },
        )
        .await
        .unwrap();

    let record = service
        .create_action_in_environment(
            "workspace-1",
            "production",
            ineligible_refund("idem-ineligible-denied"),
        )
        .await
        .unwrap();

    assert_eq!(record.authorization_effect, AuthorizationEffect::Deny);
    assert_eq!(
        record.authorization_status,
        AuthorizationIntentStatus::Denied
    );
    assert_eq!(
        record.execution_status,
        FinancialExecutionStatus::NotStarted
    );
    assert_eq!(record.state, FinancialActionState::Blocked);
    assert_eq!(
        record.state_reason.as_deref(),
        Some("Amount exceeds refundable balance")
    );
}

#[tokio::test]
async fn execution_transitions_refresh_the_product_state() {
    let store = MemoryFinancialStore::new();
    let created = store
        .create_action("workspace-1", "production", action("idem-executed"))
        .await
        .unwrap();
    assert_eq!(created.state, FinancialActionState::Evaluating);

    let authorized = store
        .update_authorization(
            "workspace-1",
            "production",
            &created.id,
            Some("intent-1"),
            Some("receipt-1"),
            AuthorizationEffect::Permit,
            AuthorizationIntentStatus::Authorized,
        )
        .await
        .unwrap();
    assert_eq!(authorized.state, FinancialActionState::Authorized);

    let executing = store
        .transition_execution(
            "workspace-1",
            "production",
            &created.id,
            FinancialExecutionStatus::Executing,
            None,
        )
        .await
        .unwrap();
    assert_eq!(executing.state, FinancialActionState::Executing);

    let executed = store
        .transition_execution(
            "workspace-1",
            "production",
            &created.id,
            FinancialExecutionStatus::Succeeded,
            None,
        )
        .await
        .unwrap();
    assert_eq!(executed.state, FinancialActionState::Executed);
}
