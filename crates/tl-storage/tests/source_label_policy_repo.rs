//! SourceLabelPolicyRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test source_label_policy_repo

#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{Confidentiality, Origin, SourceLabelPolicy, Trust};
use tl_storage::{
    connect_postgres, migrate_postgres, schema::source_label_policy, SourceLabelPolicyRepo,
    StorageError,
};

async fn fresh_repo() -> (
    SourceLabelPolicyRepo,
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
    (SourceLabelPolicyRepo::new(pool), container)
}

fn sample_policy(origin: Origin) -> SourceLabelPolicy {
    SourceLabelPolicy {
        origin,
        trust: Some(Trust::Untrusted),
        confidentiality: Some(Confidentiality::Private),
        integrity: None,
    }
}

#[tokio::test]
async fn insert_and_get_round_trips_typed_policy() {
    let (repo, _c) = fresh_repo().await;
    let policy = sample_policy(Origin::Web);

    repo.upsert("default", &policy, true).await.expect("upsert");

    let stored = repo.get("default", Origin::Web).await.expect("get");
    assert_eq!(stored.policy, policy);
    assert!(stored.enabled);
}

#[tokio::test]
async fn list_returns_only_active_workspace_rows() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("ws_a", &sample_policy(Origin::Web), true)
        .await
        .expect("upsert a1");
    repo.upsert("ws_a", &sample_policy(Origin::Email), true)
        .await
        .expect("upsert a2");
    repo.upsert("ws_b", &sample_policy(Origin::Web), true)
        .await
        .expect("upsert b1");
    repo.delete("ws_a", Origin::Email).await.expect("delete");

    let listed = repo.list("ws_a").await.expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].policy.origin, Origin::Web);
}

#[tokio::test]
async fn list_includes_disabled_rows() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("default", &sample_policy(Origin::Web), false)
        .await
        .expect("upsert");

    let listed = repo.list("default").await.expect("list");
    assert_eq!(listed.len(), 1);
    assert!(!listed[0].enabled, "control plane must see disabled rows");
}

#[tokio::test]
async fn upsert_invalidates_workspace_cache() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("default", &sample_policy(Origin::Web), true)
        .await
        .expect("upsert");
    // Prime the cache.
    let _ = repo.list("default").await.expect("list");

    let mut updated = sample_policy(Origin::Web);
    updated.trust = Some(Trust::Trusted);
    repo.upsert("default", &updated, false)
        .await
        .expect("re-upsert");

    let listed = repo.list("default").await.expect("list");
    assert_eq!(
        listed[0].policy.trust,
        Some(Trust::Trusted),
        "cache must not serve stale rows after upsert"
    );
    assert!(!listed[0].enabled);
}

#[tokio::test]
async fn soft_delete_hides_from_get_and_list() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("default", &sample_policy(Origin::Web), true)
        .await
        .expect("upsert");

    repo.delete("default", Origin::Web).await.expect("delete");

    assert!(matches!(
        repo.get("default", Origin::Web).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repo.delete("default", Origin::Web).await,
        Err(StorageError::NotFound)
    ));
    assert!(repo.list("default").await.expect("list").is_empty());
}

#[tokio::test]
async fn upsert_revives_soft_deleted_row() {
    let (repo, _c) = fresh_repo().await;
    let policy = sample_policy(Origin::Web);
    repo.upsert("default", &policy, true).await.expect("upsert");
    repo.delete("default", Origin::Web).await.expect("delete");

    repo.upsert("default", &policy, true).await.expect("revive");

    let stored = repo.get("default", Origin::Web).await.expect("get");
    assert_eq!(stored.policy, policy);
}

#[tokio::test]
async fn second_list_uses_cache() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("default", &sample_policy(Origin::Web), true)
        .await
        .expect("upsert");
    let _ = repo.list("default").await.expect("first list");

    // Hard-delete in Postgres without going through repo.delete.
    let mut conn = repo.pool().get().await.expect("connection");
    diesel::delete(
        source_label_policy::table.filter(source_label_policy::workspace_id.eq("default")),
    )
    .execute(&mut conn)
    .await
    .expect("hard delete");

    // Cache must still serve the list within the TTL.
    let cached = repo.list("default").await.expect("cache hit");
    assert_eq!(cached.len(), 1);
}

#[tokio::test]
async fn empty_workspace_list_is_negatively_cached() {
    let (repo, _c) = fresh_repo().await;
    assert!(repo.list("default").await.expect("first list").is_empty());

    // Insert a row directly, bypassing the repo and its cache.
    let policy = sample_policy(Origin::Web);
    let mut conn = repo.pool().get().await.expect("connection");
    diesel::insert_into(source_label_policy::table)
        .values((
            source_label_policy::workspace_id.eq("default"),
            source_label_policy::origin.eq("web"),
            source_label_policy::spec.eq(serde_json::to_value(&policy).expect("serialize")),
            source_label_policy::enabled.eq(true),
        ))
        .execute(&mut conn)
        .await
        .expect("direct insert");

    // The empty list is authoritative within the TTL: served from cache,
    // not Postgres.
    assert!(repo.list("default").await.expect("cached list").is_empty());
}

#[tokio::test]
async fn get_is_isolated_by_workspace() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("ws_a", &sample_policy(Origin::Web), true)
        .await
        .expect("upsert");

    assert!(matches!(
        repo.get("ws_b", Origin::Web).await,
        Err(StorageError::NotFound)
    ));
}
