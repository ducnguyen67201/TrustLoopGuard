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
use tl_core::{
    new_trace_id, Action, CheckerFindingEvidence, CheckerRun, Confidentiality, Decision,
    EnforcementMode, EventKind, GuardEvent, Integrity, LabelBasis, LabelBasisSet,
    LabelPolicyStatus, LabelResolution, Labels, Principal, ProvenanceMap, SideEffectClass,
    SourceLabelEvidence, Trust, Verdict,
};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, traces, workspace_environments, workspaces},
    spawn_writer, DbPool, TraceWrite, WriterConfig,
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
    // Traces carry a workspace/environment foreign key; seed the rows
    // the tests write against.
    {
        let mut conn = pool.get().await.expect("connection");
        diesel::insert_into(organizations::table)
            .values((
                organizations::id.eq("org_default"),
                organizations::name.eq("Default Org"),
                organizations::slug.eq("default-org"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert organization");
        diesel::insert_into(workspaces::table)
            .values((
                workspaces::id.eq("default"),
                workspaces::organization_id.eq("org_default"),
                workspaces::name.eq("Default Workspace"),
                workspaces::slug.eq("default"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert workspace");
        diesel::insert_into(workspace_environments::table)
            .values((
                workspace_environments::workspace_id.eq("default"),
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
            event: None,
            run_id: None,
            run_event_id: None,
            session_id: None,
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
            event: None,
            run_id: None,
            run_event_id: None,
            session_id: None,
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
            event: None,
            run_id: None,
            run_event_id: None,
            session_id: None,
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
async fn event_evidence_round_trips_in_payload() {
    let (pool, _c) = fresh_pool().await;
    let (tx, handle) = spawn_writer(pool.clone(), WriterConfig::default());

    let event = GuardEvent {
        kind: EventKind::OutputProposed,
        principal: Principal {
            workspace_id: "default".into(),
            environment_id: "production".into(),
            agent_id: "agent-1".into(),
            user_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            run_event_id: None,
        },
        action: Action {
            operation: "output".into(),
            parameters: serde_json::json!({ "text": "safe reply" }),
            side_effect: Some(SideEffectClass::None),
        },
        sources: vec![],
        provenance: ProvenanceMap::default(),
        resolution: None,
        label_resolution: Some(LabelResolution {
            policy_status: LabelPolicyStatus::NotConfigured,
            sources: vec![SourceLabelEvidence {
                source_id: "legacy.input".into(),
                labels: Labels {
                    trust: Trust::Trusted,
                    confidentiality: Confidentiality::Private,
                    integrity: Integrity::High,
                },
                basis: LabelBasisSet {
                    trust: LabelBasis::OriginDefault,
                    confidentiality: LabelBasis::OriginDefault,
                    integrity: LabelBasis::OriginDefault,
                },
            }],
            derived: [(
                "text".to_string(),
                Labels {
                    trust: Trust::Trusted,
                    confidentiality: Confidentiality::Private,
                    integrity: Integrity::High,
                },
            )]
            .into_iter()
            .collect(),
        }),
        checks: vec![CheckerRun {
            checker_id: "information_flow".into(),
            mode: EnforcementMode::Shadow,
            findings: vec![CheckerFindingEvidence {
                rule: "action-integrity".into(),
                reason: "high-impact action is controlled by untrusted context".into(),
                recommended_verdict: Some(Verdict::Block),
                source_chain: vec!["src.web".into()],
                risk_source: Some("web".into()),
                failure_mode: Some("untrusted_control".into()),
                harm_class: Some("integrity".into()),
                action_feedback: vec![],
            }],
        }],
        signals: vec![],
        context: serde_json::Value::Null,
    };
    tx.send(TraceWrite {
        workspace_id: "default".into(),
        environment_id: "production".into(),
        decision: fake_decision(),
        event: Some(event),
        run_id: None,
        run_event_id: None,
        session_id: None,
        domain: "customer_support".into(),
    })
    .await
    .expect("send");

    drop(tx);
    handle.await.expect("writer task");

    let mut conn = pool.get().await.expect("connection");
    let payload = traces::table
        .select(traces::payload)
        .first::<serde_json::Value>(&mut conn)
        .await
        .expect("payload");

    assert_eq!(payload["event"]["kind"], "output.proposed");
    assert_eq!(payload["event"]["principal"]["agent_id"], "agent-1");
    // Label resolution evidence rides along in the event object.
    let label_resolution = &payload["event"]["label_resolution"];
    assert_eq!(label_resolution["policy_status"], "not_configured");
    assert_eq!(label_resolution["sources"][0]["source_id"], "legacy.input");
    assert_eq!(label_resolution["sources"][0]["labels"]["trust"], "trusted");
    assert_eq!(
        label_resolution["sources"][0]["basis"]["trust"],
        "origin_default"
    );
    assert_eq!(label_resolution["derived"]["text"]["trust"], "trusted");
    // Checker evidence rides along too, with the full shadow hypothetical.
    let checks = &payload["event"]["checks"];
    assert_eq!(checks[0]["checker_id"], "information_flow");
    assert_eq!(checks[0]["mode"], "shadow");
    assert_eq!(checks[0]["findings"][0]["rule"], "action-integrity");
    assert_eq!(checks[0]["findings"][0]["recommended_verdict"], "block");
    // Enriched payload still parses as a Decision for existing readers.
    let parsed: Decision = serde_json::from_value(payload).expect("decision parse");
    assert_eq!(parsed.verdict, Verdict::Allow);
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
            event: None,
            run_id: None,
            run_event_id: None,
            session_id: None,
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

#[tokio::test]
async fn session_id_round_trips_and_filters_trace_lists() {
    let (pool, _c) = fresh_pool().await;
    let (tx, handle) = spawn_writer(pool.clone(), WriterConfig::default());

    for session in [Some("sess_a"), Some("sess_a"), Some("sess_b"), None] {
        tx.send(TraceWrite {
            workspace_id: "default".into(),
            environment_id: "production".into(),
            decision: fake_decision(),
            event: None,
            run_id: None,
            run_event_id: None,
            session_id: session.map(str::to_string),
            domain: "customer_support".into(),
        })
        .await
        .expect("send");
    }

    drop(tx);
    handle.await.expect("writer task");

    // The column round-trips verbatim.
    let mut conn = pool.get().await.expect("connection");
    let tagged: i64 = traces::table
        .filter(traces::session_id.eq("sess_a"))
        .select(count_star())
        .first(&mut conn)
        .await
        .expect("count");
    assert_eq!(tagged, 2, "session column did not persist");

    // The repo filter isolates one session; no filter returns everything.
    let repo = tl_storage::TraceRepo::new(pool.clone());
    let sess_a = repo
        .list_recent("default", "production", Some("sess_a"), 50)
        .await
        .expect("filtered list");
    assert_eq!(sess_a.len(), 2);
    assert!(sess_a
        .iter()
        .all(|row| row.session_id.as_deref() == Some("sess_a")));

    let all = repo
        .list_recent("default", "production", None, 50)
        .await
        .expect("unfiltered list");
    assert_eq!(all.len(), 4);
}
