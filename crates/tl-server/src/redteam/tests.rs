use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use tl_core::{JobStatus, RedteamDispatchRequest, RedteamGenerator};
use tokio::sync::mpsc;

use super::handlers::dispatch_job;
use super::orchestrator::run_dispatch;
use super::runner_client::{
    RedteamRunner, RedteamRunnerClient, RunnerAttack, RunnerDispatch, RunnerError, RunnerHandle,
    RunnerReport, RunnerStatus,
};
use super::validation::validate_dispatch;
use super::{
    DispatchConfig, DispatchJob, JobCounts, MemoryRedteamJobStore, RedteamJobListFilter,
    RedteamJobStore, RedteamState,
};
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
