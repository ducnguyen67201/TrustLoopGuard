//! `EscalationRepo` integration tests against testcontainers Postgres.

#![cfg(feature = "postgres-it")]

use std::time::Duration;

use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{migrate_postgres, EscalationRepo};
use uuid::Uuid;

async fn fresh_repo() -> (
    EscalationRepo,
    testcontainers::ContainerAsync<PostgresImage>,
) {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect");
    migrate_postgres(&pool).await.expect("migrate");
    (EscalationRepo::new(pool), container)
}

#[tokio::test]
async fn insert_then_mark_sent() {
    let (repo, _c) = fresh_repo().await;
    let id = Uuid::now_v7();
    let trace = Uuid::now_v7();
    let payload = json!({"trace_id": trace.to_string(), "agent_id": "a"});

    repo.insert_pending(id, trace, "https://example.test/h", &payload)
        .await
        .expect("insert");
    repo.record_attempt(id).await.expect("attempt 1");
    repo.mark_sent(id).await.expect("sent");
    // Stale-pending list should be empty after sending.
    let stale = repo
        .list_stale_pending(Duration::from_secs(0))
        .await
        .expect("list");
    assert!(stale.is_empty());
}

#[tokio::test]
async fn insert_then_mark_failed() {
    let (repo, _c) = fresh_repo().await;
    let id = Uuid::now_v7();
    repo.insert_pending(
        id,
        Uuid::now_v7(),
        "https://example.test/h",
        &json!({"k": "v"}),
    )
    .await
    .unwrap();
    repo.record_attempt(id).await.unwrap();
    repo.record_attempt(id).await.unwrap();
    repo.mark_failed(id).await.unwrap();
    // Failed rows are also off the pending list.
    let stale = repo
        .list_stale_pending(Duration::from_secs(0))
        .await
        .unwrap();
    assert!(stale.is_empty());
}

#[tokio::test]
async fn list_stale_returns_only_old_pending() {
    let (repo, _c) = fresh_repo().await;
    let id = Uuid::now_v7();
    repo.insert_pending(
        id,
        Uuid::now_v7(),
        "https://example.test/h",
        &json!({"k": "v"}),
    )
    .await
    .unwrap();

    // Just-inserted row should NOT count as stale (default cutoff > 0).
    let fresh = repo
        .list_stale_pending(Duration::from_secs(60))
        .await
        .unwrap();
    assert!(fresh.is_empty(), "row inserted just now must not be stale");

    // Cutoff of 0s lists every pending row.
    let all_pending = repo
        .list_stale_pending(Duration::from_secs(0))
        .await
        .unwrap();
    assert_eq!(all_pending.len(), 1);
    assert_eq!(all_pending[0].id, id);
    assert_eq!(all_pending[0].status, "pending");
}

#[tokio::test]
async fn record_attempt_increments_counter() {
    let (repo, _c) = fresh_repo().await;
    let id = Uuid::now_v7();
    repo.insert_pending(
        id,
        Uuid::now_v7(),
        "https://example.test/h",
        &json!({"k": "v"}),
    )
    .await
    .unwrap();
    for _ in 0..3 {
        repo.record_attempt(id).await.unwrap();
    }
    let rows = repo
        .list_stale_pending(Duration::from_secs(0))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].attempts, 3);
}
