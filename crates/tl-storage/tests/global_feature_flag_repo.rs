//! GlobalFeatureFlagRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test global_feature_flag_repo

#![cfg(feature = "postgres-it")]

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{connect_postgres, migrate_postgres, GlobalFeatureFlagRepo};

async fn fresh_repo() -> (
    GlobalFeatureFlagRepo,
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
    (GlobalFeatureFlagRepo::new(pool), container)
}

#[tokio::test]
async fn seeded_knowledge_grounding_flag_defaults_to_disabled() {
    let (repo, _container) = fresh_repo().await;

    assert!(!repo.is_enabled("knowledge_grounding", true).await.unwrap());
}

#[tokio::test]
async fn set_enabled_upserts_global_flag() {
    let (repo, _container) = fresh_repo().await;

    let enabled = repo
        .set_enabled("knowledge_grounding", true, Some("test"))
        .await
        .unwrap();
    assert!(enabled.enabled);
    assert_eq!(enabled.updated_by.as_deref(), Some("test"));
    assert!(repo.is_enabled("knowledge_grounding", false).await.unwrap());

    let disabled = repo
        .set_enabled("knowledge_grounding", false, Some("test"))
        .await
        .unwrap();
    assert!(!disabled.enabled);
    assert!(!repo.is_enabled("knowledge_grounding", true).await.unwrap());
}
