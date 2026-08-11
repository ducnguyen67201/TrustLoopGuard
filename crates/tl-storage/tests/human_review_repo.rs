#![cfg(feature = "postgres-it")]

use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    AuthorizationEffect, CreateHumanReviewEventRequest, CreateRunEventRequest, CreateRunRequest,
    HumanReviewOutcome, RunEventKind, RunKind,
};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{agents, organizations, traces, workspace_environments, workspaces},
    DbPool, HumanReviewAnalyticsFilter, HumanReviewRepo, RunRepo,
};
use uuid::Uuid;

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
                organizations::id.eq("org_review"),
                organizations::name.eq("Review Org"),
                organizations::slug.eq("review-org"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert organization");
        diesel::insert_into(workspaces::table)
            .values((
                workspaces::id.eq("ws_review"),
                workspaces::organization_id.eq("org_review"),
                workspaces::name.eq("Review Workspace"),
                workspaces::slug.eq("review"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert workspace");
        diesel::insert_into(workspace_environments::table)
            .values((
                workspace_environments::workspace_id.eq("ws_review"),
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
                agents::workspace_id.eq("ws_review"),
                agents::id.eq("tax-agent"),
                agents::profile_yaml.eq("id: tax-agent"),
                agents::parsed_profile.eq(serde_json::json!({"agent_id":"tax-agent"})),
            ))
            .execute(&mut conn)
            .await
            .expect("insert agent");
    }
    (pool, container)
}

async fn insert_trace(
    pool: &DbPool,
    workspace_id: &str,
    trace_id: Uuid,
    run_id: Option<Uuid>,
    run_event_id: Option<Uuid>,
    effect: AuthorizationEffect,
    policy_id: &str,
    agent_id: &str,
) {
    let decision = match effect {
        AuthorizationEffect::Permit => "permit",
        AuthorizationEffect::Transform => "transform",
        AuthorizationEffect::RequireApproval => "require_approval",
        AuthorizationEffect::Defer => "defer",
        AuthorizationEffect::Deny => "deny",
    };
    let mut conn = pool.get().await.expect("connection");
    diesel::insert_into(traces::table)
        .values((
            traces::workspace_id.eq(workspace_id),
            traces::environment_id.eq("production"),
            traces::trace_id.eq(trace_id),
            traces::run_id.eq(run_id),
            traces::run_event_id.eq(run_event_id),
            traces::domain.eq("customer_support"),
            traces::decision.eq(decision),
            traces::elapsed_ms.eq(42),
            traces::payload.eq(serde_json::json!({
                "trace_id": trace_id.to_string(),
                "effect": decision,
                "agent_id": agent_id,
                "triggered_policies": [{ "id": policy_id, "severity": "high", "reason": "test" }]
            })),
            traces::created_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .expect("insert trace");
}

#[tokio::test]
async fn review_events_are_append_only_and_latest_is_queryable() {
    let (pool, _container) = fresh_pool().await;
    let repo = HumanReviewRepo::new(pool.clone());
    let trace_id = Uuid::now_v7();
    insert_trace(
        &pool,
        "ws_review",
        trace_id,
        None,
        None,
        AuthorizationEffect::RequireApproval,
        "tax-sensitive-data",
        "tax-agent",
    )
    .await;

    repo.create_event(
        "ws_review",
        &trace_id.to_string(),
        CreateHumanReviewEventRequest {
            outcome: HumanReviewOutcome::Accepted,
            reason_codes: vec!["policy_noise".into()],
            note: None,
            metadata: serde_json::json!({ "source": "dashboard" }),
        },
        Some("reviewer-1".into()),
    )
    .await
    .expect("create accepted");
    let corrected = repo
        .create_event(
            "ws_review",
            &trace_id.to_string(),
            CreateHumanReviewEventRequest {
                outcome: HumanReviewOutcome::Corrected,
                reason_codes: vec!["field_mismatch".into()],
                note: Some("Reviewer corrected a field.".into()),
                metadata: serde_json::json!({}),
            },
            Some("reviewer-2".into()),
        )
        .await
        .expect("create corrected");

    let events = repo
        .list_events("ws_review", &trace_id.to_string(), 10)
        .await
        .expect("list events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].outcome, HumanReviewOutcome::Accepted);
    assert_eq!(events[1].outcome, HumanReviewOutcome::Corrected);

    let latest = repo
        .latest_by_trace_ids("ws_review", &[trace_id.to_string()])
        .await
        .expect("latest");
    assert_eq!(latest.get(&trace_id.to_string()), Some(&corrected));
}

#[tokio::test]
async fn analytics_distinguishes_guardrail_and_human_interventions() {
    let (pool, _container) = fresh_pool().await;
    let run_repo = RunRepo::new(pool.clone());
    let review_repo = HumanReviewRepo::new(pool.clone());
    let run = run_repo
        .create(
            "ws_review",
            "production",
            CreateRunRequest {
                agent_id: "tax-agent".into(),
                kind: RunKind::Workflow,
                status: None,
                external_id: None,
                metadata: serde_json::json!({}),
            },
        )
        .await
        .expect("create run");
    let event = run_repo
        .create_event(
            "ws_review",
            &run.id,
            CreateRunEventRequest {
                agent_id: None,
                kind: RunEventKind::WorkflowStep,
                sequence: None,
                label: Some("Extract W2".into()),
                input_summary: None,
                output_summary: None,
                metadata: serde_json::json!({ "workflow_step": "document_extraction" }),
                occurred_at: None,
            },
        )
        .await
        .expect("create event");

    let run_id = Uuid::parse_str(&run.id).expect("run id");
    let run_event_id = Uuid::parse_str(&event.id).expect("run event id");
    let allow = Uuid::now_v7();
    let block = Uuid::now_v7();
    let rewrite = Uuid::now_v7();
    let escalate = Uuid::now_v7();
    insert_trace(
        &pool,
        "ws_review",
        allow,
        Some(run_id),
        Some(run_event_id),
        AuthorizationEffect::Permit,
        "baseline",
        "tax-agent",
    )
    .await;
    insert_trace(
        &pool,
        "ws_review",
        block,
        Some(run_id),
        Some(run_event_id),
        AuthorizationEffect::Deny,
        "tax-sensitive-data",
        "tax-agent",
    )
    .await;
    insert_trace(
        &pool,
        "ws_review",
        rewrite,
        Some(run_id),
        Some(run_event_id),
        AuthorizationEffect::Transform,
        "tax-sensitive-data",
        "tax-agent",
    )
    .await;
    insert_trace(
        &pool,
        "ws_review",
        escalate,
        Some(run_id),
        Some(run_event_id),
        AuthorizationEffect::RequireApproval,
        "tax-sensitive-data",
        "tax-agent",
    )
    .await;

    for (trace_id, outcome, reason) in [
        (block, HumanReviewOutcome::Corrected, "field_mismatch"),
        (rewrite, HumanReviewOutcome::FalsePositive, "policy_noise"),
        (allow, HumanReviewOutcome::MissedIssue, "unsupported_claim"),
    ] {
        review_repo
            .create_event(
                "ws_review",
                &trace_id.to_string(),
                CreateHumanReviewEventRequest {
                    outcome,
                    reason_codes: vec![reason.into()],
                    note: None,
                    metadata: serde_json::json!({}),
                },
                None,
            )
            .await
            .expect("create review");
    }

    let analytics = review_repo
        .analytics(
            "ws_review",
            HumanReviewAnalyticsFilter {
                agent_id: Some("tax-agent".into()),
                workflow_step: Some("document_extraction".into()),
                ..HumanReviewAnalyticsFilter::default()
            },
        )
        .await
        .expect("analytics");

    assert_eq!(analytics.summary.trace_count, 4);
    assert_eq!(analytics.summary.automated_intervention_count, 3);
    assert_eq!(analytics.summary.human_review_count, 3);
    assert_eq!(analytics.summary.human_intervention_count, 2);
    assert_eq!(analytics.summary.human_intervention_rate, 50.0);
    assert_eq!(analytics.summary.false_positive_rate, 25.0);
    assert_eq!(analytics.outcomes.corrected_count, 1);
    assert_eq!(analytics.outcomes.false_positive_count, 1);
    assert_eq!(analytics.outcomes.missed_issue_count, 1);
    assert_eq!(
        analytics.by_workflow_step[0].workflow_step,
        "document_extraction"
    );
    assert_eq!(analytics.by_workflow_step[0].corrected_count, 1);
    assert_eq!(analytics.by_policy[0].policy_id, "tax-sensitive-data");
    assert_eq!(analytics.by_policy[0].corrected_count, 1);
    assert_eq!(analytics.top_reasons[0].reason_code, "field_mismatch");
}
