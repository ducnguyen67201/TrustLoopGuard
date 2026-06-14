use tl_core::{
    BenchArm, BenchRunCreateRequest, BenchRunStatus, ComparedAttackStatus, JobStatus,
    RedteamGenerator, RedteamJobResult, RedteamJobSummary,
};
use tl_server::bench::{build_bench_report, BenchRunArmInput, BenchRunStore, MemoryBenchRunStore};

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
