#![cfg(feature = "postgres-it")]

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{NotificationDeliveryStatus, NotificationEventKind};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{notification_deliveries, organizations, workspace_environments, workspaces},
    DbPool, NotificationRepo,
};

async fn fresh_pool() -> (DbPool, testcontainers::ContainerAsync<PostgresImage>) {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.expect("migrate");
    let pool = connect_postgres(&url, 8).await.expect("connect");
    let mut conn = pool.get().await.expect("connection");
    diesel::insert_into(organizations::table)
        .values((
            organizations::id.eq("org_notify"),
            organizations::name.eq("Notify Org"),
            organizations::slug.eq("notify-org"),
        ))
        .execute(&mut conn)
        .await
        .expect("insert organization");
    diesel::insert_into(workspaces::table)
        .values((
            workspaces::id.eq("ws_notify"),
            workspaces::organization_id.eq("org_notify"),
            workspaces::name.eq("Notify Workspace"),
            workspaces::slug.eq("notify"),
        ))
        .execute(&mut conn)
        .await
        .expect("insert workspace");
    diesel::insert_into(workspace_environments::table)
        .values((
            workspace_environments::workspace_id.eq("ws_notify"),
            workspace_environments::id.eq("production"),
            workspace_environments::slug.eq("production"),
            workspace_environments::name.eq("Production"),
            workspace_environments::is_default.eq(true),
        ))
        .execute(&mut conn)
        .await
        .expect("insert environment");
    drop(conn);
    (pool, container)
}

#[tokio::test]
async fn delivery_dedup_claim_retry_and_expired_lease_reclaim() {
    let (pool, _container) = fresh_pool().await;
    let repo = NotificationRepo::new(pool.clone());
    repo.create_rule(
        "ws_notify",
        "production",
        None,
        "ops@example.com".into(),
        vec![
            NotificationEventKind::EvaluationFailed,
            NotificationEventKind::Test,
        ],
        true,
    )
    .await
    .expect("create rule");

    let enqueue = || {
        repo.enqueue_matching(
            "ws_notify",
            "production",
            None,
            None,
            NotificationEventKind::EvaluationFailed,
            "result-1",
            "snapshot-1",
            None,
            serde_json::json!({"title": "Evaluation failed"}),
        )
    };
    assert_eq!(enqueue().await.expect("first enqueue"), 1);
    assert_eq!(enqueue().await.expect("deduplicated enqueue"), 0);

    let claimed = repo
        .claim_delivery("worker-a", 60)
        .await
        .expect("claim")
        .expect("pending delivery");
    assert!(repo
        .claim_delivery("worker-b", 60)
        .await
        .expect("second claim")
        .is_none());
    repo.retry_or_fail("ws_notify", &claimed.delivery.id, 1, "smtp", "failed")
        .await
        .expect("terminal retry transition");
    assert_eq!(
        repo.list_deliveries("ws_notify", "production", 10)
            .await
            .expect("list deliveries")[0]
            .status,
        NotificationDeliveryStatus::Failed
    );

    assert_eq!(
        repo.enqueue_matching(
            "ws_notify",
            "production",
            None,
            None,
            NotificationEventKind::Test,
            "test-1",
            "v1",
            None,
            serde_json::json!({"title": "Test"}),
        )
        .await
        .expect("enqueue test delivery"),
        1
    );
    let first_claim = repo
        .claim_delivery("worker-a", 60)
        .await
        .expect("claim test")
        .expect("test delivery");
    let mut conn = pool.get().await.expect("connection");
    diesel::update(
        notification_deliveries::table
            .filter(notification_deliveries::workspace_id.eq("ws_notify"))
            .filter(
                notification_deliveries::id
                    .eq(uuid::Uuid::parse_str(&first_claim.delivery.id).expect("delivery id")),
            ),
    )
    .set(notification_deliveries::lease_expires_at.eq(Some(Utc::now() - Duration::seconds(1))))
    .execute(&mut conn)
    .await
    .expect("expire lease");
    drop(conn);
    let reclaimed = repo
        .claim_delivery("worker-b", 60)
        .await
        .expect("reclaim")
        .expect("expired delivery");
    assert_eq!(reclaimed.delivery.id, first_claim.delivery.id);
    assert_eq!(reclaimed.delivery.attempt_count, 2);
}
