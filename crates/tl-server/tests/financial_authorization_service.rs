//! Service-level coverage for financial action orchestration.

use std::sync::Arc;

use tl_core::{
    ApprovalRequirement, CounterpartyRef, CreateFinancialActionRequest,
    CreateFinancialMandateRequest, FinancialAction, FinancialActionKind, FinancialActionStatus,
    FinancialApprovalRequestStatus, FinancialMandateStatus, FinancialRail, MoneyAmount,
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

fn mandate_request(agent_id: &str) -> CreateFinancialMandateRequest {
    CreateFinancialMandateRequest {
        id: Some("mandate_refund_bot".into()),
        version: Some(1),
        principal_id: agent_id.into(),
        scope: serde_json::json!({
            "action_kinds": ["refund"],
            "max_amount_minor": 10_000,
            "currency": "USD"
        }),
        metadata: serde_json::json!({ "source": "service_test" }),
        starts_at: None,
        expires_at: Some("2026-08-05T19:00:00Z".into()),
    }
}

#[tokio::test]
async fn service_creates_lists_and_revokes_mandates() {
    let service = service();
    let created = service
        .create_mandate("ws_finance", mandate_request("refund-bot"))
        .await
        .unwrap();
    assert_eq!(created.status, FinancialMandateStatus::Active);
    assert_eq!(created.principal_id, "refund-bot");

    let listed = service.list_mandates("ws_finance").await.unwrap();
    assert_eq!(listed.mandates.len(), 1);
    assert_eq!(listed.mandates[0].id, created.id);

    let revoked = service
        .revoke_mandate("ws_finance", &created.id)
        .await
        .unwrap();
    assert_eq!(revoked.status, FinancialMandateStatus::Revoked);
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
async fn service_hold_creates_pending_approval_request() {
    let service = service();
    let action = service
        .create_action("ws_finance", refund_request("idem-hold", 7_500))
        .await
        .unwrap();

    let held = service
        .hold_action(
            "ws_finance",
            &action.id,
            ApprovalRequirement {
                required: true,
                approver_roles: vec!["finance_admin".into()],
                reason: "refund above auto-approval threshold".into(),
                expires_at: Some("2026-07-06T19:00:00Z".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(held.status, FinancialActionStatus::Held);

    let approvals = service.list_approval_requests("ws_finance").await.unwrap();
    assert_eq!(approvals.approval_requests.len(), 1);
    assert_eq!(approvals.approval_requests[0].action_id, action.id);
    assert_eq!(
        approvals.approval_requests[0].status,
        FinancialApprovalRequestStatus::Pending
    );
    assert_eq!(
        approvals.approval_requests[0].approver_roles,
        vec!["finance_admin".to_string()]
    );
}

#[tokio::test]
async fn service_approve_resolves_pending_approval_request() {
    let service = service();
    let action = service
        .create_action("ws_finance", refund_request("idem-approve-held", 7_500))
        .await
        .unwrap();
    service
        .hold_action(
            "ws_finance",
            &action.id,
            ApprovalRequirement {
                required: true,
                approver_roles: vec!["finance_admin".into()],
                reason: "refund above auto-approval threshold".into(),
                expires_at: None,
            },
        )
        .await
        .unwrap();

    let approved = service
        .approve_action("ws_finance", &action.id)
        .await
        .unwrap();
    assert_eq!(approved.status, FinancialActionStatus::Authorized);

    let approvals = service.list_approval_requests("ws_finance").await.unwrap();
    assert_eq!(approvals.approval_requests.len(), 1);
    assert_eq!(
        approvals.approval_requests[0].status,
        FinancialApprovalRequestStatus::Approved
    );
    assert!(approvals.approval_requests[0].decided_at.is_some());
}

#[tokio::test]
async fn service_deny_resolves_pending_approval_request() {
    let service = service();
    let action = service
        .create_action("ws_finance", refund_request("idem-deny-held", 7_500))
        .await
        .unwrap();
    service
        .hold_action(
            "ws_finance",
            &action.id,
            ApprovalRequirement {
                required: true,
                approver_roles: vec!["finance_admin".into()],
                reason: "refund above auto-approval threshold".into(),
                expires_at: None,
            },
        )
        .await
        .unwrap();

    let denied = service.deny_action("ws_finance", &action.id).await.unwrap();
    assert_eq!(denied.status, FinancialActionStatus::Denied);

    let approvals = service.list_approval_requests("ws_finance").await.unwrap();
    assert_eq!(approvals.approval_requests.len(), 1);
    assert_eq!(
        approvals.approval_requests[0].status,
        FinancialApprovalRequestStatus::Denied
    );
    assert!(approvals.approval_requests[0].decided_at.is_some());
}

#[tokio::test]
async fn service_validates_action_before_storage_transition() {
    let error = service()
        .create_action("ws_finance", refund_request("idem-invalid", 0))
        .await
        .unwrap_err();

    assert!(matches!(error, FinancialStoreError::Validation(_)));
}
