//! Service-level coverage for financial action orchestration.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use tl_core::{
    AgenticPaymentAuthorizeRequest, AgenticPaymentCommitRequest, AgenticPaymentDecision,
    AgenticPaymentMandateScope, AgenticPaymentReservation, AgenticPaymentReservationStatus,
    AgenticPaymentRollbackRequest, ApprovalRequirement, ApproveMatchingFinancialActionsRequest,
    CounterpartyRef, CreateFinancialActionRequest, CreateFinancialMandateRequest, EvidenceRef,
    FinancialAction, FinancialActionDecision, FinancialActionKind, FinancialActionOutcome,
    FinancialActionOutcomeStatus, FinancialActionPrecondition, FinancialActionStatus,
    FinancialApprovalRequestStatus, FinancialDecisionRiskCode, FinancialEligibilityStatus,
    FinancialExecutionProofStatus, FinancialMandate, FinancialMandateListResponse,
    FinancialMandateStatus, FinancialOutcomeListResponse, FinancialRail, FinancialReceipt,
    MandateRef, MoneyAmount, RecoveryStatus, ReversalCapability, X402PaymentRequirement,
    X402SettlementProof,
};
use tl_policy::{Action, FamilyPolicy, FinancialPolicy, FinancialWhen};
use tl_server::{
    AgenticPaymentBudgetReservationRequest, FinancialAuthorizationService,
    FinancialLedgerEntryKind, FinancialStore, FinancialStoreError, MemoryFinancialStore,
    MemoryPolicyStore, PolicyStore,
};

fn refund_request(idempotency_key: &str, amount_minor: i64) -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: idempotency_key.into(),
        execute: false,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            operation: "issue_refund".into(),
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

fn refund_request_with_mandate(
    idempotency_key: &str,
    amount_minor: i64,
    mandate_id: &str,
) -> CreateFinancialActionRequest {
    let mut request = refund_request(idempotency_key, amount_minor);
    request.action.mandate = Some(MandateRef {
        id: mandate_id.into(),
        version: Some(1),
    });
    request
}

fn executable_refund_request(
    idempotency_key: &str,
    amount_minor: i64,
) -> CreateFinancialActionRequest {
    let mut request = refund_request(idempotency_key, amount_minor);
    request.execute = true;
    request
}

fn payment_request(idempotency_key: &str, amount_minor: i64) -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: idempotency_key.into(),
        execute: false,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Payment,
            operation: "pay".into(),
            principal_id: "alice".into(),
            amount: MoneyAmount {
                amount_minor,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: "coffee".into(),
                display_name: Some("Coffee".into()),
                kind: "merchant".into(),
                country: None,
                metadata: serde_json::json!({}),
            }),
            rail: FinancialRail::PaymentHttp,
            mandate: None,
            memo: None,
            metadata: serde_json::json!({}),
        },
        evidence: vec![],
    }
}

fn service() -> FinancialAuthorizationService {
    FinancialAuthorizationService::new(Arc::new(MemoryFinancialStore::new()))
}

#[derive(Debug, Default)]
struct SpendAwareStore {
    inner: MemoryFinancialStore,
    spent_today_minor: i64,
    spent_week_minor: i64,
    spent_month_minor: i64,
}

#[async_trait]
impl FinancialStore for SpendAwareStore {
    async fn create_action(
        &self,
        workspace_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.inner.create_action(workspace_id, input).await
    }

    async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.inner.get_action(workspace_id, action_id).await
    }

    async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::FinancialActionListResponse, FinancialStoreError> {
        self.inner.list_actions(workspace_id).await
    }

    async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.inner.create_mandate(workspace_id, input).await
    }

    async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialMandateListResponse, FinancialStoreError> {
        self.inner.list_mandates(workspace_id).await
    }

    async fn get_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
        version: Option<i32>,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.inner
            .get_mandate(workspace_id, mandate_id, version)
            .await
    }

    async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.inner.revoke_mandate(workspace_id, mandate_id).await
    }

    async fn create_receipt(
        &self,
        workspace_id: &str,
        action_id: &str,
        trace_id: Option<&str>,
        ledger_event_ids: Vec<String>,
        proof: serde_json::Value,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.inner
            .create_receipt(workspace_id, action_id, trace_id, ledger_event_ids, proof)
            .await
    }

    async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.inner.get_receipt(workspace_id, receipt_id).await
    }

    async fn record_action_outcome(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialActionOutcome,
    ) -> Result<FinancialActionOutcome, FinancialStoreError> {
        self.inner
            .record_action_outcome(workspace_id, action_id, outcome)
            .await
    }

    async fn list_action_outcomes(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialOutcomeListResponse, FinancialStoreError> {
        self.inner
            .list_action_outcomes(workspace_id, action_id)
            .await
    }

    async fn create_approval_request(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<tl_core::FinancialApprovalRequest, FinancialStoreError> {
        self.inner
            .create_approval_request(workspace_id, action_id, approval)
            .await
    }

    async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::FinancialApprovalRequestListResponse, FinancialStoreError> {
        self.inner.list_approval_requests(workspace_id).await
    }

    async fn resolve_pending_approval_requests(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialApprovalRequestStatus,
        decided_by: Option<&str>,
    ) -> Result<(), FinancialStoreError> {
        self.inner
            .resolve_pending_approval_requests(workspace_id, action_id, status, decided_by)
            .await
    }

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.inner
            .transition_action(workspace_id, action_id, status, event_type)
            .await
    }

    async fn transition_action_with_reason(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
        reason: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.inner
            .transition_action_with_reason(workspace_id, action_id, status, event_type, reason)
            .await
    }

    async fn record_ledger_entry(
        &self,
        workspace_id: &str,
        action_id: &str,
        kind: FinancialLedgerEntryKind,
        amount_minor: i64,
        currency: &str,
        idempotency_key: &str,
        metadata: serde_json::Value,
    ) -> Result<String, FinancialStoreError> {
        self.inner
            .record_ledger_entry(
                workspace_id,
                action_id,
                kind,
                amount_minor,
                currency,
                idempotency_key,
                metadata,
            )
            .await
    }

    async fn ledger_entry_exists(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<bool, FinancialStoreError> {
        self.inner
            .ledger_entry_exists(workspace_id, idempotency_key)
            .await
    }

    async fn try_reserve_agentic_payment_budget(
        &self,
        request: AgenticPaymentBudgetReservationRequest,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        self.inner.try_reserve_agentic_payment_budget(request).await
    }

    async fn get_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        self.inner
            .get_agentic_payment_reservation(workspace_id, action_id)
            .await
    }

    async fn commit_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        proof: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        self.inner
            .commit_agentic_payment_reservation(workspace_id, action_id, proof)
            .await
    }

    async fn release_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        reason: &str,
        metadata: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        self.inner
            .release_agentic_payment_reservation(workspace_id, action_id, reason, metadata)
            .await
    }

    async fn net_spend_minor(
        &self,
        _workspace_id: &str,
        _principal_id: &str,
        _currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, FinancialStoreError> {
        let window_days = end.signed_duration_since(start).num_days();
        if window_days <= 1 {
            Ok(self.spent_today_minor)
        } else if window_days < 7 {
            Ok(self.spent_week_minor)
        } else {
            Ok(self.spent_month_minor)
        }
    }
}

fn financial_policy(daily_minor: Option<i64>, monthly_minor: Option<i64>) -> FamilyPolicy {
    FamilyPolicy::Financial(FinancialPolicy {
        id: "refund-ledger-caps".into(),
        description: None,
        severity: tl_core::Severity::High,
        when: FinancialWhen {
            agents: vec!["refund-bot".into()],
            action_kinds: vec![FinancialActionKind::Refund],
            operations: vec![],
            currencies: vec!["USD".into()],
            rails: vec![FinancialRail::Card],
        },
        meter: tl_core::SpendMeter::Actions,
        per_transaction_minor: None,
        hold_above_minor: None,
        daily_minor,
        weekly_minor: None,
        monthly_minor,
        allowed_counterparty_ids: vec![],
        denied_counterparty_ids: vec![],
        hold_new_counterparty: false,
        mandate_required: false,
        approval_threshold_minor: None,
        approver_roles: vec![],
        refund_original_method_only: false,
        required_preconditions: vec![],
        missing_evidence_action: Action::Escalate,
        failed_precondition_action: Action::Block,
        on_breach: Action::Block,
    })
}

fn payment_financial_policy(
    per_transaction_minor: Option<i64>,
    daily_minor: Option<i64>,
) -> FamilyPolicy {
    FamilyPolicy::Financial(FinancialPolicy {
        id: "pay-alice".into(),
        description: None,
        severity: tl_core::Severity::High,
        when: FinancialWhen {
            agents: vec!["alice".into()],
            action_kinds: vec![FinancialActionKind::Payment],
            operations: vec!["pay".into()],
            currencies: vec!["USD".into()],
            rails: vec![FinancialRail::PaymentHttp],
        },
        meter: tl_core::SpendMeter::Actions,
        per_transaction_minor,
        hold_above_minor: None,
        daily_minor,
        weekly_minor: None,
        monthly_minor: None,
        allowed_counterparty_ids: vec![],
        denied_counterparty_ids: vec![],
        hold_new_counterparty: false,
        mandate_required: false,
        approval_threshold_minor: None,
        approver_roles: vec![],
        refund_original_method_only: false,
        required_preconditions: vec![],
        missing_evidence_action: Action::Escalate,
        failed_precondition_action: Action::Block,
        on_breach: Action::Block,
    })
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
        payment_scope: None,
        metadata: serde_json::json!({ "source": "service_test" }),
        starts_at: None,
        expires_at: None,
    }
}

fn mandate_request_with_scope(
    agent_id: &str,
    mandate_id: &str,
    scope: serde_json::Value,
) -> CreateFinancialMandateRequest {
    CreateFinancialMandateRequest {
        id: Some(mandate_id.into()),
        scope,
        ..mandate_request(agent_id)
    }
}

fn outcome(action_id: &str, status: FinancialActionOutcomeStatus) -> FinancialActionOutcome {
    FinancialActionOutcome {
        action_id: action_id.into(),
        status,
        reversal_capability: ReversalCapability::ManualRecovery,
        recovery_status: RecoveryStatus::ManualRequired,
        provider_status: Some("provider_status".into()),
        provider_reference: Some("provider_ref_123".into()),
        final_loss_amount: None,
        occurred_at: "2026-07-05T20:00:00Z".into(),
        metadata: serde_json::json!({ "source": "service_test" }),
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
async fn service_normalizes_typed_payment_scope_and_authorizes_matching_x402_payment() {
    let service = service();
    let mandate = service
        .create_mandate(
            "ws_finance",
            CreateFinancialMandateRequest {
                id: Some("mandate_x402_article".into()),
                version: Some(1),
                principal_id: "spid:pay-agent".into(),
                scope: serde_json::Value::Null,
                payment_scope: Some(AgenticPaymentMandateScope {
                    intent_label: Some("Buy the requested paid article".into()),
                    action_kinds: vec![FinancialActionKind::Payment],
                    operation: Some("x402_resource_payment".into()),
                    max_amount_minor: Some(800),
                    currency: Some("USD".into()),
                    rail: Some(FinancialRail::X402),
                    allowed_counterparty_ids: vec!["0xcafe".into()],
                    allowed_hosts: vec!["api.vendor.test".into()],
                    allowed_resources: vec!["/article/typed".into()],
                    allowed_networks: vec!["base".into()],
                    allowed_assets: vec!["USDC".into()],
                    allowed_pay_to: vec!["0xcafe".into()],
                    required_preconditions: vec![],
                }),
                metadata: serde_json::json!({ "source": "service_test" }),
                starts_at: None,
                expires_at: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(mandate.scope["rail"], "x402");
    assert_eq!(mandate.scope["operation"], "x402_resource_payment");
    assert_eq!(mandate.scope["allowed_resources"][0], "/article/typed");

    let mut request = x402_authorize_request(
        "x402-typed-scope",
        "session-typed",
        700,
        1_000,
        "/article/typed",
    );
    request.mandate = Some(MandateRef {
        id: mandate.id.clone(),
        version: Some(mandate.version),
    });

    let authorized = service
        .authorize_agentic_payment_in_environment("ws_finance", "prod", None, request)
        .await
        .unwrap();

    assert!(authorized.signable);
    let receipt = service
        .get_decision_receipt("ws_finance", "prod", &authorized.record.id)
        .await
        .unwrap();
    assert_eq!(
        receipt.authorization_scope.result,
        FinancialEligibilityStatus::Passed
    );
    assert!(receipt.authorization_scope.mandate_hash.is_some());
    assert_eq!(
        receipt.authorization_scope.normalized_scope.unwrap()["allowed_pay_to"][0],
        "0xcafe"
    );
}

#[tokio::test]
async fn service_records_and_lists_action_outcomes() {
    let service = service();
    let action = service
        .create_action("ws_finance", refund_request("idem-outcome", 7_500))
        .await
        .unwrap();

    service
        .record_action_outcome(
            "ws_finance",
            &action.id,
            outcome(&action.id, FinancialActionOutcomeStatus::Succeeded),
        )
        .await
        .unwrap();
    service
        .record_action_outcome(
            "ws_finance",
            &action.id,
            outcome(&action.id, FinancialActionOutcomeStatus::RecoveryStarted),
        )
        .await
        .unwrap();

    let outcomes = service
        .list_action_outcomes("ws_finance", &action.id)
        .await
        .unwrap();

    assert_eq!(outcomes.outcomes.len(), 2);
    assert_eq!(
        outcomes.outcomes[0].status,
        FinancialActionOutcomeStatus::RecoveryStarted
    );
    assert_eq!(
        outcomes.outcomes[1].status,
        FinancialActionOutcomeStatus::Succeeded
    );
}

#[tokio::test]
async fn service_blocks_financial_action_when_ledger_window_exceeds_cap() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    policy_store
        .upsert_family(
            "ws_finance",
            "production",
            &financial_policy(Some(5_000), Some(10_000)),
            "family: financial",
        )
        .await
        .unwrap();
    let store = Arc::new(SpendAwareStore {
        spent_today_minor: 4_500,
        spent_week_minor: 4_500,
        spent_month_minor: 4_500,
        ..Default::default()
    });
    let service = FinancialAuthorizationService::with_policy_store(store, policy_store);

    let action = service
        .create_action_in_environment(
            "ws_finance",
            "production",
            refund_request("idem-ledger-cap", 750),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
    let approvals = service.list_approval_requests("ws_finance").await.unwrap();
    assert!(approvals.approval_requests.is_empty());
}

#[tokio::test]
async fn service_blocks_financial_action_when_weekly_ledger_window_exceeds_cap() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let mut policy = financial_policy(None, None);
    if let FamilyPolicy::Financial(financial) = &mut policy {
        financial.weekly_minor = Some(5_000);
    }
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let store = Arc::new(SpendAwareStore {
        spent_today_minor: 4_500,
        spent_week_minor: 4_500,
        spent_month_minor: 4_500,
        ..Default::default()
    });
    let service = FinancialAuthorizationService::with_policy_store(store, policy_store);

    let action = service
        .create_action_in_environment(
            "ws_finance",
            "production",
            refund_request("idem-weekly-ledger-cap", 750),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
    assert_eq!(
        action.status_reason.as_deref(),
        Some("financial policy `refund-ledger-caps`: weekly spend would exceed cap 5000")
    );
    let approvals = service.list_approval_requests("ws_finance").await.unwrap();
    assert!(approvals.approval_requests.is_empty());
}

#[tokio::test]
async fn service_rechecks_budget_before_approving_a_stale_proposal() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    policy_store
        .upsert_family(
            "ws_finance",
            "production",
            &financial_policy(Some(10_000), None),
            "family: financial",
        )
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );

    let stale = service
        .create_action_in_environment(
            "ws_finance",
            "production",
            refund_request("idem-stale-proposal", 2_500),
        )
        .await
        .unwrap();
    assert_eq!(stale.status, FinancialActionStatus::Proposed);

    let spent = service
        .create_action_in_environment(
            "ws_finance",
            "production",
            executable_refund_request("idem-fill-daily-budget", 10_000),
        )
        .await
        .unwrap();
    assert_eq!(spent.status, FinancialActionStatus::Executed);

    let stale = service
        .approve_action("ws_finance", &stale.id)
        .await
        .unwrap();

    assert_eq!(stale.status, FinancialActionStatus::Denied);
    assert_eq!(
        stale.status_reason.as_deref(),
        Some("financial policy `refund-ledger-caps`: daily spend would exceed cap 10000")
    );
}

#[tokio::test]
async fn service_serializes_concurrent_reservations_against_the_daily_cap() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    policy_store
        .upsert_family(
            "ws_finance",
            "production",
            &financial_policy(Some(2_500), None),
            "family: financial",
        )
        .await
        .unwrap();
    // This wrapper intentionally returns the same stale read to both callers.
    // The store-level reserve operation must remain authoritative.
    let store = Arc::new(SpendAwareStore::default());
    let service = FinancialAuthorizationService::with_policy_store(store.clone(), policy_store);

    let (first, second) = tokio::join!(
        service.create_action_in_environment(
            "ws_finance",
            "production",
            executable_refund_request("idem-concurrent-budget-a", 2_500),
        ),
        service.create_action_in_environment(
            "ws_finance",
            "production",
            executable_refund_request("idem-concurrent-budget-b", 2_500),
        )
    );
    let statuses = [first.unwrap().status, second.unwrap().status];

    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == FinancialActionStatus::Executed)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == FinancialActionStatus::Denied)
            .count(),
        1
    );
    assert_eq!(
        store
            .inner
            .net_spend_minor(
                "ws_finance",
                "refund-bot",
                "USD",
                Utc::now() - Duration::minutes(5),
                Utc::now() + Duration::minutes(5),
            )
            .await
            .unwrap(),
        2_500
    );
}

#[tokio::test]
async fn service_holds_when_required_financial_evidence_is_missing() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let mut policy = financial_policy(None, None);
    if let FamilyPolicy::Financial(financial) = &mut policy {
        financial.required_preconditions = vec![FinancialActionPrecondition::PaymentCaptured];
        financial.approver_roles = vec!["finance_admin".into()];
    }
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );

    let action = service
        .create_action_in_environment(
            "ws_finance",
            "production",
            refund_request("idem-missing-evidence", 750),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Held);
    let approvals = service.list_approval_requests("ws_finance").await.unwrap();
    assert_eq!(
        approvals.approval_requests[0].approver_roles,
        vec!["finance_admin".to_string()]
    );
    assert!(approvals.approval_requests[0]
        .reason
        .contains("missing eligibility evidence `payment_captured`"));
}

#[tokio::test]
async fn service_denies_when_required_financial_evidence_fails() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let mut policy = financial_policy(None, None);
    if let FamilyPolicy::Financial(financial) = &mut policy {
        financial.required_preconditions = vec![FinancialActionPrecondition::PaymentCaptured];
    }
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );
    let mut request = refund_request("idem-failed-evidence", 750);
    request.evidence = vec![EvidenceRef {
        source: "customer_backend".into(),
        source_id: "elig_failed".into(),
        kind: "refund_eligibility".into(),
        observed_at: None,
        metadata: serde_json::json!({ "payment_captured": false }),
    }];

    let action = service
        .create_action_in_environment("ws_finance", "production", request)
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
    assert_eq!(
        action.status_reason.as_deref(),
        Some("financial policy `refund-ledger-caps`: eligibility precondition `payment_captured` failed")
    );
}

#[tokio::test]
async fn service_allows_when_required_financial_evidence_passes() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let mut policy = financial_policy(None, None);
    if let FamilyPolicy::Financial(financial) = &mut policy {
        financial.required_preconditions = vec![FinancialActionPrecondition::PaymentCaptured];
    }
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );
    let mut request = refund_request("idem-passed-evidence", 750);
    request.evidence = vec![EvidenceRef {
        source: "customer_backend".into(),
        source_id: "elig_passed".into(),
        kind: "refund_eligibility".into(),
        observed_at: None,
        metadata: serde_json::json!({ "payment_captured": true }),
    }];

    let action = service
        .create_action_in_environment("ws_finance", "production", request)
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Proposed);
}

#[tokio::test]
async fn service_applies_financial_caps_to_typed_payment_actions() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let policy = payment_financial_policy(Some(10_000), None);
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );

    let action = service
        .create_action_in_environment(
            "ws_finance",
            "production",
            payment_request("idem-payment-over-cap", 80_000),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
    assert_eq!(
        action.status_reason.as_deref(),
        Some("financial policy `pay-alice`: amount 80000 over per-transaction cap 10000")
    );
}

#[tokio::test]
async fn service_applies_financial_payment_daily_caps_from_ledger() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let policy = payment_financial_policy(None, Some(10_000));
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let store = Arc::new(SpendAwareStore {
        spent_today_minor: 6_000,
        spent_week_minor: 6_000,
        spent_month_minor: 6_000,
        ..Default::default()
    });
    let service = FinancialAuthorizationService::with_policy_store(store, policy_store);

    let action = service
        .create_action_in_environment(
            "ws_finance",
            "production",
            payment_request("idem-payment-daily-cap", 5_000),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
    assert_eq!(
        action.status_reason.as_deref(),
        Some("financial policy `pay-alice`: daily spend would exceed cap 10000")
    );
}

#[tokio::test]
async fn service_allows_action_when_referenced_mandate_is_active_and_in_scope() {
    let service = service();
    service
        .create_mandate("ws_finance", mandate_request("refund-bot"))
        .await
        .unwrap();

    let action = service
        .create_action(
            "ws_finance",
            refund_request_with_mandate("idem-valid-mandate", 7_500, "mandate_refund_bot"),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Proposed);
}

#[tokio::test]
async fn service_denies_action_when_referenced_mandate_is_missing() {
    let service = service();

    let action = service
        .create_action(
            "ws_finance",
            refund_request_with_mandate("idem-missing-mandate", 7_500, "missing_mandate"),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
}

#[tokio::test]
async fn service_denies_action_when_referenced_mandate_is_revoked() {
    let service = service();
    service
        .create_mandate("ws_finance", mandate_request("refund-bot"))
        .await
        .unwrap();
    service
        .revoke_mandate("ws_finance", "mandate_refund_bot")
        .await
        .unwrap();

    let action = service
        .create_action(
            "ws_finance",
            refund_request_with_mandate("idem-revoked-mandate", 7_500, "mandate_refund_bot"),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
}

#[tokio::test]
async fn service_denies_action_when_mandate_scope_does_not_cover_action() {
    let service = service();
    service
        .create_mandate(
            "ws_finance",
            mandate_request_with_scope(
                "refund-bot",
                "mandate_refund_bot",
                serde_json::json!({
                    "action_kinds": ["payout"],
                    "max_amount_minor": 5_000,
                    "currency": "USD"
                }),
            ),
        )
        .await
        .unwrap();

    let action = service
        .create_action(
            "ws_finance",
            refund_request_with_mandate("idem-scope-mismatch", 7_500, "mandate_refund_bot"),
        )
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
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

    let receipt = service
        .get_receipt("ws_finance", &created.id)
        .await
        .unwrap();
    assert_eq!(receipt.id, created.id);
    assert_eq!(receipt.action_id, created.id);
    assert_eq!(receipt.proof["action_status"], "executed");

    let fetched = service.get_action("ws_finance", &created.id).await.unwrap();
    assert_eq!(fetched.status, FinancialActionStatus::Executed);
}

#[tokio::test]
async fn service_rejects_execute_before_authorization() {
    let service = service();
    let created = service
        .create_action("ws_finance", refund_request("idem-execute-proposed", 7_500))
        .await
        .unwrap();

    let err = service
        .execute_action("ws_finance", &created.id)
        .await
        .unwrap_err();

    assert!(
        matches!(err, FinancialStoreError::Validation(message) if message.contains("requires authorization"))
    );
}

#[tokio::test]
async fn service_execute_true_authorizes_executes_and_records_ledger_receipt() {
    let store = Arc::new(MemoryFinancialStore::new());
    let service = FinancialAuthorizationService::new(store.clone());

    let executed = service
        .create_action(
            "ws_finance",
            executable_refund_request("idem-execute-now", 7_500),
        )
        .await
        .unwrap();

    assert_eq!(executed.status, FinancialActionStatus::Executed);
    let receipt = service
        .get_receipt("ws_finance", &executed.id)
        .await
        .unwrap();
    assert_eq!(receipt.action_id, executed.id);
    assert_eq!(receipt.ledger_event_ids.len(), 1);
    assert_eq!(receipt.proof["ledger_source"], "financial_ledger_entries");
    let spend = store
        .net_spend_minor(
            "ws_finance",
            "refund-bot",
            "USD",
            Utc::now() - Duration::minutes(5),
            Utc::now() + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(spend, 7_500);
}

#[tokio::test]
async fn service_execute_after_hold_approval_releases_reservation_before_execution() {
    let store = Arc::new(MemoryFinancialStore::new());
    let service = FinancialAuthorizationService::new(store.clone());
    let action = service
        .create_action("ws_finance", refund_request("idem-held-execute", 7_500))
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
    service
        .approve_action("ws_finance", &action.id)
        .await
        .unwrap();

    let executed = service
        .execute_action("ws_finance", &action.id)
        .await
        .unwrap();

    assert_eq!(executed.status, FinancialActionStatus::Executed);
    let receipt = service.get_receipt("ws_finance", &action.id).await.unwrap();
    assert_eq!(receipt.ledger_event_ids.len(), 2);
    let spend = store
        .net_spend_minor(
            "ws_finance",
            "refund-bot",
            "USD",
            Utc::now() - Duration::minutes(5),
            Utc::now() + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(spend, 7_500);
}

#[tokio::test]
async fn service_receipt_proof_snapshots_policy_mandate_approval_and_evidence() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let mut policy = financial_policy(None, None);
    if let FamilyPolicy::Financial(financial) = &mut policy {
        financial.hold_above_minor = Some(5_000);
        financial.approver_roles = vec!["finance_admin".into()];
    }
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );
    service
        .create_mandate("ws_finance", mandate_request("refund-bot"))
        .await
        .unwrap();

    let mut request =
        refund_request_with_mandate("idem-receipt-proof", 7_500, "mandate_refund_bot");
    request.evidence = vec![EvidenceRef {
        source: "customer_backend".into(),
        source_id: "refund_eligibility_check_789".into(),
        kind: "refund_eligibility".into(),
        observed_at: Some("2026-07-06T10:00:00Z".into()),
        metadata: serde_json::json!({
            "order_exists": true,
            "payment_captured": true,
            "refundable_balance_minor": 10_000
        }),
    }];

    let held = service
        .create_action_in_environment("ws_finance", "production", request)
        .await
        .unwrap();
    assert_eq!(held.status, FinancialActionStatus::Held);
    let pending = service.list_approval_requests("ws_finance").await.unwrap();
    assert_eq!(
        pending.approval_requests[0].approver_roles,
        vec!["finance_admin".to_string()]
    );
    service
        .approve_action("ws_finance", &held.id)
        .await
        .unwrap();
    service
        .execute_action("ws_finance", &held.id)
        .await
        .unwrap();

    let receipt = service.get_receipt("ws_finance", &held.id).await.unwrap();
    assert_eq!(receipt.proof["schema"], "financial_execution_receipt.v1");
    assert_eq!(receipt.proof["action_snapshot"]["kind"], "refund");
    assert_eq!(
        receipt.proof["mandate_snapshot"]["id"],
        "mandate_refund_bot"
    );
    assert_eq!(
        receipt.proof["evidence_refs"][0]["source_id"],
        "refund_eligibility_check_789"
    );
    assert_eq!(receipt.proof["approval_requests"][0]["status"], "approved");
    assert_eq!(
        receipt.proof["approval_requests"][0]["approver_roles"][0],
        "finance_admin"
    );
    assert_eq!(
        receipt.proof["policy_snapshots"][0]["id"],
        "refund-ledger-caps"
    );
    assert_eq!(
        receipt.proof["ledger_event_ids"].as_array().unwrap().len(),
        receipt.ledger_event_ids.len()
    );
}

#[tokio::test]
async fn service_decision_receipt_explains_held_refund() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let mut policy = financial_policy(None, None);
    if let FamilyPolicy::Financial(financial) = &mut policy {
        financial.hold_above_minor = Some(5_000);
        financial.mandate_required = true;
        financial.approver_roles = vec!["finance_admin".into()];
        financial.required_preconditions = vec![
            FinancialActionPrecondition::OrderExists,
            FinancialActionPrecondition::PaymentCaptured,
            FinancialActionPrecondition::RefundWindowOpen,
            FinancialActionPrecondition::DestinationIsOriginalPaymentMethod,
        ];
    }
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: financial")
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );
    service
        .create_mandate("ws_finance", mandate_request("refund-bot"))
        .await
        .unwrap();
    let mut request =
        refund_request_with_mandate("idem-held-decision", 7_500, "mandate_refund_bot");
    request.evidence = vec![EvidenceRef {
        source: "customer_backend".into(),
        source_id: "eligibility_held".into(),
        kind: "refund_eligibility".into(),
        observed_at: None,
        metadata: serde_json::json!({
            "order_exists": true,
            "payment_captured": true,
            "refund_window_open": true,
            "destination_is_original_payment_method": true
        }),
    }];

    let action = service
        .create_action_in_environment("ws_finance", "production", request)
        .await
        .unwrap();
    let receipt = service
        .get_decision_receipt("ws_finance", "production", &action.id)
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Held);
    assert_eq!(receipt.decision, FinancialActionDecision::Hold);
    assert_eq!(
        receipt.reason,
        "valid refund, but above threshold so human approval required"
    );
    assert_eq!(
        receipt.authorization_scope.result,
        FinancialEligibilityStatus::Passed
    );
    assert!(receipt
        .evidence
        .iter()
        .all(|proof| proof.status == FinancialEligibilityStatus::Passed));
    assert!(receipt
        .risks
        .iter()
        .any(|risk| risk.code == FinancialDecisionRiskCode::AmountAboveAutoApproveThreshold));
    assert_eq!(
        receipt.approval.as_ref().unwrap().approver_roles,
        vec!["finance_admin".to_string()]
    );
    assert_eq!(
        receipt.execution.status,
        FinancialExecutionProofStatus::NotStarted
    );
}

#[tokio::test]
async fn service_decision_receipt_explains_denied_missing_authorization_scope() {
    let service = service();
    let action = service
        .create_action(
            "ws_finance",
            refund_request_with_mandate("idem-missing-scope-receipt", 7_500, "missing_scope"),
        )
        .await
        .unwrap();

    let receipt = service
        .get_decision_receipt("ws_finance", "production", &action.id)
        .await
        .unwrap();

    assert_eq!(action.status, FinancialActionStatus::Denied);
    assert_eq!(receipt.decision, FinancialActionDecision::Block);
    assert_eq!(
        receipt.authorization_scope.result,
        FinancialEligibilityStatus::Missing
    );
    assert!(receipt
        .risks
        .iter()
        .any(|risk| risk.code == FinancialDecisionRiskCode::MissingAuthorizationScope));
}

#[tokio::test]
async fn service_decision_receipt_explains_executed_action_receipt() {
    let service = service();
    let action = service
        .create_action(
            "ws_finance",
            refund_request("idem-executed-decision", 4_000),
        )
        .await
        .unwrap();
    service
        .authorize_action("ws_finance", &action.id)
        .await
        .unwrap();
    service
        .execute_action("ws_finance", &action.id)
        .await
        .unwrap();

    let receipt = service
        .get_decision_receipt("ws_finance", "production", &action.id)
        .await
        .unwrap();

    assert_eq!(receipt.decision, FinancialActionDecision::Allow);
    assert_eq!(receipt.status, FinancialActionStatus::Executed);
    assert_eq!(
        receipt.execution.status,
        FinancialExecutionProofStatus::Executed
    );
    assert_eq!(
        receipt.execution.receipt_id.as_deref(),
        Some(action.id.as_str())
    );
    assert!(!receipt.execution.ledger_event_ids.is_empty());
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
    let store = Arc::new(MemoryFinancialStore::new());
    let service = FinancialAuthorizationService::new(store.clone());
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
    let spend = store
        .net_spend_minor(
            "ws_finance",
            "refund-bot",
            "USD",
            Utc::now() - Duration::minutes(5),
            Utc::now() + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(spend, 7_500);
}

#[tokio::test]
async fn reusable_approval_binds_a_fingerprint_and_auto_attaches_the_latest_mandate() {
    let store = Arc::new(MemoryFinancialStore::new());
    let service = FinancialAuthorizationService::new(store);
    let action = service
        .create_action("ws_finance", refund_request("idem-reuse-source", 7_500))
        .await
        .unwrap();
    let held = service
        .hold_action(
            "ws_finance",
            &action.id,
            ApprovalRequirement {
                required: true,
                approver_roles: vec!["finance_admin".into()],
                reason: "review first matching refund".into(),
                expires_at: None,
            },
        )
        .await
        .unwrap();

    let envelope = service
        .get_approval_envelope("ws_finance", &held.id)
        .await
        .unwrap();
    assert_eq!(envelope.fingerprint_version, 1);
    assert!(envelope.action_fingerprint.starts_with("sha256:v1:"));
    assert_eq!(envelope.counterparty_id.as_deref(), Some("cust_456"));

    let reusable = service
        .approve_matching_actions_as(
            "ws_finance",
            &held.id,
            Some("finance-admin-1"),
            ApproveMatchingFinancialActionsRequest {
                action_fingerprint: envelope.action_fingerprint.clone(),
                max_amount_minor: 10_000,
                expires_at: (Utc::now() + Duration::hours(1)).to_rfc3339(),
            },
        )
        .await
        .unwrap();
    assert_eq!(reusable.action.status, FinancialActionStatus::Authorized);
    assert_eq!(reusable.mandate.scope["max_amount_minor"], 10_000);
    assert_eq!(reusable.mandate.metadata["mode"], "approval_reuse");
    assert_eq!(
        reusable.mandate.metadata["action_fingerprint"],
        envelope.action_fingerprint
    );

    let mut matching = executable_refund_request("idem-reuse-match", 5_000);
    matching.action.memo = Some("a different memo is not part of action identity".into());
    let executed = service.create_action("ws_finance", matching).await.unwrap();
    assert_eq!(executed.status, FinancialActionStatus::Executed);
    assert_eq!(
        executed
            .action
            .mandate
            .as_ref()
            .map(|reference| reference.id.as_str()),
        Some(reusable.mandate.id.as_str())
    );

    let over_cap = service
        .create_action("ws_finance", refund_request("idem-reuse-over-cap", 10_001))
        .await
        .unwrap();
    assert_eq!(over_cap.status, FinancialActionStatus::Proposed);
    assert!(over_cap.action.mandate.is_none());

    let mut other_counterparty = refund_request("idem-reuse-other-counterparty", 5_000);
    other_counterparty.action.counterparty.as_mut().unwrap().id = "cust_other".into();
    let other_counterparty = service
        .create_action("ws_finance", other_counterparty)
        .await
        .unwrap();
    assert!(other_counterparty.action.mandate.is_none());

    service
        .revoke_mandate("ws_finance", &reusable.mandate.id)
        .await
        .unwrap();
    let after_revoke = service
        .create_action("ws_finance", refund_request("idem-reuse-revoked", 5_000))
        .await
        .unwrap();
    assert!(after_revoke.action.mandate.is_none());
}

#[tokio::test]
async fn reusable_approval_does_not_bypass_the_live_ledger_budget() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    policy_store
        .upsert_family(
            "ws_finance",
            "production",
            &financial_policy(Some(10_000), None),
            "family: financial",
        )
        .await
        .unwrap();
    let service = FinancialAuthorizationService::with_policy_store(
        Arc::new(MemoryFinancialStore::new()),
        policy_store,
    );
    let source = service
        .create_action("ws_finance", refund_request("idem-reuse-budget-source", 2_500))
        .await
        .unwrap();
    let held = service
        .hold_action(
            "ws_finance",
            &source.id,
            ApprovalRequirement {
                required: true,
                approver_roles: vec!["finance_admin".into()],
                reason: "review first matching refund".into(),
                expires_at: None,
            },
        )
        .await
        .unwrap();
    let envelope = service
        .get_approval_envelope("ws_finance", &held.id)
        .await
        .unwrap();
    service
        .approve_matching_actions_as(
            "ws_finance",
            &held.id,
            Some("finance-admin-1"),
            ApproveMatchingFinancialActionsRequest {
                action_fingerprint: envelope.action_fingerprint,
                max_amount_minor: 10_000,
                expires_at: (Utc::now() + Duration::hours(1)).to_rfc3339(),
            },
        )
        .await
        .unwrap();

    let matching = service
        .create_action(
            "ws_finance",
            executable_refund_request("idem-reuse-budget-match", 8_000),
        )
        .await
        .unwrap();

    assert!(matching.action.mandate.is_some());
    assert_eq!(matching.status, FinancialActionStatus::Denied);
    assert_eq!(
        matching.status_reason.as_deref(),
        Some("financial policy `refund-ledger-caps`: daily spend would exceed cap 10000")
    );
}

#[tokio::test]
async fn reusable_approval_rejects_a_stale_fingerprint() {
    let service = service();
    let action = service
        .create_action("ws_finance", refund_request("idem-reuse-stale", 7_500))
        .await
        .unwrap();
    let held = service
        .hold_action(
            "ws_finance",
            &action.id,
            ApprovalRequirement {
                required: true,
                approver_roles: vec![],
                reason: "review".into(),
                expires_at: None,
            },
        )
        .await
        .unwrap();

    let error = service
        .approve_matching_actions_as(
            "ws_finance",
            &held.id,
            None,
            ApproveMatchingFinancialActionsRequest {
                action_fingerprint: "sha256:v1:stale".into(),
                max_amount_minor: 10_000,
                expires_at: (Utc::now() + Duration::hours(1)).to_rfc3339(),
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(error, FinancialStoreError::Conflict));
    assert!(service
        .list_mandates("ws_finance")
        .await
        .unwrap()
        .mandates
        .is_empty());
}

#[tokio::test]
async fn reusable_approval_caps_the_standing_authority_window() {
    let service = service();
    let action = service
        .create_action("ws_finance", refund_request("idem-reuse-expiry", 7_500))
        .await
        .unwrap();
    let held = service
        .hold_action(
            "ws_finance",
            &action.id,
            ApprovalRequirement {
                required: true,
                approver_roles: vec![],
                reason: "review".into(),
                expires_at: None,
            },
        )
        .await
        .unwrap();
    let envelope = service
        .get_approval_envelope("ws_finance", &held.id)
        .await
        .unwrap();

    let error = service
        .approve_matching_actions_as(
            "ws_finance",
            &held.id,
            None,
            ApproveMatchingFinancialActionsRequest {
                action_fingerprint: envelope.action_fingerprint,
                max_amount_minor: 10_000,
                expires_at: (Utc::now() + Duration::days(31)).to_rfc3339(),
            },
        )
        .await
        .unwrap_err();

    assert!(
        matches!(error, FinancialStoreError::Validation(message) if message.contains("30 days"))
    );
    assert_eq!(
        service
            .get_action("ws_finance", &held.id)
            .await
            .unwrap()
            .status,
        FinancialActionStatus::Held
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
async fn service_approve_with_actor_records_decided_by() {
    let service = service();
    let action = service
        .create_action("ws_finance", refund_request("idem-approve-actor", 7_500))
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

    service
        .approve_action_as("ws_finance", &action.id, Some("approver_123"))
        .await
        .unwrap();

    let approvals = service.list_approval_requests("ws_finance").await.unwrap();
    assert_eq!(
        approvals.approval_requests[0].decided_by.as_deref(),
        Some("approver_123")
    );
}

#[tokio::test]
async fn service_deny_resolves_pending_approval_request() {
    let store = Arc::new(MemoryFinancialStore::new());
    let service = FinancialAuthorizationService::new(store.clone());
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
    let spend = store
        .net_spend_minor(
            "ws_finance",
            "refund-bot",
            "USD",
            Utc::now() - Duration::minutes(5),
            Utc::now() + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(spend, 0);
}

#[tokio::test]
async fn service_validates_action_before_storage_transition() {
    let error = service()
        .create_action("ws_finance", refund_request("idem-invalid", 0))
        .await
        .unwrap_err();

    assert!(matches!(error, FinancialStoreError::Validation(_)));
}

fn x402_authorize_request(
    idempotency_key: &str,
    session_id: &str,
    amount_minor: i64,
    session_limit_minor: i64,
    resource: &str,
) -> AgenticPaymentAuthorizeRequest {
    AgenticPaymentAuthorizeRequest {
        idempotency_key: idempotency_key.into(),
        principal_id: "spid:pay-agent".into(),
        session_id: session_id.into(),
        operation: Some("x402_resource_payment".into()),
        session_limit_minor: Some(session_limit_minor),
        reservation_expires_at: None,
        mandate: None,
        payment_requirement: X402PaymentRequirement {
            amount: MoneyAmount {
                amount_minor,
                currency: "usd".into(),
            },
            pay_to: "0xCAFE".into(),
            network: Some("base".into()),
            asset: Some("usdc".into()),
            scheme: Some("exact".into()),
            resource: Some(resource.into()),
            method: Some("get".into()),
            host: Some("api.vendor.test".into()),
            facilitator: Some("facilitator.test".into()),
            raw: serde_json::json!({ "source": "x402" }),
        },
        evidence: vec![],
        metadata: serde_json::json!({ "tenant_ref": "tenant_123" }),
        traceparent: None,
        tracestate: None,
    }
}

fn x402_commit_request(
    payment_requirement_hash: String,
    amount_minor: i64,
) -> AgenticPaymentCommitRequest {
    AgenticPaymentCommitRequest {
        proof: X402SettlementProof {
            settlement_reference: Some("settlement_123".into()),
            provider: Some("coinbase".into()),
            payment_requirement_hash: Some(payment_requirement_hash),
            amount: Some(MoneyAmount {
                amount_minor,
                currency: "USD".into(),
            }),
            network: Some("base".into()),
            asset: Some("USDC".into()),
            pay_to: Some("0xcafe".into()),
            payment_response: serde_json::json!({ "transaction": "0xsettled" }),
            raw: serde_json::json!({ "facilitator": "ok" }),
        },
        idempotency_key: Some("commit_123".into()),
    }
}

#[tokio::test]
async fn service_authorizes_and_commits_x402_agentic_payment() {
    let store = Arc::new(MemoryFinancialStore::new());
    let service = FinancialAuthorizationService::new(store.clone());
    let authorize_request =
        x402_authorize_request("x402-idem-1", "session-1", 700, 1_000, "/article/1");

    let authorized = service
        .authorize_agentic_payment_in_environment(
            "ws_finance",
            "prod",
            None,
            authorize_request.clone(),
        )
        .await
        .unwrap();

    assert_eq!(authorized.decision, AgenticPaymentDecision::Authorized);
    assert!(authorized.signable);
    assert_eq!(
        authorized.record.action.status,
        FinancialActionStatus::Authorized
    );
    assert_eq!(authorized.record.action.action.rail, FinancialRail::X402);
    assert_eq!(
        authorized.record.reservation.as_ref().unwrap().status,
        AgenticPaymentReservationStatus::Reserved
    );

    let retried = service
        .authorize_agentic_payment_in_environment("ws_finance", "prod", None, authorize_request)
        .await
        .unwrap();
    assert!(retried.signable);
    assert_eq!(retried.record.id, authorized.record.id);
    assert_eq!(
        retried.record.reservation.as_ref().unwrap().status,
        AgenticPaymentReservationStatus::Reserved
    );

    let committed = service
        .commit_agentic_payment(
            "ws_finance",
            &authorized.record.id,
            None,
            x402_commit_request(
                authorized
                    .record
                    .normalized_requirement
                    .payment_requirement_hash
                    .clone(),
                700,
            ),
        )
        .await
        .unwrap();

    assert_eq!(committed.decision, AgenticPaymentDecision::Committed);
    assert_eq!(committed.action.status, FinancialActionStatus::Executed);
    assert_eq!(
        committed.reservation.as_ref().unwrap().status,
        AgenticPaymentReservationStatus::Committed
    );
    assert_eq!(committed.receipt_id.as_deref(), Some(committed.id.as_str()));

    let spend = store
        .net_spend_minor(
            "ws_finance",
            "spid:pay-agent",
            "USD",
            Utc::now() - Duration::minutes(5),
            Utc::now() + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(spend, 700);
}

#[tokio::test]
async fn service_rejects_x402_payment_when_session_budget_is_reserved() {
    let service = service();

    service
        .authorize_agentic_payment_in_environment(
            "ws_finance",
            "prod",
            None,
            x402_authorize_request("x402-idem-2", "session-2", 700, 1_000, "/article/2"),
        )
        .await
        .unwrap();

    let error = service
        .authorize_agentic_payment_in_environment(
            "ws_finance",
            "prod",
            None,
            x402_authorize_request("x402-idem-3", "session-2", 400, 1_000, "/article/3"),
        )
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        FinancialStoreError::Validation(_) | FinancialStoreError::Conflict
    ));
}

#[tokio::test]
async fn service_rolls_back_unsettled_x402_reservation_without_spend() {
    let store = Arc::new(MemoryFinancialStore::new());
    let service = FinancialAuthorizationService::new(store.clone());
    let authorized = service
        .authorize_agentic_payment_in_environment(
            "ws_finance",
            "prod",
            None,
            x402_authorize_request("x402-idem-4", "session-4", 500, 500, "/article/4"),
        )
        .await
        .unwrap();

    let rolled_back = service
        .rollback_agentic_payment(
            "ws_finance",
            &authorized.record.id,
            None,
            AgenticPaymentRollbackRequest {
                reason: "payment signing failed".into(),
                provider_error: Some("wallet_rejected".into()),
                idempotency_key: Some("rollback_123".into()),
                metadata: serde_json::json!({}),
            },
        )
        .await
        .unwrap();

    assert_eq!(rolled_back.decision, AgenticPaymentDecision::RolledBack);
    assert_eq!(rolled_back.action.status, FinancialActionStatus::Failed);
    assert_eq!(
        rolled_back.reservation.as_ref().unwrap().status,
        AgenticPaymentReservationStatus::Released
    );
    let spend = store
        .net_spend_minor(
            "ws_finance",
            "spid:pay-agent",
            "USD",
            Utc::now() - Duration::minutes(5),
            Utc::now() + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(spend, 0);
}
