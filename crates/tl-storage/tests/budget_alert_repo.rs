//! Budget alert repository integration tests.
//!
//!   cargo test -p tl-storage --features postgres-it --test budget_alert_repo

#![cfg(feature = "postgres-it")]

use chrono::{Duration, Utc};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{
    connect_postgres, migrate_postgres, BudgetAlertRepo, DbPool, NewBudgetAlertConfigParams,
    NewBudgetAlertFiringParams, StorageError, UpdateBudgetAlertConfigParams,
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

fn config(name: &str) -> NewBudgetAlertConfigParams {
    NewBudgetAlertConfigParams {
        name: name.into(),
        window: "week".into(),
        principal_id: None,
        threshold_type: "percent".into(),
        threshold_value: 80,
        webhook_url: Some("https://hooks.example.com/alerts".into()),
        enabled: true,
    }
}

fn firing(config_id: &str, principal: &str) -> NewBudgetAlertFiringParams {
    NewBudgetAlertFiringParams {
        config_id: config_id.into(),
        principal_id: principal.into(),
        window_start: Utc::now(),
        cap_minor: 5_000,
        spent_minor: 4_000,
        currency: "USD".into(),
        payload: serde_json::json!({ "type": "budget_alert" }),
    }
}

#[tokio::test]
async fn config_round_trip_and_firing_dedup() {
    let (pool, _container) = fresh_pool().await;
    let repo = BudgetAlertRepo::new(pool);
    let ws = "ws_budget_alerts";

    // Create + duplicate-name conflict.
    let created = repo
        .create_config(ws, config("weekly-80"))
        .await
        .expect("create");
    assert_eq!(created.window, "week");
    assert_eq!(created.threshold_value, 80);
    assert!(created.enabled);
    let duplicate = repo.create_config(ws, config("weekly-80")).await;
    assert!(matches!(duplicate, Err(StorageError::Conflict)));
    // Same name in another workspace is fine.
    repo.create_config("ws_other", config("weekly-80"))
        .await
        .expect("cross-workspace name reuse");

    // Read paths.
    let fetched = repo.get_config(ws, &created.id).await.expect("get");
    assert_eq!(fetched, created);
    assert_eq!(repo.list_configs(ws).await.expect("list").len(), 1);
    assert_eq!(
        repo.list_enabled_configs(ws).await.expect("enabled").len(),
        1
    );

    // Partial update leaves other fields alone.
    let updated = repo
        .update_config(
            ws,
            &created.id,
            UpdateBudgetAlertConfigParams {
                enabled: Some(false),
                threshold_value: Some(90),
                ..Default::default()
            },
        )
        .await
        .expect("update");
    assert!(!updated.enabled);
    assert_eq!(updated.threshold_value, 90);
    assert_eq!(updated.name, "weekly-80");
    assert!(repo
        .list_enabled_configs(ws)
        .await
        .expect("enabled")
        .is_empty());
    assert!(updated.updated_at >= created.updated_at);

    // Firing dedup: UNIQUE (config_id, principal_id, window_start).
    let first = firing(&created.id, "user:a");
    assert!(repo
        .try_record_firing(ws, first.clone())
        .await
        .expect("first firing"));
    assert!(!repo
        .try_record_firing(ws, first.clone())
        .await
        .expect("dedup is not an error"));
    // Different principal in the same window fires.
    assert!(repo
        .try_record_firing(
            ws,
            NewBudgetAlertFiringParams {
                principal_id: "user:b".into(),
                ..first.clone()
            }
        )
        .await
        .expect("second principal"));
    // A new window fires again.
    assert!(repo
        .try_record_firing(
            ws,
            NewBudgetAlertFiringParams {
                window_start: first.window_start + Duration::days(7),
                ..first
            }
        )
        .await
        .expect("new window"));

    let firings = repo
        .list_firings(ws, Some(&created.id))
        .await
        .expect("firings");
    assert_eq!(firings.len(), 3);
    assert_eq!(firings[0].cap_minor, 5_000);
    assert_eq!(firings[0].payload["type"], "budget_alert");
    assert_eq!(
        repo.list_firings(ws, None)
            .await
            .expect("all firings")
            .len(),
        3
    );
    // Unknown config id filters to nothing (and parses as NotFound).
    assert!(matches!(
        repo.list_firings(ws, Some("not-a-uuid")).await,
        Err(StorageError::NotFound)
    ));

    // Delete cascades firings.
    repo.delete_config(ws, &created.id).await.expect("delete");
    assert!(matches!(
        repo.get_config(ws, &created.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(repo
        .list_firings(ws, None)
        .await
        .expect("post-delete")
        .is_empty());
    assert!(matches!(
        repo.delete_config(ws, &created.id).await,
        Err(StorageError::NotFound)
    ));
}
