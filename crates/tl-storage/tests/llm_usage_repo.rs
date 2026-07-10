//! LLM usage metering repository integration tests.
//!
//!   cargo test -p tl-storage --features postgres-it --test llm_usage_repo

#![cfg(feature = "postgres-it")]

use chrono::{Duration, Utc};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{
    connect_postgres, migrate_postgres, DbPool, LlmBudgetCapsNanos, LlmUsageEventFilter,
    LlmUsageGroupBy, LlmUsageRepo, NewLlmBudgetReservationParams, NewLlmUsageEventParams,
    ReserveLlmBudgetResult,
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
    (pool, container)
}

fn reservation(
    request_id: &str,
    reserved_nanos: i64,
    cap_nanos: i64,
) -> NewLlmBudgetReservationParams {
    let now = Utc::now();
    NewLlmBudgetReservationParams {
        request_id: request_id.into(),
        principal_id: "user:daniel".into(),
        api_key_id: "key_1".into(),
        currency: "USD".into(),
        reserved_nanos,
        caps: LlmBudgetCapsNanos {
            weekly: Some(cap_nanos),
            ..Default::default()
        },
        day_start: now - Duration::days(1),
        week_start: now - Duration::days(7),
        month_start: now - Duration::days(31),
        now: now + Duration::seconds(1),
    }
}

fn event(principal: &str, model: &str, cost: i64, request_id: &str) -> NewLlmUsageEventParams {
    NewLlmUsageEventParams {
        principal_id: principal.into(),
        api_key_id: "key_1".into(),
        model: model.into(),
        prompt_tokens: 1_000,
        completion_tokens: 200,
        cost_minor: cost,
        cost_nanos: cost.saturating_mul(10_000_000),
        currency: "USD".into(),
        request_id: request_id.into(),
        metadata: serde_json::json!({ "route_id": "route" }),
    }
}

#[tokio::test]
async fn insert_window_sum_and_grouping_round_trip() {
    let (pool, _container) = fresh_pool().await;
    let repo = LlmUsageRepo::new(pool);
    let ws = "ws_llm_usage";

    repo.insert_event(ws, event("user:daniel", "deepseek-chat", 137, "req-1"))
        .await
        .expect("insert 1");
    repo.insert_event(ws, event("user:daniel", "gpt-4o", 50, "req-2"))
        .await
        .expect("insert 2");
    repo.insert_event(ws, event("user:other", "deepseek-chat", 9, "req-3"))
        .await
        .expect("insert 3");
    // Retried metering write for req-1: idempotent, no duplicate row.
    repo.insert_event(ws, event("user:daniel", "deepseek-chat", 137, "req-1"))
        .await
        .expect("idempotent re-insert");

    let now = Utc::now();
    let spent = repo
        .net_llm_spend_minor(
            ws,
            "user:daniel",
            "usd",
            now - Duration::hours(1),
            now + Duration::hours(1),
        )
        .await
        .expect("window sum");
    assert_eq!(spent, 187);

    let outside = repo
        .net_llm_spend_minor(
            ws,
            "user:daniel",
            "USD",
            now - Duration::hours(2),
            now - Duration::hours(1),
        )
        .await
        .expect("empty window sum");
    assert_eq!(outside, 0);

    let events = repo
        .list_events(ws, &LlmUsageEventFilter::default())
        .await
        .expect("list");
    assert_eq!(events.len(), 3);

    let daniel_only = repo
        .list_events(
            ws,
            &LlmUsageEventFilter {
                principal_id: Some("user:daniel".into()),
                ..Default::default()
            },
        )
        .await
        .expect("filtered list");
    assert_eq!(daniel_only.len(), 2);

    let by_principal = repo
        .grouped_usage(
            ws,
            LlmUsageGroupBy::Principal,
            &LlmUsageEventFilter::default(),
        )
        .await
        .expect("grouped by principal");
    assert_eq!(by_principal.len(), 2);
    assert_eq!(by_principal[0].key, "user:daniel");
    assert_eq!(by_principal[0].cost_minor, 187);
    assert_eq!(by_principal[0].calls, 2);
    assert_eq!(by_principal[1].key, "user:other");
    assert_eq!(by_principal[1].cost_minor, 9);

    let by_model = repo
        .grouped_usage(ws, LlmUsageGroupBy::Model, &LlmUsageEventFilter::default())
        .await
        .expect("grouped by model");
    assert_eq!(by_model.len(), 2);
    assert_eq!(by_model[0].key, "deepseek-chat");
    assert_eq!(by_model[0].calls, 2);
    assert_eq!(by_model[0].prompt_tokens, 2_000);

    let by_day = repo
        .grouped_usage(ws, LlmUsageGroupBy::Day, &LlmUsageEventFilter::default())
        .await
        .expect("grouped by day");
    assert_eq!(by_day.len(), 1);
    assert_eq!(by_day[0].key, Utc::now().date_naive().to_string());
    assert_eq!(by_day[0].calls, 3);
    assert_eq!(by_day[0].cost_minor, 196);

    // Workspace isolation.
    let other_ws = repo
        .list_events("ws_other", &LlmUsageEventFilter::default())
        .await
        .expect("other workspace list");
    assert!(other_ws.is_empty());
}

#[tokio::test]
async fn concurrent_reservations_are_atomic_and_settlement_releases_unused_budget() {
    let (pool, _container) = fresh_pool().await;
    let repo = LlmUsageRepo::new(pool);
    let workspace = "ws_llm_reservations";

    let first = repo.reserve_budget(workspace, reservation("req-a", 60, 100));
    let second = repo.reserve_budget(workspace, reservation("req-b", 60, 100));
    let (first, second) = tokio::join!(first, second);
    let outcomes = [first.unwrap(), second.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ReserveLlmBudgetResult::Reserved))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ReserveLlmBudgetResult::Exceeded { .. }))
            .count(),
        1
    );

    let reserved_request = if matches!(outcomes[0], ReserveLlmBudgetResult::Reserved) {
        "req-a"
    } else {
        "req-b"
    };
    let mut actual = event("user:daniel", "deepseek-chat", 0, reserved_request);
    actual.cost_nanos = 20;
    repo.settle_budget(workspace, reserved_request, actual)
        .await
        .expect("settle reservation");

    let third = repo
        .reserve_budget(workspace, reservation("req-c", 60, 100))
        .await
        .expect("reserve after settlement");
    assert_eq!(third, ReserveLlmBudgetResult::Reserved);
}
