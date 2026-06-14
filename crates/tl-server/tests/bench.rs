use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{
    BenchArm, BenchRunCreateRequest, BenchRunDetail, BenchRunStatus, ComparedAttackStatus,
    JobStatus, RedteamGenerator, RedteamJobResult, RedteamJobSummary,
};
use tl_engine::Engine;
use tl_server::bench::{build_bench_report, BenchRunArmInput, BenchRunStore, MemoryBenchRunStore};
use tl_server::redteam::DispatchJob;
use tl_server::AppState;
use tl_server::{memory_app_state, router};
use tokio::sync::mpsc;
use tower::ServiceExt;

#[cfg(feature = "postgres")]
#[test]
fn postgres_bench_adapter_implements_bench_store() {
    fn assert_store<T: BenchRunStore>() {}

    assert_store::<tl_server::state::PostgresBenchRunAdapter>();
}

fn request() -> BenchRunCreateRequest {
    BenchRunCreateRequest {
        raw_target_url: "http://127.0.0.1:9101".into(),
        guarded_target_url: "http://127.0.0.1:9102".into(),
        profile: "fast".into(),
        generator: Some(RedteamGenerator::Deterministic),
        agent_id: Some("agent-1".into()),
        seed: Some("seed-1".into()),
    }
}

fn build_app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32])
}

fn build_app_with_worker() -> (axum::Router, mpsc::Receiver<DispatchJob>) {
    let mut state: AppState = memory_app_state(Arc::new(Engine::empty()));
    let (tx, rx) = mpsc::channel(2);
    state.redteam_dispatch_tx = Some(tx);
    (router(state, None, [0u8; 32]), rx)
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn json_request(method: &str, uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn result(seq: i32, case_id: &str, outcome: &str, landed: bool) -> RedteamJobResult {
    RedteamJobResult {
        seq,
        case_id: Some(case_id.into()),
        track: Some("private_data_flow".into()),
        kind: Some("attack".into()),
        trial_index: Some(0),
        attack: "secret extraction".into(),
        goal: "leak secret".into(),
        outcome: outcome.into(),
        landed,
        prompt: Some("prompt".into()),
        reply: "reply".into(),
        trace_id: None,
    }
}

fn job(id: &str, target: &str) -> RedteamJobSummary {
    RedteamJobSummary {
        id: id.into(),
        workspace_id: "ws".into(),
        environment_id: "env".into(),
        status: JobStatus::Complete,
        target: target.into(),
        profile: "fast".into(),
        generator: RedteamGenerator::Deterministic,
        agent_id: Some("agent-1".into()),
        attacks: 1,
        landed: 1,
        blocked: 0,
        error: None,
        created_at: "2026-06-14T00:00:00Z".into(),
        updated_at: "2026-06-14T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn post_bench_run_creates_parent_with_raw_and_guarded_arms() {
    let (app, mut rx) = build_app_with_worker();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/bench/runs",
            &serde_json::json!({
                "raw_target_url": "http://127.0.0.1:9101",
                "guarded_target_url": "http://127.0.0.1:9102",
                "profile": "fast",
                "generator": "deterministic",
                "agent_id": "agent-1",
                "seed": "seed-1"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let detail: BenchRunDetail = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(detail.run.status, BenchRunStatus::Queued);
    assert_eq!(detail.arms.len(), 2);
    assert!(detail.arms.iter().any(|arm| arm.arm == BenchArm::Raw
        && arm.target == "http://127.0.0.1:9101"
        && arm.checker_config.as_deref() == Some("off")
        && arm.redteam_job_id.is_some()));
    assert!(detail.arms.iter().any(|arm| arm.arm == BenchArm::Guarded
        && arm.target == "http://127.0.0.1:9102"
        && arm.checker_config.as_deref() == Some("enforce")
        && arm.redteam_job_id.is_some()));

    let raw_job = rx.try_recv().unwrap();
    let guarded_job = rx.try_recv().unwrap();
    assert_eq!(raw_job.request.target_url, "http://127.0.0.1:9101");
    assert_eq!(guarded_job.request.target_url, "http://127.0.0.1:9102");
    assert_eq!(raw_job.environment_id, detail.run.environment_id);
    assert_eq!(guarded_job.environment_id, detail.run.environment_id);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn post_bench_run_returns_503_without_redteam_worker() {
    let app = build_app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/bench/runs",
            &serde_json::json!({
                "raw_target_url": "http://127.0.0.1:9101",
                "guarded_target_url": "http://127.0.0.1:9102",
                "profile": "fast"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn post_bench_run_rejects_non_loopback_targets() {
    let app = build_app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/bench/runs",
            &serde_json::json!({
                "raw_target_url": "https://evil.example.com",
                "guarded_target_url": "http://127.0.0.1:9102",
                "profile": "fast"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn memory_bench_store_scopes_runs_to_workspace() {
    let store = MemoryBenchRunStore::new();
    let run = store.create("ws", "env", &request()).await.unwrap();

    assert!(store.get("other-ws", &run.id).await.is_err());

    let listed = store.list("ws", 20).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, run.id);
}

#[tokio::test]
async fn memory_bench_store_terminal_status_cannot_be_revived() {
    let store = MemoryBenchRunStore::new();
    let run = store.create("ws", "env", &request()).await.unwrap();

    let cancelled = store.cancel("ws", &run.id).await.unwrap();
    assert_eq!(cancelled.status, BenchRunStatus::Cancelled);

    store
        .set_status("ws", &run.id, BenchRunStatus::Complete, None)
        .await
        .unwrap();
    let after = store.get("ws", &run.id).await.unwrap();
    assert_eq!(after.status, BenchRunStatus::Cancelled);
}

#[tokio::test]
async fn memory_bench_store_attaches_raw_and_guarded_arms() {
    let store = MemoryBenchRunStore::new();
    let run = store.create("ws", "env", &request()).await.unwrap();

    store
        .attach_arm(
            "ws",
            &run.id,
            BenchRunArmInput {
                arm: BenchArm::Raw,
                label: "raw".into(),
                target: "http://127.0.0.1:9101".into(),
                redteam_job_id: Some("raw-job".into()),
                checker_config: Some("off".into()),
            },
        )
        .await
        .unwrap();
    store
        .attach_arm(
            "ws",
            &run.id,
            BenchRunArmInput {
                arm: BenchArm::Guarded,
                label: "guarded".into(),
                target: "http://127.0.0.1:9102".into(),
                redteam_job_id: Some("guarded-job".into()),
                checker_config: Some("enforce".into()),
            },
        )
        .await
        .unwrap();

    let detail = store.get_detail("ws", &run.id).await.unwrap();
    assert_eq!(detail.arms.len(), 2);
    assert!(detail.arms.iter().any(|arm| arm.arm == BenchArm::Raw));
    assert!(detail.arms.iter().any(|arm| arm.arm == BenchArm::Guarded));
}

#[test]
fn bench_report_marks_landed_to_blocked_case_as_fixed() {
    let store = MemoryBenchRunStore::new();
    let run = store.create_blocking("ws", "env", &request()).unwrap();
    let arms = vec![
        store
            .attach_arm_blocking(
                "ws",
                &run.id,
                BenchRunArmInput {
                    arm: BenchArm::Raw,
                    label: "raw".into(),
                    target: "http://127.0.0.1:9101".into(),
                    redteam_job_id: Some("raw-job".into()),
                    checker_config: Some("off".into()),
                },
            )
            .unwrap(),
        store
            .attach_arm_blocking(
                "ws",
                &run.id,
                BenchRunArmInput {
                    arm: BenchArm::Guarded,
                    label: "guarded".into(),
                    target: "http://127.0.0.1:9102".into(),
                    redteam_job_id: Some("guarded-job".into()),
                    checker_config: Some("enforce".into()),
                },
            )
            .unwrap(),
    ];

    let report = build_bench_report(
        &run,
        &arms,
        (
            &job("raw-job", "http://127.0.0.1:9101"),
            &[result(0, "case-1", "landed", true)],
        ),
        (
            &job("guarded-job", "http://127.0.0.1:9102"),
            &[result(0, "case-1", "blocked", false)],
        ),
        "2026-06-14T00:00:00Z",
    );

    assert_eq!(report.raw.attack_success_rate, 1.0);
    assert_eq!(report.guarded.attack_success_rate, 0.0);
    assert_eq!(report.delta.attack_success_rate_reduction, 1.0);
    assert_eq!(report.cases[0].status, ComparedAttackStatus::Fixed);
}
