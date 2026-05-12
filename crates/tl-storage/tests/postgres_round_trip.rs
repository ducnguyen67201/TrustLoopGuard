//! Postgres integration tests via testcontainers. Off by default —
//! requires Docker. Enable with:
//!
//!   cargo test -p tl-storage --features postgres-it
//!
//! What we cover:
//! - Migration runs cleanly on a fresh database (idempotent on re-run).
//! - Decision round-trips through `put` + `get`.
//! - Unknown trace_id yields `StorageError::NotFound`.
//! - Putting the same trace_id twice is a no-op (idempotent insert).

#![cfg(feature = "postgres-it")]

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{new_trace_id, Decision, Verdict};
use tl_storage::{connect_postgres, migrate_postgres, DecisionStore, PostgresStore, StorageError};

async fn fresh_store() -> (
    PostgresStore,
    String,
    testcontainers::ContainerAsync<PostgresImage>,
) {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.expect("migrate");
    let pool = connect_postgres(&url, 4).await.expect("connect");
    (PostgresStore::new(pool), url, container)
}

fn fake_decision(trace_id: String, verdict: Verdict) -> Decision {
    let mut d = Decision::allow(trace_id);
    d.verdict = verdict;
    d.reason = "integration test".into();
    d.latency_ms = 42;
    d
}

#[tokio::test]
async fn migration_runs_clean_and_is_idempotent() {
    let (_store, url, _c) = fresh_store().await;
    migrate_postgres(&url).await.expect("idempotent");
}

#[tokio::test]
async fn put_then_get_round_trips() {
    let (store, _url, _c) = fresh_store().await;
    let id = new_trace_id();
    let original = fake_decision(id.clone(), Verdict::Block);
    store.put(&original).await.expect("put");

    let fetched = store.get(&id).await.expect("get");
    assert_eq!(fetched.trace_id, original.trace_id);
    assert_eq!(fetched.verdict, original.verdict);
    assert_eq!(fetched.reason, original.reason);
    assert_eq!(fetched.latency_ms, original.latency_ms);
}

#[tokio::test]
async fn missing_trace_id_returns_not_found() {
    let (store, _url, _c) = fresh_store().await;
    let id = new_trace_id();
    match store.get(&id).await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn duplicate_put_is_idempotent() {
    let (store, _url, _c) = fresh_store().await;
    let id = new_trace_id();
    let d = fake_decision(id.clone(), Verdict::Allow);
    store.put(&d).await.expect("first");
    store.put(&d).await.expect("second");
    // Idempotency: still fetchable, no panic.
    let _ = store.get(&id).await.expect("get");
}

#[tokio::test]
async fn invalid_uuid_returns_internal_error() {
    let (store, _url, _c) = fresh_store().await;
    let err = store.get("not-a-uuid").await.unwrap_err();
    assert!(matches!(err, StorageError::Internal(_)));
}
