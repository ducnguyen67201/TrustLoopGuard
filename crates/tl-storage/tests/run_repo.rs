#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    CreateRunEventRequest, CreateRunRequest, RunEventKind, RunKind, RunStatus, UpdateRunRequest,
};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, workspace_environments, workspaces},
    DbPool, RunFilter, RunRepo,
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
    (pool, container)
}

#[tokio::test]
async fn create_list_and_update_run() {
    let (pool, _container) = fresh_pool().await;
    let repo = RunRepo::new(pool);

    let created = repo
        .create(
            "ws_test",
            "production",
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

#[tokio::test]
async fn create_event_rejects_invalid_input() {
    let (pool, _container) = fresh_pool().await;
    let repo = RunRepo::new(pool);
    let run = repo
        .create(
            "ws_test",
            "production",
            CreateRunRequest {
                agent_id: "agent-a".into(),
                kind: RunKind::Workflow,
                status: None,
                external_id: None,
                metadata: serde_json::json!({}),
            },
        )
        .await
        .expect("create run");

    let sequence_err = repo
        .create_event(
            "ws_test",
            &run.id,
            CreateRunEventRequest {
                kind: RunEventKind::WorkflowStep,
                sequence: Some(0),
                label: None,
                input_summary: None,
                output_summary: None,
                metadata: serde_json::json!({}),
                occurred_at: None,
            },
        )
        .await
        .expect_err("sequence zero should fail");
    assert!(sequence_err.to_string().contains("sequence"));

    let metadata_err = repo
        .create_event(
            "ws_test",
            &run.id,
            CreateRunEventRequest {
                kind: RunEventKind::WorkflowStep,
                sequence: None,
                label: None,
                input_summary: None,
                output_summary: None,
                metadata: serde_json::json!([]),
                occurred_at: None,
            },
        )
        .await
        .expect_err("non-object metadata should fail");
    assert!(metadata_err.to_string().contains("metadata"));
}

#[tokio::test]
async fn create_event_auto_sequence_is_concurrency_safe() {
    let (pool, _container) = fresh_pool().await;
    let repo = RunRepo::new(pool);
    let run = repo
        .create(
            "ws_test",
            "production",
            CreateRunRequest {
                agent_id: "agent-a".into(),
                kind: RunKind::Workflow,
                status: None,
                external_id: None,
                metadata: serde_json::json!({}),
            },
        )
        .await
        .expect("create run");

    let mut handles = Vec::new();
    for index in 0..12 {
        let repo = repo.clone();
        let run_id = run.id.clone();
        handles.push(tokio::spawn(async move {
            repo.create_event(
                "ws_test",
                &run_id,
                CreateRunEventRequest {
                    kind: RunEventKind::WorkflowStep,
                    sequence: None,
                    label: Some(format!("step {index}")),
                    input_summary: None,
                    output_summary: None,
                    metadata: serde_json::json!({}),
                    occurred_at: None,
                },
            )
            .await
        }));
    }

    for handle in handles {
        handle.await.expect("task").expect("create event");
    }

    let events = repo.events("ws_test", &run.id, 20).await.expect("events");
    let sequences: Vec<i32> = events.into_iter().map(|event| event.sequence).collect();
    assert_eq!(sequences, (1..=12).collect::<Vec<_>>());
}
