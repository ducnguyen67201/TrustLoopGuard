use std::sync::Arc;

use serde_json::json;
use tl_core::{
    AuthorizationEffect, AuthorizationIntentStatus, CreateFinancialActionRequest,
    CreateFinancialPolicyRequest, FinancialAction, FinancialActionKind, FinancialExecutionStatus,
    FinancialPolicySelector, FinancialRail, MoneyAmount, SpendMeter,
};
use tl_server::financial::{FinancialAuthorizationService, MemoryFinancialStore};

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
}
