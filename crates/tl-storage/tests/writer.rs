//! Trace writer integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test writer

#![cfg(feature = "postgres-it")]

use std::time::{Duration, Instant};

use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{new_trace_id, Decision, Verdict};
use tl_storage::{
    connect_postgres, migrate_postgres, schema::traces, spawn_writer, DbPool, TraceWrite,
    WriterConfig,
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

async fn trace_count(pool: &DbPool) -> i64 {
    let mut conn = pool.get().await.expect("connection");
    traces::table
        .select(count_star())
        .first::<i64>(&mut conn)
        .await
        .expect("count")
}

async fn trace_environment_count(pool: &DbPool, environment_id: &str) -> i64 {
    let mut conn = pool.get().await.expect("connection");
    traces::table
        .filter(traces::environment_id.eq(environment_id))
        .select(count_star())
        .first::<i64>(&mut conn)
        .await
        .expect("environment count")
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
            workspace_id: "default".into(),
            environment_id: "production".into(),
            decision: fake_decision(),
            run_id: None,
            run_event_id: None,
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
    let n = trace_count(&pool).await;
    assert_eq!(n, 1_000, "expected 1000 rows persisted, got {n}");
    let env_n = trace_environment_count(&pool, "production").await;
    assert_eq!(
        env_n, 1_000,
        "expected all traces in production, got {env_n}"
    );
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
            workspace_id: "default".into(),
            environment_id: "production".into(),
            decision: fake_decision(),
            run_id: None,
            run_event_id: None,
            domain: "customer_support".into(),
        })
        .await
        .expect("send");
    }

    // Give the writer a moment to drain + flush. Batch hits at 10 entries.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let n = trace_count(&pool).await;
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
            workspace_id: "default".into(),
            environment_id: "production".into(),
            decision: fake_decision(),
            run_id: None,
            run_event_id: None,
            domain: "customer_support".into(),
        })
        .await
        .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(120)).await;

    let n = trace_count(&pool).await;
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
            workspace_id: "default".into(),
            environment_id: "production".into(),
            decision: fake_decision(),
            run_id: None,
            run_event_id: None,
            domain: "customer_support".into(),
        })
        .await
        .unwrap();
    }

    // Closing the channel forces the final flush even though neither
    // batch nor interval triggered.
    drop(tx);
    handle.await.expect("graceful shutdown");

    let n = trace_count(&pool).await;
    assert_eq!(n, 7);
}
