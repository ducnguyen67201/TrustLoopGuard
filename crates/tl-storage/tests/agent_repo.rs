//! AgentRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test agent_repo

#![cfg(feature = "postgres-it")]

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    AgentAuthority, AgentProfile, AgentScope, AgentTone, KnowledgeSource,
};
use tl_storage::{migrate_postgres, AgentRepo, StorageError};

async fn fresh_repo() -> (AgentRepo, testcontainers::ContainerAsync<PostgresImage>) {
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
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect");
    migrate_postgres(&pool).await.expect("migrate");
    (AgentRepo::new(pool), container)
}

fn sample_profile(id: &str) -> AgentProfile {
    AgentProfile {
        agent_id: id.into(),
        display_name: format!("{id} display"),
        scope: AgentScope {
            in_scope: vec!["billing".into()],
            out_of_scope: vec!["legal".into()],
        },
        authority: AgentAuthority {
            can_promise: vec!["respond within 24h".into()],
            cannot_promise: vec!["refunds".into()],
        },
        tone: AgentTone {
            target: "warm-professional".into(),
            forbidden: vec!["sarcastic".into()],
        },
        knowledge_sources: vec![KnowledgeSource {
            kb_id: "acme-help".into(),
        }],
        escalation_triggers: vec!["self-harm".into()],
    }
}

const SOURCE_YAML: &str = "id: minimal\n";

#[tokio::test]
async fn upsert_and_get_round_trips() {
    let (repo, _c) = fresh_repo().await;
    let profile = sample_profile("acme-support-v3");
    repo.upsert(&profile, SOURCE_YAML).await.expect("upsert");

    let fetched = repo.get("acme-support-v3").await.expect("get");
    assert_eq!(fetched.agent_id, "acme-support-v3");
    assert_eq!(fetched.display_name, "acme-support-v3 display");
    assert_eq!(fetched.scope.in_scope, vec!["billing".to_string()]);
    assert_eq!(fetched.authority.cannot_promise, vec!["refunds".to_string()]);
}

#[tokio::test]
async fn upsert_overwrites_existing() {
    let (repo, _c) = fresh_repo().await;
    let mut p = sample_profile("dual");
    repo.upsert(&p, SOURCE_YAML).await.expect("first");
    p.display_name = "second display".into();
    repo.upsert(&p, SOURCE_YAML).await.expect("second");
    let got = repo.get("dual").await.expect("get");
    assert_eq!(got.display_name, "second display");
}

#[tokio::test]
async fn missing_agent_returns_not_found() {
    let (repo, _c) = fresh_repo().await;
    match repo.get("nope").await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn delete_makes_subsequent_get_not_found() {
    let (repo, _c) = fresh_repo().await;
    let p = sample_profile("transient");
    repo.upsert(&p, SOURCE_YAML).await.expect("upsert");
    assert!(repo.get("transient").await.is_ok());

    repo.delete("transient").await.expect("delete");
    match repo.get("transient").await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound after delete, got {other:?}"),
    }
}

#[tokio::test]
async fn delete_is_idempotent_on_missing() {
    let (repo, _c) = fresh_repo().await;
    match repo.delete("never-existed").await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn upsert_after_delete_resurrects_the_agent() {
    let (repo, _c) = fresh_repo().await;
    let p = sample_profile("zombie");
    repo.upsert(&p, SOURCE_YAML).await.expect("upsert");
    repo.delete("zombie").await.expect("delete");
    repo.upsert(&p, SOURCE_YAML).await.expect("upsert again");
    let got = repo.get("zombie").await.expect("get");
    assert_eq!(got.agent_id, "zombie");
}

#[tokio::test]
async fn list_returns_only_active_agents() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert(&sample_profile("a"), SOURCE_YAML).await.unwrap();
    repo.upsert(&sample_profile("b"), SOURCE_YAML).await.unwrap();
    repo.upsert(&sample_profile("c"), SOURCE_YAML).await.unwrap();
    repo.delete("b").await.unwrap();

    let all = repo.list().await.expect("list");
    let ids: Vec<&str> = all.iter().map(|p| p.agent_id.as_str()).collect();
    assert_eq!(ids, vec!["a", "c"]);
}

#[tokio::test]
async fn second_get_uses_cache() {
    // Verifies cache hit by deleting the underlying row directly via SQL
    // (bypassing the repo so the cache isn't invalidated). If the cache
    // is working, `get` still succeeds; otherwise it would `NotFound`.
    let (repo, _c) = fresh_repo().await;
    let p = sample_profile("cached");
    repo.upsert(&p, SOURCE_YAML).await.unwrap();

    // Populate cache.
    let _ = repo.get("cached").await.expect("first");

    // Hard-delete in Postgres without going through repo.delete.
    sqlx::query(r#"DELETE FROM "Agent" WHERE id = $1"#)
        .bind("cached")
        .execute(repo.pool())
        .await
        .expect("hard delete");

    // Cache must still serve the value.
    let cached = repo.get("cached").await.expect("cache hit");
    assert_eq!(cached.agent_id, "cached");
}

#[tokio::test]
async fn capacity_zero_disables_cache() {
    // Same hard-delete trick: with capacity 0 the cache stores nothing,
    // so a hard-deleted row must surface as NotFound on the next get.
    let (_, c) = fresh_repo().await;
    let host = c.get_host().await.unwrap();
    let port = c.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    let repo = AgentRepo::with_cache(pool.clone(), 0, Duration::from_secs(1));
    repo.upsert(&sample_profile("nocache"), SOURCE_YAML).await.unwrap();
    let _ = repo.get("nocache").await.unwrap();

    sqlx::query(r#"DELETE FROM "Agent" WHERE id = $1"#)
        .bind("nocache")
        .execute(&pool)
        .await
        .unwrap();

    match repo.get("nocache").await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound (no cache), got {other:?}"),
    }
}
