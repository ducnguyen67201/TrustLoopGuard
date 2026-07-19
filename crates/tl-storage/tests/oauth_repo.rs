#![cfg(feature = "postgres-it")]

use std::sync::Arc;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, users, workspaces},
    DbPool, NewOAuthAuthorizationCode, OAuthRepo, StorageError,
};
use uuid::Uuid;

async fn fresh() -> (DbPool, Uuid, testcontainers::ContainerAsync<PostgresImage>) {
    let container = PostgresImage::default().start().await.unwrap();
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.unwrap();
    let pool = connect_postgres(&url, 8).await.unwrap();
    let user = Uuid::new_v4();
    let mut conn = pool.get().await.unwrap();
    diesel::insert_into(organizations::table)
        .values((
            organizations::id.eq("org"),
            organizations::name.eq("Org"),
            organizations::slug.eq("org"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    diesel::insert_into(workspaces::table)
        .values((
            workspaces::id.eq("ws"),
            workspaces::organization_id.eq("org"),
            workspaces::name.eq("Workspace"),
            workspaces::slug.eq("ws"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    diesel::insert_into(users::table)
        .values((
            users::id.eq(user),
            users::username.eq("member@example.com"),
            users::password_hash.eq("hash"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    drop(conn);
    (pool, user, container)
}

#[tokio::test]
async fn authorization_code_is_hash_only_and_atomically_single_use() {
    let (pool, user, _container) = fresh().await;
    let repo = Arc::new(OAuthRepo::new(pool.clone()));
    repo.create_client_bounded(
        "client",
        Some("AI client"),
        &["https://client.example/callback".into()],
        10_000,
    )
    .await
    .unwrap();
    assert!(matches!(
        repo.create_client_bounded(
            "over-capacity",
            None,
            &["https://client.example/callback".into()],
            1,
        )
        .await,
        Err(StorageError::Conflict)
    ));
    let raw = "raw-secret-code";
    let hash = tl_server_hash_for_test(raw);
    repo.put_code(NewOAuthAuthorizationCode {
        code_hash: hash.clone(),
        client_id: "client".into(),
        redirect_uri: "https://client.example/callback".into(),
        user_id: user,
        username: "member@example.com".into(),
        workspace_id: "ws".into(),
        resource: "https://guard.example/mcp".into(),
        scope: "mcp:tools".into(),
        code_challenge: "challenge".into(),
        expires_at: Utc::now() + Duration::minutes(1),
    })
    .await
    .unwrap();
    let mut conn = pool.get().await.unwrap();
    let stored = tl_storage::schema::mcp_oauth_authorization_codes::table
        .select(tl_storage::schema::mcp_oauth_authorization_codes::code_hash)
        .first::<String>(&mut conn)
        .await
        .unwrap();
    assert_eq!(stored, hash);
    assert_ne!(stored, raw);
    drop(conn);
    let (one, two) = tokio::join!(repo.take_code_by_hash(&hash), repo.take_code_by_hash(&hash));
    assert!(one.is_ok() ^ two.is_ok());
    assert!(matches!(
        one.err().or(two.err()),
        Some(StorageError::NotFound)
    ));

    let mut conn = pool.get().await.unwrap();
    diesel::update(
        tl_storage::schema::mcp_oauth_clients::table
            .filter(tl_storage::schema::mcp_oauth_clients::client_id.eq("client")),
    )
    .set(tl_storage::schema::mcp_oauth_clients::created_at.eq(Utc::now() - Duration::days(31)))
    .execute(&mut conn)
    .await
    .unwrap();
    drop(conn);
    assert_eq!(
        repo.prune_inactive_clients(Utc::now() - Duration::days(30))
            .await
            .unwrap(),
        1
    );
    assert!(matches!(
        repo.get_client("client").await,
        Err(StorageError::NotFound)
    ));
}

fn tl_server_hash_for_test(value: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
