//! PolicyRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test policy_repo

#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::AuthorizationEffect;
use tl_core::Severity;
use tl_policy::{MatchClause, Matcher, Policy};
use tl_storage::{connect_postgres, migrate_postgres, schema::policies, PolicyRepo, StorageError};

const WORKSPACE_ID: &str = "ws_policy_repo_test";

async fn fresh_repo() -> (PolicyRepo, testcontainers::ContainerAsync<PostgresImage>) {
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
    (PolicyRepo::new(pool), container)
}

fn sample_policy(id: &str) -> Policy {
    Policy {
        id: id.into(),
        description: Some(format!("{id} description")),
        when: Default::default(),
        r#match: MatchClause::Single(Matcher::Literal("guaranteed refund".into())),
        action: AuthorizationEffect::Deny,
        rewrite: None,
        severity: Severity::High,
        owner_agent_id: None,
    }
}

fn source_yaml(id: &str) -> String {
    format!(
        r#"
id: {id}
description: Prevents guaranteed refund promises.
match:
  literal: guaranteed refund
action: deny
severity: high
"#
    )
}

#[tokio::test]
async fn upsert_and_get_round_trips_policy() {
    let (repo, _c) = fresh_repo().await;
    let policy = sample_policy("refund-guarantee");
    let yaml = source_yaml(&policy.id);
    repo.upsert_in(WORKSPACE_ID, &policy, &yaml)
        .await
        .expect("upsert");

    let fetched = repo
        .get_in(WORKSPACE_ID, "refund-guarantee")
        .await
        .expect("get");
    assert_eq!(fetched.id, "refund-guarantee");
    assert!(matches!(fetched.action, AuthorizationEffect::Deny));
    assert_eq!(fetched.severity, Severity::High);
}

#[tokio::test]
async fn upsert_stores_source_yaml_for_audit() {
    let (repo, _c) = fresh_repo().await;
    let yaml = source_yaml("audit");
    repo.upsert_in(WORKSPACE_ID, &sample_policy("audit"), &yaml)
        .await
        .expect("upsert");

    let mut conn = repo.pool().get().await.expect("connection");
    let source_yaml = policies::table
        .filter(policies::id.eq("audit"))
        .select(policies::policy_yaml)
        .first::<String>(&mut conn)
        .await
        .expect("source yaml");
    assert_eq!(source_yaml, yaml);
}

#[tokio::test]
async fn list_enabled_filters_disabled_and_deleted() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert_in(
        WORKSPACE_ID,
        &sample_policy("active"),
        &source_yaml("active"),
    )
    .await
    .unwrap();
    repo.upsert_in(
        WORKSPACE_ID,
        &sample_policy("disabled"),
        &source_yaml("disabled"),
    )
    .await
    .unwrap();
    repo.upsert_in(
        WORKSPACE_ID,
        &sample_policy("deleted"),
        &source_yaml("deleted"),
    )
    .await
    .unwrap();

    repo.set_enabled_in(WORKSPACE_ID, "disabled", false)
        .await
        .unwrap();
    repo.delete_in(WORKSPACE_ID, "deleted").await.unwrap();

    let ids: Vec<_> = repo
        .list_enabled_in(WORKSPACE_ID)
        .await
        .unwrap()
        .into_iter()
        .map(|policy| policy.id.clone())
        .collect();
    assert_eq!(ids, vec!["active"]);
}

#[tokio::test]
async fn upsert_after_delete_resurrects_policy_enabled() {
    let (repo, _c) = fresh_repo().await;
    let policy = sample_policy("zombie");
    let yaml = source_yaml(&policy.id);
    repo.upsert_in(WORKSPACE_ID, &policy, &yaml).await.unwrap();
    repo.set_enabled_in(WORKSPACE_ID, "zombie", false)
        .await
        .unwrap();
    repo.delete_in(WORKSPACE_ID, "zombie").await.unwrap();

    repo.upsert_in(WORKSPACE_ID, &policy, &yaml).await.unwrap();

    let ids: Vec<_> = repo
        .list_enabled_in(WORKSPACE_ID)
        .await
        .unwrap()
        .into_iter()
        .map(|policy| policy.id.clone())
        .collect();
    assert_eq!(ids, vec!["zombie"]);
}

#[tokio::test]
async fn missing_policy_returns_not_found() {
    let (repo, _c) = fresh_repo().await;
    match repo.get_in(WORKSPACE_ID, "missing").await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn set_enabled_requires_existing_active_policy() {
    let (repo, _c) = fresh_repo().await;
    match repo.set_enabled_in(WORKSPACE_ID, "missing", false).await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn batch_set_enabled_is_atomic_for_missing_policy() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert_in(
        WORKSPACE_ID,
        &sample_policy("active"),
        &source_yaml("active"),
    )
    .await
    .unwrap();

    let ids = vec!["active".to_string(), "missing".to_string()];
    match repo.batch_set_enabled_in(WORKSPACE_ID, &ids, false).await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }

    let records = repo.list_records_in(WORKSPACE_ID).await.unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].enabled);
}

#[tokio::test]
async fn batch_set_enabled_updates_all_selected_policies() {
    let (repo, _c) = fresh_repo().await;
    for id in ["alpha", "beta", "gamma"] {
        repo.upsert_in(WORKSPACE_ID, &sample_policy(id), &source_yaml(id))
            .await
            .unwrap();
    }

    let ids = vec!["alpha".to_string(), "gamma".to_string()];
    let rows = repo
        .batch_set_enabled_in(WORKSPACE_ID, &ids, false)
        .await
        .unwrap();
    let ids: Vec<_> = rows.into_iter().map(|row| row.policy.id).collect();
    assert_eq!(ids, vec!["alpha", "gamma"]);

    let enabled_ids: Vec<_> = repo
        .list_enabled_in(WORKSPACE_ID)
        .await
        .unwrap()
        .into_iter()
        .map(|policy| policy.id.clone())
        .collect();
    assert_eq!(enabled_ids, vec!["beta"]);
}
