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

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{new_trace_id, AuthorizationEffect, Decision};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, workspace_environments, workspaces},
    DecisionStore, PostgresStore, StorageError,
};

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
    // Traces carry a workspace/environment foreign key; seed the rows
    // `PostgresStore::put` writes against (it stamps the backwards-compatible
    // `default` workspace and `production` environment).
    {
        let mut conn = pool.get().await.expect("connection");
        diesel::insert_into(organizations::table)
            .values((
                organizations::id.eq("org_default"),
                organizations::name.eq("Default Org"),
                organizations::slug.eq("default-org"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert organization");
        diesel::insert_into(workspaces::table)
            .values((
                workspaces::id.eq("default"),
                workspaces::organization_id.eq("org_default"),
                workspaces::name.eq("Default Workspace"),
                workspaces::slug.eq("default"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert workspace");
        diesel::insert_into(workspace_environments::table)
            .values((
                workspace_environments::workspace_id.eq("default"),
                workspace_environments::id.eq("production"),
                workspace_environments::slug.eq("production"),
                workspace_environments::name.eq("Production"),
                workspace_environments::is_default.eq(true),
            ))
            .execute(&mut conn)
            .await
            .expect("insert environment");
    }
    (PostgresStore::new(pool), url, container)
}

fn fake_decision(trace_id: String, effect: AuthorizationEffect) -> Decision {
    let mut d = Decision::allow(trace_id);
    d.effect = effect;
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
    let original = fake_decision(id.clone(), AuthorizationEffect::Deny);
    store.put(&original).await.expect("put");

    let fetched = store.get(&id).await.expect("get");
    assert_eq!(fetched.trace_id, original.trace_id);
    assert_eq!(fetched.effect, original.effect);
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
    let d = fake_decision(id.clone(), AuthorizationEffect::Permit);
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
