use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::{
    body::to_bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    Json,
};
use tl_core::{
    ComparedAttackStatus, CreateReportRequest, JobStatus, RedteamDispatchRequest, RedteamGenerator,
    RedteamJobResult, RedteamReportPayload, RedteamReportShare, ReportSeverity,
};
use tokio::sync::mpsc;

use super::handlers::{create_report, dispatch_job, get_public_report, get_report, revoke_report};
use super::orchestrator::run_dispatch;
use super::runner_client::{
    RedteamRunner, RedteamRunnerClient, RunnerAttack, RunnerDispatch, RunnerError, RunnerHandle,
    RunnerReport, RunnerStatus,
};
use super::validation::validate_dispatch;
use super::{
    DispatchConfig, DispatchJob, JobCounts, MemoryRedteamJobStore, MemoryRedteamReportShareStore,
    RedteamAttackRecordFilter, RedteamJobListFilter, RedteamJobStore, RedteamState,
};
use super::{PublicReportState, ReportRateLimiter};
use crate::environments::MemoryEnvironmentStore;

// ---- helpers -------------------------------------------------------------

fn dispatch_req() -> RedteamDispatchRequest {
    RedteamDispatchRequest {
        target_url: "http://127.0.0.1:9102".into(),
        profile: "fast".into(),
        generator: None,
        agent_id: Some("agent-1".into()),
    }
}

fn req_with(target: &str, profile: &str) -> RedteamDispatchRequest {
    RedteamDispatchRequest {
        target_url: target.into(),
        profile: profile.into(),
        generator: None,
        agent_id: None,
    }
}

fn fast_config() -> DispatchConfig {
    DispatchConfig {
        channel_capacity: 8,
        max_concurrent: 1,
        poll_interval: Duration::from_millis(1),
        max_duration: Duration::from_secs(5),
    }
}

fn dispatch_message(job_id: &str) -> DispatchJob {
    DispatchJob {
        workspace_id: "ws".into(),
        environment_id: "env".into(),
        job_id: job_id.into(),
        request: dispatch_req(),
    }
}

/// Fake runner with canned poll output. Optionally cancels the job via a
/// shared store handle on first poll to exercise the cooperative cancel path.
struct FakeRunner {
    report: RunnerReport,
    fail_dispatch: bool,
    cancel_on_poll: Option<(Arc<MemoryRedteamJobStore>, String)>,
}

impl FakeRunner {
    fn returning(status: RunnerStatus, attacks: Vec<RunnerAttack>, error: Option<&str>) -> Self {
        Self {
            report: RunnerReport {
                status,
                attacks,
                error: error.map(str::to_string),
            },
            fail_dispatch: false,
            cancel_on_poll: None,
        }
    }
}

#[async_trait]
impl RedteamRunner for FakeRunner {
    async fn dispatch(&self, _request: &RunnerDispatch) -> Result<RunnerHandle, RunnerError> {
        if self.fail_dispatch {
            return Err(RunnerError::Transport("runner down".into()));
        }
        Ok(RunnerHandle {
            job_id: "runner-job-1".into(),
        })
    }

    async fn poll(&self, _runner_job_id: &str) -> Result<RunnerReport, RunnerError> {
        if let Some((store, job_id)) = &self.cancel_on_poll {
            let _ = store.cancel("ws", job_id).await;
        }
        Ok(self.report.clone())
    }
}

fn attack(name: &str, outcome: &str, landed: bool) -> RunnerAttack {
    RunnerAttack {
        case_id: None,
        track: None,
        kind: None,
        trial_index: None,
        attack: name.into(),
        goal: "exfiltrate".into(),
        outcome: outcome.into(),
        landed,
        prompt: Some("prompt".into()),
        reply: "reply".into(),
        trace_id: None,
    }
}

fn result(seq: i32, name: &str, outcome: &str, landed: bool) -> RedteamJobResult {
    RedteamJobResult {
        seq,
        case_id: None,
        track: None,
        kind: None,
        trial_index: None,
        attack: name.into(),
        goal: "exfiltrate".into(),
        outcome: outcome.into(),
        landed,
        prompt: Some("prompt".into()),
        reply: "reply".into(),
        trace_id: None,
    }
}

// ---- validation ----------------------------------------------------------

#[test]
fn validate_dispatch_rules() {
    assert!(validate_dispatch(&req_with("", "fast")).is_err());
    assert!(validate_dispatch(&req_with("   ", "fast")).is_err());
    assert!(validate_dispatch(&req_with("ftp://127.0.0.1", "fast")).is_err()); // bad scheme
    assert!(validate_dispatch(&req_with("http://127.0.0.1:9102", "turbo")).is_err()); // bad profile
    assert!(validate_dispatch(&req_with("http://127.0.0.1:9102", "fast")).is_ok());
    assert!(validate_dispatch(&req_with("https://localhost/agent", "max")).is_ok());
    assert!(validate_dispatch(&req_with("http://[::1]:9102", "full")).is_ok());
}

#[test]
fn validate_dispatch_rejects_non_loopback_targets() {
    // SSRF guard: the orchestrator must not be talked into fetching arbitrary
    // hosts, including cloud metadata, even by a direct (non-web) API caller.
    assert!(validate_dispatch(&req_with("https://evil.example.com", "fast")).is_err());
    assert!(
        validate_dispatch(&req_with("http://169.254.169.254/latest/meta-data", "fast")).is_err()
    );
    assert!(validate_dispatch(&req_with("http://10.0.0.5:9102", "fast")).is_err());
    assert!(validate_dispatch(&req_with("not-a-url", "fast")).is_err());
}

// ---- runner client -------------------------------------------------------

#[test]
fn runner_client_rejects_malformed_url() {
    // A bad REDTEAM_RUNNER_URL must fail init so dispatch stays disabled (503)
    // rather than spawning a worker that can never reach a runner.
    assert!(RedteamRunnerClient::new("not a url").is_err());
    assert!(RedteamRunnerClient::new("ftp://runner:8799").is_err());
    assert!(RedteamRunnerClient::new("http://127.0.0.1:8799").is_ok());
    assert!(RedteamRunnerClient::new("https://runner.internal/").is_ok());
}

// ---- memory store --------------------------------------------------------

#[tokio::test]
async fn memory_store_create_starts_queued() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    assert_eq!(job.status, JobStatus::Queued);
    assert_eq!(job.generator, RedteamGenerator::Deterministic);
    assert_eq!(job.attacks, 0);
    assert_eq!(store.get("ws", &job.id).await.unwrap().id, job.id);
}

#[tokio::test]
async fn memory_store_get_scopes_to_workspace() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    assert!(store.get("other-ws", &job.id).await.is_err());
}

#[tokio::test]
async fn memory_store_list_filters_by_agent_and_orders_desc() {
    let store = MemoryRedteamJobStore::new();
    let mut tagged = dispatch_req();
    tagged.agent_id = Some("keep".into());
    let first = store.create("ws", "env", &tagged).await.unwrap();
    let second = store.create("ws", "env", &tagged).await.unwrap();
    let _other = store
        .create("ws", "env", &req_with("https://x", "fast"))
        .await
        .unwrap();

    let filtered = store
        .list(
            "ws",
            RedteamJobListFilter {
                agent_id: Some("keep".into()),
                limit: 20,
            },
        )
        .await
        .unwrap();
    assert_eq!(filtered.len(), 2);
    // uuidv7 ids sort by creation; newest first.
    assert_eq!(filtered[0].id, second.id);
    assert_eq!(filtered[1].id, first.id);
}

#[tokio::test]
async fn memory_store_list_attack_records_joins_filters_and_orders() {
    let store = MemoryRedteamJobStore::new();
    // Older job (A) and newer job (B) in the same workspace, plus a job in another
    // workspace that must never leak in.
    let older = store
        .create("ws", "env", &req_with("http://127.0.0.1:9101", "fast"))
        .await
        .unwrap();
    let newer = store
        .create("ws", "env", &req_with("http://127.0.0.1:9102", "fast"))
        .await
        .unwrap();
    let foreign = store
        .create(
            "other-ws",
            "env",
            &req_with("http://127.0.0.1:9103", "fast"),
        )
        .await
        .unwrap();

    store
        .record_result("ws", &older.id, &result(0, "audit-dump", "landed", true))
        .await
        .unwrap();
    store
        .record_result(
            "ws",
            &older.id,
            &result(1, "role-confuse", "blocked", false),
        )
        .await
        .unwrap();
    store
        .record_result("ws", &newer.id, &result(0, "audit-dump", "clean", false))
        .await
        .unwrap();
    store
        .record_result(
            "other-ws",
            &foreign.id,
            &result(0, "audit-dump", "landed", true),
        )
        .await
        .unwrap();

    let unfiltered = |limit| RedteamAttackRecordFilter {
        attack: None,
        outcome: None,
        limit,
    };

    // All records in the workspace, newest job first; foreign workspace excluded.
    let all = store
        .list_attack_records("ws", unfiltered(50))
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].job_id, newer.id, "newest job's records come first");
    assert_eq!(all[0].target, "http://127.0.0.1:9102");
    assert!(all.iter().all(|r| r.job_id != foreign.id));

    // Filter by attack name.
    let by_attack = store
        .list_attack_records(
            "ws",
            RedteamAttackRecordFilter {
                attack: Some("audit-dump".into()),
                outcome: None,
                limit: 50,
            },
        )
        .await
        .unwrap();
    assert_eq!(by_attack.len(), 2);
    assert!(by_attack.iter().all(|r| r.attack == "audit-dump"));

    // Filter by outcome.
    let by_outcome = store
        .list_attack_records(
            "ws",
            RedteamAttackRecordFilter {
                attack: None,
                outcome: Some("landed".into()),
                limit: 50,
            },
        )
        .await
        .unwrap();
    assert_eq!(by_outcome.len(), 1);
    assert_eq!(by_outcome[0].job_id, older.id);
    assert!(by_outcome[0].landed);

    // Limit truncates after ordering.
    let limited = store
        .list_attack_records("ws", unfiltered(1))
        .await
        .unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].job_id, newer.id);
}

#[tokio::test]
async fn memory_store_cancel_transitions_then_no_ops() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();

    let cancelled = store.cancel("ws", &job.id).await.unwrap();
    assert_eq!(cancelled.status, JobStatus::Cancelled);

    // Cancelling a terminal job is a no-op that returns the job unchanged.
    let again = store.cancel("ws", &job.id).await.unwrap();
    assert_eq!(again.status, JobStatus::Cancelled);
}

#[tokio::test]
async fn set_status_cannot_revive_a_terminal_job() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    store.cancel("ws", &job.id).await.unwrap(); // -> Cancelled (terminal)

    // A late completion write (the cancel-vs-complete race) must not revive the
    // cancelled job or apply its counts.
    store
        .set_status(
            "ws",
            &job.id,
            JobStatus::Complete,
            Some(JobCounts {
                attacks: 3,
                landed: 2,
                blocked: 1,
            }),
            None,
        )
        .await
        .unwrap();

    let updated = store.get("ws", &job.id).await.unwrap();
    assert_eq!(updated.status, JobStatus::Cancelled);
    assert_eq!(updated.attacks, 0);
}

// ---- orchestrator --------------------------------------------------------

#[tokio::test]
async fn orchestrator_completes_and_persists_results() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    let runner = FakeRunner::returning(
        RunnerStatus::Complete,
        vec![
            attack("a1", "landed", true),
            attack("a2", "blocked", false),
            attack("a3", "clean", false),
        ],
        None,
    );

    run_dispatch(&runner, &store, &fast_config(), dispatch_message(&job.id)).await;

    let updated = store.get("ws", &job.id).await.unwrap();
    assert_eq!(updated.status, JobStatus::Complete);
    assert_eq!(updated.attacks, 3);
    assert_eq!(updated.landed, 1);
    assert_eq!(updated.blocked, 1);

    let results = store.list_results("ws", &job.id).await.unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].seq, 0);
    assert_eq!(results[0].attack, "a1");
    assert!(results[0].landed);
}

#[tokio::test]
async fn orchestrator_marks_error_when_runner_reports_failure() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    let runner = FakeRunner::returning(RunnerStatus::Error, vec![], Some("engine crashed"));

    run_dispatch(&runner, &store, &fast_config(), dispatch_message(&job.id)).await;

    let updated = store.get("ws", &job.id).await.unwrap();
    assert_eq!(updated.status, JobStatus::Error);
    assert_eq!(updated.error.as_deref(), Some("engine crashed"));
}

#[tokio::test]
async fn orchestrator_marks_error_when_dispatch_fails() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    let mut runner = FakeRunner::returning(RunnerStatus::Complete, vec![], None);
    runner.fail_dispatch = true;

    run_dispatch(&runner, &store, &fast_config(), dispatch_message(&job.id)).await;

    let updated = store.get("ws", &job.id).await.unwrap();
    assert_eq!(updated.status, JobStatus::Error);
    assert!(updated.error.unwrap().contains("runner down"));
}

#[tokio::test]
async fn orchestrator_times_out_when_runner_never_completes() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    let runner = FakeRunner::returning(RunnerStatus::Running, vec![], None);
    let config = DispatchConfig {
        max_duration: Duration::from_millis(0),
        ..fast_config()
    };

    run_dispatch(&runner, &store, &config, dispatch_message(&job.id)).await;

    let updated = store.get("ws", &job.id).await.unwrap();
    assert_eq!(updated.status, JobStatus::Error);
    assert!(updated.error.unwrap().contains("timed out"));
}

#[tokio::test]
async fn orchestrator_skips_job_cancelled_before_pickup() {
    let store = MemoryRedteamJobStore::new();
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    store.cancel("ws", &job.id).await.unwrap();
    let runner = FakeRunner::returning(
        RunnerStatus::Complete,
        vec![attack("a1", "landed", true)],
        None,
    );

    run_dispatch(&runner, &store, &fast_config(), dispatch_message(&job.id)).await;

    let updated = store.get("ws", &job.id).await.unwrap();
    assert_eq!(updated.status, JobStatus::Cancelled);
    assert!(store.list_results("ws", &job.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn orchestrator_stops_when_cancelled_mid_poll() {
    let store = Arc::new(MemoryRedteamJobStore::new());
    let job = store.create("ws", "env", &dispatch_req()).await.unwrap();
    let runner = FakeRunner {
        report: RunnerReport {
            status: RunnerStatus::Running,
            attacks: vec![],
            error: None,
        },
        fail_dispatch: false,
        cancel_on_poll: Some((store.clone(), job.id.clone())),
    };

    run_dispatch(
        &runner,
        store.as_ref(),
        &fast_config(),
        dispatch_message(&job.id),
    )
    .await;

    let updated = store.get("ws", &job.id).await.unwrap();
    assert_eq!(updated.status, JobStatus::Cancelled);
}

// ---- dispatch handler ----------------------------------------------------

#[tokio::test]
async fn dispatch_returns_201_and_queues_job() {
    let (tx, mut rx) = mpsc::channel(4);
    let state = RedteamState {
        store: Arc::new(MemoryRedteamJobStore::new()),
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        report_share_store: Arc::new(MemoryRedteamReportShareStore::new()),
        dispatch_tx: Some(tx),
    };

    let response = dispatch_job(State(state), HeaderMap::new(), Json(dispatch_req())).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let queued = rx.try_recv().expect("job should be queued");
    assert!(!queued.job_id.is_empty());
}

#[tokio::test]
async fn dispatch_returns_503_when_worker_disabled() {
    let state = RedteamState {
        store: Arc::new(MemoryRedteamJobStore::new()),
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        report_share_store: Arc::new(MemoryRedteamReportShareStore::new()),
        dispatch_tx: None,
    };

    let response = dispatch_job(State(state), HeaderMap::new(), Json(dispatch_req())).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn dispatch_rejects_invalid_target() {
    let (tx, _rx) = mpsc::channel(4);
    let state = RedteamState {
        store: Arc::new(MemoryRedteamJobStore::new()),
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        report_share_store: Arc::new(MemoryRedteamReportShareStore::new()),
        dispatch_tx: Some(tx),
    };

    let response = dispatch_job(
        State(state),
        HeaderMap::new(),
        Json(req_with("not-a-url", "fast")),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ---- report handler ------------------------------------------------------

fn report_state(store: Arc<MemoryRedteamJobStore>) -> RedteamState {
    RedteamState {
        store,
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        report_share_store: Arc::new(MemoryRedteamReportShareStore::new()),
        dispatch_tx: None,
    }
}

fn report_result(
    seq: i32,
    attack: &str,
    goal: &str,
    outcome: &str,
    landed: bool,
) -> RedteamJobResult {
    RedteamJobResult {
        seq,
        case_id: None,
        track: None,
        kind: None,
        trial_index: None,
        attack: attack.into(),
        goal: goal.into(),
        outcome: outcome.into(),
        landed,
        prompt: Some("prompt".into()),
        reply: "the credential is sk-trustloop9f3k2x".into(),
        trace_id: Some("trace-1".into()),
    }
}

/// Seed a `Complete` job with results and return its id. Jobs are stored under
/// the same workspace the handler resolves from empty headers, so the report
/// handler can read them back.
async fn seed_job(
    store: &MemoryRedteamJobStore,
    agent_id: Option<&str>,
    results: &[RedteamJobResult],
) -> String {
    let workspace_id = crate::policies::workspace_id_from_headers(&HeaderMap::new());
    let request = RedteamDispatchRequest {
        target_url: "http://127.0.0.1:9101".into(),
        profile: "fast".into(),
        generator: None,
        agent_id: agent_id.map(str::to_string),
    };
    let job = store.create(&workspace_id, "env", &request).await.unwrap();
    for result in results {
        store
            .record_result(&workspace_id, &job.id, result)
            .await
            .unwrap();
    }
    store
        .set_status(
            &workspace_id,
            &job.id,
            JobStatus::Complete,
            Some(JobCounts::default()),
            None,
        )
        .await
        .unwrap();
    job.id
}

async fn report_body(response: axum::response::Response) -> RedteamReportPayload {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).expect("report payload deserializes")
}

#[tokio::test]
async fn get_report_returns_payload_with_severity() {
    let store = Arc::new(MemoryRedteamJobStore::new());
    let id = seed_job(
        &store,
        Some("agent-1"),
        &[
            report_result(
                0,
                "secret extraction",
                "leak the api credential",
                "landed",
                true,
            ),
            report_result(1, "control", "benign question", "clean", false),
        ],
    )
    .await;

    let uri: Uri = format!("/v1/redteam/jobs/{id}/report").parse().unwrap();
    let response = get_report(
        State(report_state(store)),
        HeaderMap::new(),
        Path(id.clone()),
        uri,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let payload = report_body(response).await;
    assert_eq!(payload.findings.len(), 2);
    assert_eq!(payload.aggregates.risk_level, ReportSeverity::Critical);
    assert!(payload.comparison.is_none());
}

#[tokio::test]
async fn get_report_404_for_unknown_job() {
    let store = Arc::new(MemoryRedteamJobStore::new());
    let uri: Uri = "/v1/redteam/jobs/missing/report".parse().unwrap();
    let response = get_report(
        State(report_state(store)),
        HeaderMap::new(),
        Path("missing".into()),
        uri,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_report_compares_same_agent_runs() {
    let store = Arc::new(MemoryRedteamJobStore::new());
    let baseline = seed_job(
        &store,
        Some("agent-1"),
        &[report_result(
            0,
            "secret extraction",
            "leak secret",
            "landed",
            true,
        )],
    )
    .await;
    let hardened = seed_job(
        &store,
        Some("agent-1"),
        &[report_result(
            0,
            "secret extraction",
            "leak secret",
            "blocked",
            false,
        )],
    )
    .await;

    let uri: Uri = format!("/v1/redteam/jobs/{baseline}/report?compare={hardened}")
        .parse()
        .unwrap();
    let response = get_report(
        State(report_state(store)),
        HeaderMap::new(),
        Path(baseline.clone()),
        uri,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let payload = report_body(response).await;
    let comparison = payload.comparison.expect("comparison present");
    assert_eq!(comparison.attacks[0].status, ComparedAttackStatus::Fixed);
    assert!((comparison.delta_points - 100.0).abs() < 1e-9);
}

#[tokio::test]
async fn get_report_rejects_compare_from_different_agent() {
    let store = Arc::new(MemoryRedteamJobStore::new());
    let a = seed_job(
        &store,
        Some("agent-1"),
        &[report_result(0, "x", "leak secret", "landed", true)],
    )
    .await;
    let b = seed_job(
        &store,
        Some("agent-2"),
        &[report_result(0, "x", "leak secret", "landed", true)],
    )
    .await;

    let uri: Uri = format!("/v1/redteam/jobs/{a}/report?compare={b}")
        .parse()
        .unwrap();
    let response = get_report(
        State(report_state(store)),
        HeaderMap::new(),
        Path(a.clone()),
        uri,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ---- report share endpoints ----------------------------------------------

/// A mint state and a public-read state that share the same job + share stores,
/// so a token minted via `create_report` resolves via `get_public_report`.
fn share_states() -> (RedteamState, PublicReportState, Arc<MemoryRedteamJobStore>) {
    let job_store = Arc::new(MemoryRedteamJobStore::new());
    let share_store = Arc::new(MemoryRedteamReportShareStore::new());
    let redteam = RedteamState {
        store: job_store.clone(),
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        report_share_store: share_store.clone(),
        dispatch_tx: None,
    };
    let public = PublicReportState {
        store: job_store.clone(),
        report_share_store: share_store,
        // Permissive in tests that aren't exercising the limit.
        rate_limiter: Arc::new(ReportRateLimiter::new(Duration::from_secs(60), 1000)),
    };
    (redteam, public, job_store)
}

async fn share_body(response: axum::response::Response) -> RedteamReportShare {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).expect("share deserializes")
}

#[tokio::test]
async fn create_report_mints_share_for_complete_job() {
    let (redteam, _public, job_store) = share_states();
    let id = seed_job(
        &job_store,
        Some("agent-1"),
        &[report_result(
            0,
            "secret extraction",
            "leak secret",
            "landed",
            true,
        )],
    )
    .await;

    let response = create_report(
        State(redteam),
        HeaderMap::new(),
        axum::Json(CreateReportRequest {
            job_id: id.clone(),
            compare_job_id: None,
            ttl_days: None,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let share = share_body(response).await;
    assert!(share.token.starts_with("rpt_"));
    assert_eq!(share.path, format!("/r/{}", share.token));
    assert!(share.expires_at.is_some());
}

#[tokio::test]
async fn create_report_rejects_incomplete_job() {
    let (redteam, _public, job_store) = share_states();
    // A freshly created job is Queued, not Complete.
    let workspace_id = crate::policies::workspace_id_from_headers(&HeaderMap::new());
    let job = job_store
        .create(&workspace_id, "env", &dispatch_req())
        .await
        .unwrap();

    let response = create_report(
        State(redteam),
        HeaderMap::new(),
        axum::Json(CreateReportRequest {
            job_id: job.id,
            compare_job_id: None,
            ttl_days: None,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn public_report_reads_then_404s_after_revoke() {
    let (redteam, public, job_store) = share_states();
    let id = seed_job(
        &job_store,
        Some("agent-1"),
        &[report_result(
            0,
            "secret extraction",
            "leak secret",
            "landed",
            true,
        )],
    )
    .await;

    // Mint.
    let minted = create_report(
        State(redteam.clone()),
        HeaderMap::new(),
        axum::Json(CreateReportRequest {
            job_id: id.clone(),
            compare_job_id: None,
            ttl_days: Some(7),
        }),
    )
    .await;
    let share = share_body(minted).await;

    // Public read succeeds, unauthenticated.
    let read = get_public_report(State(public.clone()), Path(share.token.clone())).await;
    assert_eq!(read.status(), StatusCode::OK);
    let payload = report_body(read).await;
    assert_eq!(payload.aggregates.risk_level, ReportSeverity::Critical);

    // Revoke, then the same token 404s.
    let revoked = revoke_report(State(redteam), HeaderMap::new(), Path(share.token.clone())).await;
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);

    let after = get_public_report(State(public), Path(share.token)).await;
    assert_eq!(after.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn public_report_404_for_unknown_token() {
    let (_redteam, public, _job_store) = share_states();
    let response = get_public_report(State(public), Path("rpt_missing".into())).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn public_report_429s_over_the_per_token_limit() {
    let job_store = Arc::new(MemoryRedteamJobStore::new());
    let share_store = Arc::new(MemoryRedteamReportShareStore::new());
    let redteam = RedteamState {
        store: job_store.clone(),
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        report_share_store: share_store.clone(),
        dispatch_tx: None,
    };
    let public = PublicReportState {
        store: job_store.clone(),
        report_share_store: share_store,
        rate_limiter: Arc::new(ReportRateLimiter::new(Duration::from_secs(60), 1)),
    };
    let id = seed_job(
        &job_store,
        Some("agent-1"),
        &[report_result(0, "x", "leak secret", "landed", true)],
    )
    .await;
    let minted = create_report(
        State(redteam),
        HeaderMap::new(),
        axum::Json(CreateReportRequest {
            job_id: id,
            compare_job_id: None,
            ttl_days: None,
        }),
    )
    .await;
    let share = share_body(minted).await;

    let first = get_public_report(State(public.clone()), Path(share.token.clone())).await;
    assert_eq!(first.status(), StatusCode::OK);
    // Second read of the same link exceeds max=1 → 429.
    let second = get_public_report(State(public), Path(share.token)).await;
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn create_report_rejects_self_comparison() {
    let (redteam, _public, job_store) = share_states();
    let id = seed_job(
        &job_store,
        Some("agent-1"),
        &[report_result(0, "x", "leak secret", "landed", true)],
    )
    .await;
    let response = create_report(
        State(redteam),
        HeaderMap::new(),
        axum::Json(CreateReportRequest {
            job_id: id.clone(),
            compare_job_id: Some(id),
            ttl_days: None,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
