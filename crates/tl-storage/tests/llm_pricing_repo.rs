//! Workspace LLM model price repository integration tests.
//!
//!   cargo test -p tl-storage --features postgres-it --test llm_pricing_repo

#![cfg(feature = "postgres-it")]

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{connect_postgres, migrate_postgres, DbPool, LlmPricingRepo};

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
async fn upsert_get_list_and_delete_round_trip() {
    let (pool, _container) = fresh_pool().await;
    let repo = LlmPricingRepo::new(pool);
    let ws = "ws_llm_pricing";

    repo.upsert_price(ws, "gpt-4o", 250, 1000, 2_500_000_000, 10_000_000_000)
        .await
        .expect("insert gpt-4o");
    repo.upsert_price(ws, "mystery-1", 100, 300, 1_000_000_000, 3_000_000_000)
        .await
        .expect("insert mystery-1");
    // Same-key upsert updates in place, no duplicate row.
    repo.upsert_price(ws, "gpt-4o", 500, 2000, 5_000_000_000, 20_000_000_000)
        .await
        .expect("update gpt-4o");

    let price = repo
        .get_price(ws, "gpt-4o")
        .await
        .expect("get")
        .expect("row exists");
    assert_eq!(price.input_per_million_minor, 500);
    assert_eq!(price.output_per_million_minor, 2000);
    assert_eq!(price.input_per_million_nanos, 5_000_000_000);
    // The migration's column default applies.
    assert_eq!(price.currency, "USD");

    assert!(repo.get_price(ws, "unknown").await.expect("get").is_none());

    let rows = repo.list_prices(ws).await.expect("list");
    assert_eq!(rows.len(), 2);
    // Model ascending.
    assert_eq!(rows[0].model, "gpt-4o");
    assert_eq!(rows[1].model, "mystery-1");

    // Workspace isolation.
    assert!(repo
        .get_price("ws_other", "gpt-4o")
        .await
        .expect("get")
        .is_none());
    assert!(repo.list_prices("ws_other").await.expect("list").is_empty());

    assert!(repo.delete_price(ws, "gpt-4o").await.expect("delete"));
    assert!(!repo.delete_price(ws, "gpt-4o").await.expect("re-delete"));
    assert!(repo.get_price(ws, "gpt-4o").await.expect("get").is_none());
    assert_eq!(repo.list_prices(ws).await.expect("list").len(), 1);
}
