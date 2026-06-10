//! DashboardAdminRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test dashboard_admin_repo

#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, workspace_environments, workspaces},
    DashboardAdminRepo, StorageError,
};

async fn fresh_repo() -> (
    DashboardAdminRepo,
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
    {
        let mut conn = pool.get().await.expect("connection");
        diesel::insert_into(organizations::table)
            .values((
                organizations::id.eq("org_test"),
                organizations::name.eq("Test Org"),
                organizations::slug.eq("test-org"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert organization");
        diesel::insert_into(workspaces::table)
            .values((
                workspaces::id.eq("ws_test"),
                workspaces::organization_id.eq("org_test"),
                workspaces::name.eq("Test Workspace"),
                workspaces::slug.eq("test"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert workspace");
        diesel::insert_into(workspace_environments::table)
            .values((
                workspace_environments::workspace_id.eq("ws_test"),
                workspace_environments::id.eq("production"),
                workspace_environments::slug.eq("production"),
                workspace_environments::name.eq("Production"),
                workspace_environments::is_default.eq(true),
            ))
            .execute(&mut conn)
            .await
            .expect("insert environment");
    }
    (DashboardAdminRepo::new(pool), container)
}

#[tokio::test]
async fn batch_revoke_api_keys_updates_status_and_auth_lookup() {
    let (repo, _c) = fresh_repo().await;
    repo.create_api_key(
        "apk_one",
        "ws_test",
        "production",
        "One",
        "tl_live_one",
        "hash_one",
        None,
    )
    .await
    .unwrap();
    repo.create_api_key(
        "apk_two",
        "ws_test",
        "production",
        "Two",
        "tl_live_two",
        "hash_two",
        None,
    )
    .await
    .unwrap();

    let ids = vec!["apk_one".to_string(), "apk_two".to_string()];
    let rows = repo.batch_revoke_api_keys("ws_test", &ids).await.unwrap();

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.status == "revoked"));
    assert!(repo
        .verify_api_key_hash("hash_one")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn batch_revoke_api_keys_is_workspace_scoped() {
    let (repo, _c) = fresh_repo().await;
    repo.create_api_key(
        "apk_one",
        "ws_test",
        "production",
        "One",
        "tl_live_one",
        "hash_one",
        None,
    )
    .await
    .unwrap();

    let ids = vec!["apk_one".to_string()];
    match repo.batch_revoke_api_keys("ws_other", &ids).await {
        Err(StorageError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }

    let rows = repo.list_api_keys("ws_test").await.unwrap();
    assert_eq!(rows[0].status, "active");
}
