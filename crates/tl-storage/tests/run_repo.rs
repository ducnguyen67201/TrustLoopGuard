#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    CreateRunEventRequest, CreateRunRequest, FinalizeRunRequest, RunBoundarySource, RunEventKind,
    RunKind, RunStatus, UpdateRunRequest,
};
use tl_storage::{
    connect_postgres, migrate_postgres,
    models::NewRunSpan,
    schema::{agents, organizations, run_spans, runs, workspace_environments, workspaces},
    DbPool, RunFilter, RunRepo, StorageError,
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
        diesel::insert_into(agents::table)
            .values((
                agents::workspace_id.eq("ws_test"),
                agents::id.eq("agent-a"),
                agents::profile_yaml.eq("id: agent-a"),
                agents::parsed_profile.eq(serde_json::json!({"agent_id":"agent-a"})),
            ))
            .execute(&mut conn)
            .await
            .expect("insert agent");
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
async fn lists_run_spans_in_waterfall_order() {
    let (pool, _container) = fresh_pool().await;
    let repo = RunRepo::new(pool.clone());
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
    let run_id = uuid::Uuid::parse_str(&run.id).expect("run uuid");
    let root_started = chrono::Utc::now();
    let child_started = root_started + chrono::Duration::milliseconds(10);
    let spans = [
        NewRunSpan {
            workspace_id: "ws_test".into(),
            environment_id: "production".into(),
            run_id,
            agent_id: "agent-a".into(),
            run_event_id: None,
            otel_trace_id: "0123456789abcdef0123456789abcdef".into(),
            otel_span_id: "1111111111111111".into(),
            parent_span_id: Some("0000000000000001".into()),
            name: "model call".into(),
            span_kind: 3,
            operation_name: Some("chat".into()),
            conversation_id: None,
            external_agent_id: None,
            started_at: child_started,
            ended_at: child_started + chrono::Duration::milliseconds(20),
            status_code: 1,
            status_message: None,
            resource: serde_json::json!({
                "attributes": {"service.name": "model-gateway"},
                "scope": {"name": "test-instrumentation"}
            }),
            attributes: serde_json::json!({"gen_ai.request.model": "example-model"}),
            events: serde_json::json!([]),
            links: serde_json::json!([]),
            content_capture_status: "metadata_only".into(),
            dropped_attribute_count: 0,
            late_evidence: false,
        },
        NewRunSpan {
            workspace_id: "ws_test".into(),
            environment_id: "production".into(),
            run_id,
            agent_id: "agent-a".into(),
            run_event_id: None,
            otel_trace_id: "0123456789abcdef0123456789abcdef".into(),
            otel_span_id: "0000000000000001".into(),
            parent_span_id: None,
            name: "agent turn".into(),
            span_kind: 2,
            operation_name: Some("workflow".into()),
            conversation_id: None,
            external_agent_id: Some("customer-agent".into()),
            started_at: root_started,
            ended_at: root_started + chrono::Duration::milliseconds(50),
            status_code: 1,
            status_message: None,
            resource: serde_json::json!({
                "attributes": {"service.name": "customer-agent"},
                "scope": {"name": "test-instrumentation"}
            }),
            attributes: serde_json::json!({}),
            events: serde_json::json!([]),
            links: serde_json::json!([]),
            content_capture_status: "metadata_only".into(),
            dropped_attribute_count: 0,
            late_evidence: false,
        },
    ];
    let mut conn = pool.get().await.expect("connection");
    diesel::insert_into(run_spans::table)
        .values(&spans)
        .execute(&mut conn)
        .await
        .expect("insert spans");

    let listed = repo
        .spans("ws_test", &run.id, 100)
        .await
        .expect("list spans");

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].span_id, "0000000000000001");
    assert_eq!(
        listed[1].parent_span_id.as_deref(),
        Some("0000000000000001")
    );
    assert_eq!(
        listed[1].resource["attributes"]["service.name"],
        "model-gateway"
    );
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
                agent_id: None,
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
                agent_id: None,
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
                    agent_id: None,
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

#[tokio::test]
async fn finalization_is_idempotent_and_closes_the_event_boundary() {
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
    let request = FinalizeRunRequest {
        status: RunStatus::Completed,
        ended_at: None,
        boundary_source: RunBoundarySource::ExplicitSdk,
        expected_flush_id: Some("flush-1".into()),
        last_event_sequence: Some(0),
    };

    let first = repo
        .finalize("ws_test", "production", &run.id, request.clone(), 1_000)
        .await
        .expect("first finalization");
    let retry = repo
        .finalize("ws_test", "production", &run.id, request, 1_000)
        .await
        .expect("idempotent retry");
    assert_eq!(
        first.finalization.finalized_at,
        retry.finalization.finalized_at
    );

    let conflict = repo
        .finalize(
            "ws_test",
            "production",
            &run.id,
            FinalizeRunRequest {
                status: RunStatus::Failed,
                ended_at: None,
                boundary_source: RunBoundarySource::ExplicitSdk,
                expected_flush_id: Some("flush-1".into()),
                last_event_sequence: None,
            },
            1_000,
        )
        .await
        .expect_err("conflicting terminal transition");
    assert!(matches!(conflict, StorageError::Conflict));

    let late_event = repo
        .create_event(
            "ws_test",
            &run.id,
            CreateRunEventRequest {
                agent_id: None,
                kind: RunEventKind::WorkflowStep,
                sequence: None,
                label: None,
                input_summary: None,
                output_summary: None,
                metadata: serde_json::json!({}),
                occurred_at: None,
            },
        )
        .await
        .expect_err("post-finalization event");
    assert!(matches!(late_event, StorageError::Conflict));
}

#[tokio::test]
async fn concurrent_gateway_session_has_one_active_winner_and_can_restart_after_terminal() {
    let (pool, _container) = fresh_pool().await;
    let repo = RunRepo::new(pool.clone());
    let input = CreateRunRequest {
        agent_id: "agent-a".into(),
        kind: RunKind::ChatSession,
        status: Some(RunStatus::Running),
        external_id: Some("customer-session-1".into()),
        metadata: serde_json::json!({
            "integration_mode": "gateway",
            "route_id": "route-a"
        }),
    };

    let (left, right) = tokio::join!(
        repo.create("ws_test", "production", input.clone()),
        repo.create("ws_test", "production", input.clone())
    );
    let winner = match (left, right) {
        (Ok(winner), Err(StorageError::Conflict)) | (Err(StorageError::Conflict), Ok(winner)) => {
            winner
        }
        outcomes => panic!("expected one winner and one conflict, got {outcomes:?}"),
    };
    let active = repo
        .list(
            "ws_test",
            RunFilter {
                environment_id: Some("production".into()),
                agent_id: Some("agent-a".into()),
                kind: Some(RunKind::ChatSession),
                status: Some(RunStatus::Running),
                external_id: Some("customer-session-1".into()),
                limit: 10,
            },
        )
        .await
        .expect("list active session");
    assert_eq!(active.len(), 1);

    {
        let mut conn = pool.get().await.expect("connection");
        let stale_at = chrono::Utc::now() - chrono::Duration::minutes(10);
        diesel::update(
            runs::table
                .filter(runs::workspace_id.eq("ws_test"))
                .filter(runs::id.eq(uuid::Uuid::parse_str(&winner.id).unwrap())),
        )
        .set(runs::last_evidence_at.eq(stale_at))
        .execute(&mut conn)
        .await
        .expect("age gateway activity");
    }
    let stale = repo
        .list_stale_gateway_runs(
            chrono::Utc::now() - chrono::Duration::minutes(5),
            chrono::Utc::now() - chrono::Duration::hours(1),
            10,
        )
        .await
        .expect("list stale gateway sessions");
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].run_id, winner.id);
    assert!(!stale[0].max_duration_exceeded);

    repo.finalize(
        "ws_test",
        "production",
        &winner.id,
        FinalizeRunRequest {
            status: RunStatus::Completed,
            ended_at: None,
            boundary_source: RunBoundarySource::FrameworkAdapter,
            expected_flush_id: None,
            last_event_sequence: None,
        },
        1_000,
    )
    .await
    .expect("finalize winner");

    let later = repo
        .create("ws_test", "production", input)
        .await
        .expect("same customer session starts a later bounded run");
    assert_ne!(later.id, winner.id);
    assert_eq!(later.status, RunStatus::Running);
}
