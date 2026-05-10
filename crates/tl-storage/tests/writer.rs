//! Trace writer integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test writer

#![cfg(feature = "postgres-it")]

use std::time::{Duration, Instant};

use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{new_trace_id, Decision, Verdict};
use tl_storage::{migrate_postgres, spawn_writer, TraceWrite, WriterConfig};

async fn fresh_pool() -> (sqlx::PgPool, testcontainers::ContainerAsync<PostgresImage>) {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await
        .expect("connect");
    migrate_postgres(&pool).await.expect("migrate");
    (pool, container)
}

fn fake_decision() -> Decision {
    let mut d = Decision::allow(new_trace_id());
    d.verdict = Verdict::Allow;
    d.latency_ms = 12;
    d
}

#[tokio::test]
async fn caller_send_is_non_blocking_under_load() {
    // Merge gate: 1000 traces written via channel, caller never blocks > 5 ms.
    // We measure each `try_send` call individually so a single slow writer
    // can't be hidden by amortised throughput.
    let (pool, _c) = fresh_pool().await;
    let (tx, handle) = spawn_writer(pool.clone(), WriterConfig::default());

    for _ in 0..1_000 {
        let w = TraceWrite {
            decision: fake_decision(),
            domain: "customer_support".into(),
        };
        let start = Instant::now();
        // try_send ensures the test asserts on the FAST path even when
        // the channel fills up — a Full would surface as a panic here.
        tx.try_send(w)
            .expect("channel must accept under default capacity");
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(5),
            "send took {elapsed:?}, exceeds 5ms gate"
        );
    }

    // Drop the sender → writer flushes remaining buffer and exits.
    drop(tx);
    handle.await.expect("writer task");

    // Confirm everything actually persisted.
    let row = sqlx::query(r#"SELECT COUNT(*)::BIGINT as n FROM "Traces""#)
        .fetch_one(&pool)
        .await
        .expect("count");
    let n: i64 = row.get("n");
    assert_eq!(n, 1_000, "expected 1000 rows persisted, got {n}");
}

#[tokio::test]
async fn batch_size_triggers_flush() {
    // Configure a small batch and a long interval so we know the size
    // trigger is what produced the flush.
    let (pool, _c) = fresh_pool().await;
    let cfg = WriterConfig {
        buffer_size: 256,
        batch_size: 10,
        flush_interval: Duration::from_secs(60),
    };
    let (tx, _handle) = spawn_writer(pool.clone(), cfg);

    for _ in 0..10 {
        tx.send(TraceWrite {
            decision: fake_decision(),
            domain: "customer_support".into(),
        })
        .await
        .expect("send");
    }

    // Give the writer a moment to drain + flush. Batch hits at 10 entries.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let row = sqlx::query(r#"SELECT COUNT(*)::BIGINT as n FROM "Traces""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    let n: i64 = row.get("n");
    assert_eq!(n, 10, "size-triggered flush did not persist");
}

#[tokio::test]
async fn interval_flushes_partial_batch() {
    // Send fewer than batch_size; verify the time-based trigger
    // still flushes them within ~2x the interval.
    let (pool, _c) = fresh_pool().await;
    let cfg = WriterConfig {
        buffer_size: 256,
        batch_size: 100,
        flush_interval: Duration::from_millis(30),
    };
    let (tx, _handle) = spawn_writer(pool.clone(), cfg);

    for _ in 0..5 {
        tx.send(TraceWrite {
            decision: fake_decision(),
            domain: "customer_support".into(),
        })
        .await
        .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(120)).await;

    let row = sqlx::query(r#"SELECT COUNT(*)::BIGINT as n FROM "Traces""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    let n: i64 = row.get("n");
    assert_eq!(n, 5, "interval-triggered flush did not persist");
}

#[tokio::test]
async fn graceful_shutdown_flushes_remaining() {
    let (pool, _c) = fresh_pool().await;
    let cfg = WriterConfig {
        buffer_size: 256,
        batch_size: 1_000_000, // never hit by size in this test
        flush_interval: Duration::from_secs(60), // never hit by time
    };
    let (tx, handle) = spawn_writer(pool.clone(), cfg);

    for _ in 0..7 {
        tx.send(TraceWrite {
            decision: fake_decision(),
            domain: "customer_support".into(),
        })
        .await
        .unwrap();
    }

    // Closing the channel forces the final flush even though neither
    // batch nor interval triggered.
    drop(tx);
    handle.await.expect("graceful shutdown");

    let row = sqlx::query(r#"SELECT COUNT(*)::BIGINT as n FROM "Traces""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    let n: i64 = row.get("n");
    assert_eq!(n, 7);
}
