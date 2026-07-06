//! Service-level coverage for financial action orchestration.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use tl_core::{
    ApprovalRequirement, CounterpartyRef, CreateFinancialActionRequest,
    CreateFinancialMandateRequest, EvidenceRef, FinancialAction, FinancialActionKind,
    FinancialActionOutcome, FinancialActionOutcomeStatus, FinancialActionStatus,
    FinancialApprovalRequestStatus, FinancialMandate, FinancialMandateListResponse,
    FinancialMandateStatus, FinancialOutcomeListResponse, FinancialRail, FinancialReceipt,
    MandateRef, MoneyAmount, RecoveryStatus, ReversalCapability,
};
use tl_policy::{Action, FamilyPolicy, FinancialPolicy, FinancialWhen, PaymentPolicy, PaymentWhen};
use tl_server::{
    FinancialAuthorizationService, FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
    MemoryFinancialStore, MemoryPolicyStore, PolicyStore,
};

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
            metadata: serde_json::json!({ "operation": "pay" }),
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

    async fn net_spend_minor(
        &self,
        _workspace_id: &str,
        _principal_id: &str,
        _currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, FinancialStoreError> {
        if end.signed_duration_since(start).num_days() <= 1 {
            Ok(self.spent_today_minor)
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
        per_transaction_minor: None,
        hold_above_minor: None,
        daily_minor,
        monthly_minor,
        allowed_counterparty_ids: vec![],
        denied_counterparty_ids: vec![],
        hold_new_counterparty: false,
        mandate_required: false,
        approval_threshold_minor: None,
        refund_original_method_only: false,
        required_preconditions: vec![],
        missing_evidence_action: Action::Escalate,
        failed_precondition_action: Action::Block,
        on_breach: Action::Block,
    })
}

fn legacy_payment_policy(
    per_transaction_minor: Option<i64>,
    daily_minor: Option<i64>,
) -> FamilyPolicy {
    FamilyPolicy::Payment(PaymentPolicy {
        id: "pay-alice".into(),
        description: None,
        severity: tl_core::Severity::High,
        when: PaymentWhen {
            agents: vec!["alice".into()],
            operations: vec!["pay".into()],
        },
        per_transaction_minor,
        hold_above_minor: None,
        daily_minor,
        monthly_minor: None,
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
async fn service_applies_legacy_payment_caps_to_typed_payment_actions() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let policy = legacy_payment_policy(Some(10_000), None);
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: payment")
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
}

#[tokio::test]
async fn service_applies_legacy_payment_daily_caps_from_financial_ledger() {
    let policy_store = Arc::new(MemoryPolicyStore::new());
    let policy = legacy_payment_policy(None, Some(10_000));
    policy_store
        .upsert_family("ws_finance", "production", &policy, "family: payment")
        .await
        .unwrap();
    let store = Arc::new(SpendAwareStore {
        spent_today_minor: 6_000,
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
        receipt.proof["policy_snapshots"][0]["id"],
        "refund-ledger-caps"
    );
    assert_eq!(
        receipt.proof["ledger_event_ids"].as_array().unwrap().len(),
        receipt.ledger_event_ids.len()
    );
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
