//! ToolMetadataRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test tool_metadata_repo

#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    AllowedSource, ApprovalRule, Origin, ParamRole, ParamSpec, SideEffectClass, ToolMetadata,
};
use tl_storage::{
    connect_postgres, migrate_postgres, schema::tool_metadata, StorageError, ToolMetadataRepo,
};

async fn fresh_repo() -> (
    ToolMetadataRepo,
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
    (ToolMetadataRepo::new(pool), container)
}

fn sample_metadata(tool: &str) -> ToolMetadata {
    ToolMetadata {
        tool: tool.into(),
        side_effect: SideEffectClass::ExternalCommunication,
        reversible: false,
        params: vec![ParamSpec {
            path: "recipient".into(),
            role: ParamRole::AuthorityBearing,
            allowed_sources: vec![AllowedSource {
                origin: Origin::User,
                source_id: None,
                kind: Some("chat_message".into()),
            }],
        }],
        approval: Some(ApprovalRule {
            required: true,
            approver_roles: vec!["admin".into()],
            reason: Some("external communication".into()),
        }),
        sandbox_hint: Some(serde_json::json!({ "network": "egress_allowlist" })),
    }
}

#[tokio::test]
async fn insert_and_get_round_trips_typed_metadata() {
    let (repo, _c) = fresh_repo().await;
    let metadata = sample_metadata("send_email");

    repo.upsert("default", &metadata, true)
        .await
        .expect("upsert");

    let stored = repo.get("default", "send_email").await.expect("get");
    assert_eq!(stored.metadata, metadata);
    assert!(stored.enabled);
}

#[tokio::test]
async fn list_returns_only_active_workspace_rows() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("ws_a", &sample_metadata("send_email"), true)
        .await
        .expect("upsert a1");
    repo.upsert("ws_a", &sample_metadata("delete_record"), true)
        .await
        .expect("upsert a2");
    repo.upsert("ws_b", &sample_metadata("send_email"), true)
        .await
        .expect("upsert b1");
    repo.delete("ws_a", "delete_record").await.expect("delete");

    let listed = repo.list("ws_a").await.expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].metadata.tool, "send_email");
}

#[tokio::test]
async fn upsert_refreshes_cache() {
    let (repo, _c) = fresh_repo().await;
    let metadata = sample_metadata("send_email");
    repo.upsert("default", &metadata, true)
        .await
        .expect("upsert");
    // Prime the cache.
    let _ = repo.get("default", "send_email").await.expect("get");

    let mut updated = metadata.clone();
    updated.reversible = true;
    repo.upsert("default", &updated, false)
        .await
        .expect("re-upsert");

    let stored = repo.get("default", "send_email").await.expect("get");
    assert!(stored.metadata.reversible, "cache must not serve stale row");
    assert!(!stored.enabled);
}

#[tokio::test]
async fn soft_delete_hides_from_get() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("default", &sample_metadata("send_email"), true)
        .await
        .expect("upsert");

    repo.delete("default", "send_email").await.expect("delete");

    assert!(matches!(
        repo.get("default", "send_email").await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repo.delete("default", "send_email").await,
        Err(StorageError::NotFound)
    ));
}

#[tokio::test]
async fn upsert_revives_soft_deleted_row() {
    let (repo, _c) = fresh_repo().await;
    let metadata = sample_metadata("send_email");
    repo.upsert("default", &metadata, true)
        .await
        .expect("upsert");
    repo.delete("default", "send_email").await.expect("delete");

    repo.upsert("default", &metadata, true)
        .await
        .expect("revive");

    let stored = repo.get("default", "send_email").await.expect("get");
    assert_eq!(stored.metadata, metadata);
}

#[tokio::test]
async fn disabled_row_still_readable_with_flag() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("default", &sample_metadata("send_email"), false)
        .await
        .expect("upsert");

    let stored = repo.get("default", "send_email").await.expect("get");
    assert!(!stored.enabled);
    assert_eq!(stored.metadata.tool, "send_email");
}

#[tokio::test]
async fn get_is_isolated_by_workspace() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("ws_a", &sample_metadata("send_email"), true)
        .await
        .expect("upsert");

    assert!(matches!(
        repo.get("ws_b", "send_email").await,
        Err(StorageError::NotFound)
    ));
}

#[tokio::test]
async fn second_get_uses_cache() {
    let (repo, _c) = fresh_repo().await;
    repo.upsert("default", &sample_metadata("send_email"), true)
        .await
        .expect("upsert");
    let _ = repo.get("default", "send_email").await.expect("first");

    // Hard-delete in Postgres without going through repo.delete.
    let mut conn = repo.pool().get().await.expect("connection");
    diesel::delete(tool_metadata::table.filter(tool_metadata::tool.eq("send_email")))
        .execute(&mut conn)
        .await
        .expect("hard delete");

    // Cache must still serve the value within the TTL.
    let cached = repo.get("default", "send_email").await.expect("cache hit");
    assert_eq!(cached.metadata.tool, "send_email");
}

#[tokio::test]
async fn negative_cache_serves_repeated_misses() {
    let (repo, _c) = fresh_repo().await;
    assert!(matches!(
        repo.get("default", "unknown_tool").await,
        Err(StorageError::NotFound)
    ));

    // Insert the row directly, bypassing the repo and its cache.
    let metadata = sample_metadata("unknown_tool");
    let mut conn = repo.pool().get().await.expect("connection");
    diesel::insert_into(tool_metadata::table)
        .values((
            tool_metadata::workspace_id.eq("default"),
            tool_metadata::tool.eq("unknown_tool"),
            tool_metadata::side_effect.eq("external_communication"),
            tool_metadata::reversible.eq(false),
            tool_metadata::spec.eq(serde_json::to_value(&metadata).expect("serialize")),
            tool_metadata::enabled.eq(true),
        ))
        .execute(&mut conn)
        .await
        .expect("direct insert");

    // The negative entry is authoritative within the TTL: the miss is
    // served from cache, not Postgres.
    assert!(matches!(
        repo.get("default", "unknown_tool").await,
        Err(StorageError::NotFound)
    ));
}
