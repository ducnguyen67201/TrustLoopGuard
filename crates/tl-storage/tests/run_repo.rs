#![cfg(feature = "postgres-it")]

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{CreateRunRequest, RunKind, RunStatus, UpdateRunRequest};
use tl_storage::{connect_postgres, migrate_postgres, DbPool, RunFilter, RunRepo};

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
    (pool, container)
}

#[tokio::test]
async fn create_list_and_update_run() {
    let (pool, _container) = fresh_pool().await;
    let repo = RunRepo::new(pool);

    let created = repo
        .create(
            "ws_test",
            CreateRunRequest {
                agent_id: "agent-a".into(),
                kind: RunKind::Workflow,
                status: None,
                external_id: Some("workflow-1".into()),
                metadata: serde_json::json!({ "source": "n8n" }),
            },
        )
        .await
        .expect("create");

    assert_eq!(created.status, RunStatus::Running);
    assert_eq!(created.external_id.as_deref(), Some("workflow-1"));

    let rows = repo
        .list(
            "ws_test",
            RunFilter {
                external_id: Some("workflow-1".into()),
                limit: 10,
                ..RunFilter::default()
            },
        )
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, created.id);

    let updated = repo
        .update(
            "ws_test",
            &created.id,
            UpdateRunRequest {
                status: Some(RunStatus::Completed),
                metadata: None,
                ended_at: None,
            },
        )
        .await
        .expect("update");
    assert_eq!(updated.status, RunStatus::Completed);
    assert!(updated.ended_at.is_some());
}
