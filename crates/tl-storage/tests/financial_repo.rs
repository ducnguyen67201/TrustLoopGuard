//! Financial authorization repository integration tests.
//!
//!   cargo test -p tl-storage --features postgres-it --test financial_repo

#![cfg(feature = "postgres-it")]

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    CounterpartyRef, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialActionKind, FinancialActionOutcome, FinancialActionOutcomeStatus,
    FinancialActionStatus, FinancialApprovalRequestStatus, FinancialMandateStatus, FinancialRail,
    MoneyAmount, RecoveryStatus, ReversalCapability,
};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, workspace_environments, workspaces},
    DbPool, FinancialLedgerEntryKind, FinancialRepo, StorageError,
};

async fn fresh_pool() -> (DbPool, testcontainers::ContainerAsync<PostgresImage>) {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.expect("migrate");
    let pool = connect_postgres(&url, 8).await.expect("connect");
    seed_workspace(&pool, "org_finance", "ws_finance").await;
    seed_workspace(&pool, "org_other", "ws_other").await;
    (pool, container)
}

async fn seed_workspace(pool: &DbPool, org_id: &str, workspace_id: &str) {
    let mut conn = pool.get().await.expect("connection");
    diesel::insert_into(organizations::table)
        .values((
            organizations::id.eq(org_id),
            organizations::name.eq(format!("{org_id} Org")),
            organizations::slug.eq(org_id),
        ))
        .execute(&mut conn)
        .await
        .expect("insert organization");
    diesel::insert_into(workspaces::table)
        .values((
            workspaces::id.eq(workspace_id),
            workspaces::organization_id.eq(org_id),
            workspaces::name.eq(format!("{workspace_id} Workspace")),
            workspaces::slug.eq(workspace_id),
        ))
        .execute(&mut conn)
        .await
        .expect("insert workspace");
    diesel::insert_into(workspace_environments::table)
        .values((
            workspace_environments::workspace_id.eq(workspace_id),
            workspace_environments::id.eq("production"),
            workspace_environments::slug.eq("production"),
            workspace_environments::name.eq("Production"),
            workspace_environments::is_default.eq(true),
        ))
        .execute(&mut conn)
        .await
        .expect("insert environment");
}

fn refund_request(agent_id: &str, cents: i64) -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: format!("idem-{agent_id}-{cents}"),
        execute: false,
        action: tl_core::FinancialAction {
            id: None,
            kind: FinancialActionKind::Refund,
            principal_id: agent_id.into(),
            amount: MoneyAmount {
                amount_minor: cents,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: "cust_456".into(),
                display_name: Some("Casey Customer".into()),
                kind: "customer".into(),
                country: Some("US".into()),
                metadata: serde_json::json!({}),
            }),
            mandate: None,
            rail: FinancialRail::Card,
            memo: Some("refund damaged item".into()),
            metadata: serde_json::json!({ "order_id": "order_123" }),
        },
        evidence: vec![],
    }
}

fn mandate_request(agent_id: &str, mandate_id: &str) -> CreateFinancialMandateRequest {
    CreateFinancialMandateRequest {
        id: Some(mandate_id.into()),
        version: Some(1),
        principal_id: agent_id.into(),
        scope: serde_json::json!({
            "action_kinds": ["refund"],
            "max_amount_minor": 10_000,
            "currency": "USD"
        }),
        metadata: serde_json::json!({ "source": "test" }),
        starts_at: None,
        expires_at: Some("2026-08-05T19:00:00Z".into()),
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
        metadata: serde_json::json!({ "source": "test" }),
    }
}

#[tokio::test]
async fn mandates_create_list_and_revoke_are_tenant_scoped() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);

    let created = repo
        .create_mandate(
            "ws_finance",
            mandate_request("refund-bot", "mandate_refund_bot"),
        )
        .await
        .expect("create mandate");
    repo.create_mandate(
        "ws_other",
        mandate_request("refund-bot", "mandate_refund_bot"),
    )
    .await
    .expect("other create");

    assert_eq!(created.status, FinancialMandateStatus::Active);
    assert_eq!(created.principal_id, "refund-bot");
    assert_eq!(created.scope["max_amount_minor"], 10_000);

    let listed = repo
        .list_mandates("ws_finance")
        .await
        .expect("list mandates");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "mandate_refund_bot");

    let revoked = repo
        .revoke_mandate("ws_finance", "mandate_refund_bot")
        .await
        .expect("revoke mandate");
    assert_eq!(revoked.status, FinancialMandateStatus::Revoked);

    let other = repo.list_mandates("ws_other").await.expect("other list");
    assert_eq!(other[0].status, FinancialMandateStatus::Active);
}

#[tokio::test]
async fn outcomes_append_and_list_by_action_without_affecting_spend() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let action = repo
        .create_action("ws_finance", refund_request("refund-bot", 7_500))
        .await
        .expect("create action");
    let other = repo
        .create_action("ws_other", refund_request("refund-bot", 7_500))
        .await
        .expect("create other action");

    repo.record_action_outcome(
        "ws_finance",
        &action.id,
        outcome(&action.id, FinancialActionOutcomeStatus::Succeeded),
    )
    .await
    .expect("record succeeded outcome");
    repo.record_action_outcome(
        "ws_finance",
        &action.id,
        outcome(&action.id, FinancialActionOutcomeStatus::RecoveryStarted),
    )
    .await
    .expect("record recovery outcome");
    repo.record_action_outcome(
        "ws_other",
        &other.id,
        outcome(&other.id, FinancialActionOutcomeStatus::Succeeded),
    )
    .await
    .expect("record other outcome");

    let outcomes = repo
        .list_action_outcomes("ws_finance", &action.id)
        .await
        .expect("list outcomes");

    assert_eq!(outcomes.len(), 2);
    assert_eq!(
        outcomes[0].outcome.status,
        FinancialActionOutcomeStatus::RecoveryStarted
    );
    assert_eq!(
        outcomes[1].outcome.status,
        FinancialActionOutcomeStatus::Succeeded
    );
    assert_eq!(
        outcomes[0].outcome.provider_reference.as_deref(),
        Some("provider_ref_123")
    );

    let other_outcomes = repo.list_action_outcomes("ws_other", &action.id).await;
    assert!(matches!(other_outcomes, Err(StorageError::NotFound)));

    let spend = repo
        .net_spend_minor(
            "ws_finance",
            "refund-bot",
            "USD",
            Utc::now() - Duration::hours(1),
            Utc::now() + Duration::hours(1),
        )
        .await
        .expect("spend unaffected by outcomes");
    assert_eq!(spend, 0);
}

#[tokio::test]
async fn receipts_create_and_get_are_tenant_scoped() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let action = repo
        .create_action("ws_finance", refund_request("refund-bot", 7_500))
        .await
        .expect("create action");
    let other = repo
        .create_action("ws_other", refund_request("refund-bot", 7_500))
        .await
        .expect("create other action");

    let receipt = repo
        .create_receipt(
            "ws_finance",
            &action.id,
            Some("018f4444-4444-7444-8444-444444444444"),
            vec!["ledger_reserve_1".into(), "ledger_execute_1".into()],
            serde_json::json!({
                "policy_id": "refund-cap-v1",
                "provider_reference": "refund_123",
                "mandate_id": "mandate_refund_bot"
            }),
        )
        .await
        .expect("create receipt");
    repo.create_receipt(
        "ws_other",
        &other.id,
        None,
        vec![],
        serde_json::json!({ "provider_reference": "other_refund" }),
    )
    .await
    .expect("create other receipt");

    assert_eq!(receipt.id, action.id);
    assert_eq!(receipt.action_id, action.id);
    assert_eq!(receipt.ledger_event_ids.len(), 2);
    assert_eq!(receipt.proof["policy_id"], "refund-cap-v1");

    let fetched = repo
        .get_receipt("ws_finance", &receipt.id)
        .await
        .expect("get receipt");
    assert_eq!(fetched.id, receipt.id);
    assert_eq!(
        fetched.trace_id.as_deref(),
        Some("018f4444-4444-7444-8444-444444444444")
    );

    match repo.get_receipt("ws_other", &receipt.id).await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected tenant-scoped NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn create_action_is_idempotent_and_tenant_scoped() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let mut request = refund_request("refund-bot", 7_500);
    request.idempotency_key = "idem-refund-75".into();

    let first = repo
        .create_action("ws_finance", request.clone())
        .await
        .expect("first create");
    let duplicate = repo
        .create_action("ws_finance", request.clone())
        .await
        .expect("duplicate create");
    let other_workspace = repo
        .create_action("ws_other", request)
        .await
        .expect("other workspace create");

    assert_eq!(first.id, duplicate.id);
    assert_ne!(first.id, other_workspace.id);
    assert_eq!(first.status, FinancialActionStatus::Proposed);

    match repo.get_action("ws_other", &first.id).await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected tenant-scoped NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn list_actions_is_tenant_scoped_and_newest_first() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);

    let first = repo
        .create_action("ws_finance", refund_request("refund-bot", 7_500))
        .await
        .expect("first create");
    let second = repo
        .create_action("ws_finance", refund_request("refund-bot", 8_500))
        .await
        .expect("second create");
    repo.create_action("ws_other", refund_request("refund-bot", 9_500))
        .await
        .expect("other workspace create");

    let listed = repo.list_actions("ws_finance").await.expect("list");

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, second.id);
    assert_eq!(listed[1].id, first.id);
}

#[tokio::test]
async fn approval_requests_are_tenant_scoped_and_newest_first() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let first = repo
        .create_action("ws_finance", refund_request("refund-bot", 7_500))
        .await
        .expect("first create");
    let second = repo
        .create_action("ws_finance", refund_request("refund-bot", 8_500))
        .await
        .expect("second create");
    let other = repo
        .create_action("ws_other", refund_request("refund-bot", 9_500))
        .await
        .expect("other create");

    repo.create_approval_request(
        "ws_finance",
        &first.id,
        "first hold",
        vec!["finance_admin".into()],
        None,
        serde_json::json!({ "sequence": 1 }),
    )
    .await
    .expect("first approval");
    repo.create_approval_request(
        "ws_finance",
        &second.id,
        "second hold",
        vec!["finance_admin".into()],
        None,
        serde_json::json!({ "sequence": 2 }),
    )
    .await
    .expect("second approval");
    repo.create_approval_request(
        "ws_other",
        &other.id,
        "other hold",
        vec!["finance_admin".into()],
        None,
        serde_json::json!({}),
    )
    .await
    .expect("other approval");

    let approvals = repo
        .list_approval_requests("ws_finance")
        .await
        .expect("list approvals");

    assert_eq!(approvals.len(), 2);
    assert_eq!(approvals[0].action_id, second.id);
    assert_eq!(approvals[0].status, FinancialApprovalRequestStatus::Pending);
    assert_eq!(approvals[1].action_id, first.id);
}

#[tokio::test]
async fn resolve_pending_approval_requests_updates_only_matching_action_queue_items() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let first = repo
        .create_action("ws_finance", refund_request("refund-bot", 7_500))
        .await
        .expect("first create");
    let second = repo
        .create_action("ws_finance", refund_request("refund-bot", 8_500))
        .await
        .expect("second create");
    let other = repo
        .create_action("ws_other", refund_request("refund-bot", 9_500))
        .await
        .expect("other create");

    repo.create_approval_request(
        "ws_finance",
        &first.id,
        "first hold",
        vec!["finance_admin".into()],
        None,
        serde_json::json!({}),
    )
    .await
    .expect("first approval");
    repo.create_approval_request(
        "ws_finance",
        &second.id,
        "second hold",
        vec!["finance_admin".into()],
        None,
        serde_json::json!({}),
    )
    .await
    .expect("second approval");
    repo.create_approval_request(
        "ws_other",
        &other.id,
        "other hold",
        vec!["finance_admin".into()],
        None,
        serde_json::json!({}),
    )
    .await
    .expect("other approval");

    repo.resolve_pending_approval_requests(
        "ws_finance",
        &first.id,
        FinancialApprovalRequestStatus::Approved,
    )
    .await
    .expect("resolve first");

    let approvals = repo
        .list_approval_requests("ws_finance")
        .await
        .expect("list approvals");
    let first_approval = approvals
        .iter()
        .find(|request| request.action_id == first.id)
        .expect("first approval exists");
    let second_approval = approvals
        .iter()
        .find(|request| request.action_id == second.id)
        .expect("second approval exists");

    assert_eq!(
        first_approval.status,
        FinancialApprovalRequestStatus::Approved
    );
    assert!(first_approval.decided_at.is_some());
    assert_eq!(
        second_approval.status,
        FinancialApprovalRequestStatus::Pending
    );

    let other_approvals = repo
        .list_approval_requests("ws_other")
        .await
        .expect("list other approvals");
    assert_eq!(
        other_approvals[0].status,
        FinancialApprovalRequestStatus::Pending
    );
}

#[tokio::test]
async fn status_transitions_append_events_and_reject_regressions() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let action = repo
        .create_action("ws_finance", refund_request("refund-bot", 7_500))
        .await
        .expect("create");
    let action_id = action.id.as_str();

    repo.transition_status(
        "ws_finance",
        action_id,
        FinancialActionStatus::Held,
        "approval_required",
        serde_json::json!({ "threshold_minor": 5_000 }),
    )
    .await
    .expect("hold");
    let executed = repo
        .transition_status(
            "ws_finance",
            action_id,
            FinancialActionStatus::Executed,
            "provider_executed",
            serde_json::json!({ "provider": "stripe" }),
        )
        .await
        .expect("execute");

    assert_eq!(executed.status, FinancialActionStatus::Executed);

    match repo
        .transition_status(
            "ws_finance",
            action_id,
            FinancialActionStatus::Held,
            "regress",
            serde_json::json!({}),
        )
        .await
    {
        Err(StorageError::Conflict) => {}
        other => panic!("expected Conflict for invalid transition, got {other:?}"),
    }

    let events = repo
        .list_action_events("ws_finance", action_id)
        .await
        .expect("events");
    let event_types: Vec<_> = events.into_iter().map(|event| event.event_type).collect();
    assert_eq!(
        event_types,
        vec!["created", "approval_required", "provider_executed"]
    );
}

#[tokio::test]
async fn spend_window_uses_net_reserved_and_executed_ledger_entries() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let start = Utc::now() - Duration::hours(1);
    let end = Utc::now() + Duration::hours(1);

    let held = repo
        .create_action("ws_finance", refund_request("refund-bot", 7_500))
        .await
        .expect("held create");
    let held_id = held.id.as_str();
    repo.record_ledger_entry(
        "ws_finance",
        held_id,
        FinancialLedgerEntryKind::Reserved,
        7_500,
        "USD",
        "held-reserve",
        serde_json::json!({}),
    )
    .await
    .expect("reserve held");

    let denied = repo
        .create_action("ws_finance", refund_request("refund-bot", 9_000))
        .await
        .expect("denied create");
    let denied_id = denied.id.as_str();
    repo.record_ledger_entry(
        "ws_finance",
        denied_id,
        FinancialLedgerEntryKind::Reserved,
        9_000,
        "USD",
        "denied-reserve",
        serde_json::json!({}),
    )
    .await
    .expect("reserve denied");
    repo.record_ledger_entry(
        "ws_finance",
        denied_id,
        FinancialLedgerEntryKind::Released,
        9_000,
        "USD",
        "denied-release",
        serde_json::json!({}),
    )
    .await
    .expect("release denied");

    let executed = repo
        .create_action("ws_finance", refund_request("refund-bot", 12_500))
        .await
        .expect("executed create");
    let executed_id = executed.id.as_str();
    repo.record_ledger_entry(
        "ws_finance",
        executed_id,
        FinancialLedgerEntryKind::Reserved,
        12_500,
        "USD",
        "executed-reserve",
        serde_json::json!({}),
    )
    .await
    .expect("reserve executed");
    repo.record_ledger_entry(
        "ws_finance",
        executed_id,
        FinancialLedgerEntryKind::Released,
        12_500,
        "USD",
        "executed-release",
        serde_json::json!({}),
    )
    .await
    .expect("release executed reserve");
    repo.record_ledger_entry(
        "ws_finance",
        executed_id,
        FinancialLedgerEntryKind::Executed,
        12_500,
        "USD",
        "executed-final",
        serde_json::json!({}),
    )
    .await
    .expect("executed final");

    let spend = repo
        .net_spend_minor("ws_finance", "refund-bot", "USD", start, end)
        .await
        .expect("spend");
    assert_eq!(spend, 20_000);
}
