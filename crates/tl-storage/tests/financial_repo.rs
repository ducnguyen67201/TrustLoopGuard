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
    CounterpartyRef, CreateFinancialActionRequest, FinancialActionKind, FinancialActionStatus,
    FinancialRail, MoneyAmount,
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
