//! In-process dispatch worker.
//!
//! `dispatch_job` drops a `DispatchJob` into an mpsc channel and returns
//! immediately. This worker drains the channel, runs each job through the
//! runner under a concurrency cap, persists per-attack sessions, and writes the
//! final status. It never panics: any
//! failure marks the job `Error`.

use std::sync::Arc;
use std::time::Duration;

use tl_core::{JobStatus, RedteamAttackSession, RedteamDispatchRequest, RedteamSessionEvent};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use super::runner_client::{
    RedteamRunner, RunnerAttackSession, RunnerAttackSurface, RunnerAttackVector, RunnerDispatch,
    RunnerDocumentTemplate, RunnerRunMode, RunnerStatus,
};
use super::{is_terminal, JobCounts, RedteamJobStore};

/// One queued dispatch. The handler fills this in after persisting the
/// job as `Queued`. Public because it crosses the `RedteamState` boundary.
#[derive(Debug, Clone)]
pub struct DispatchJob {
    pub workspace_id: String,
    pub environment_id: String,
    pub job_id: String,
    pub request: RedteamDispatchRequest,
}

#[derive(Debug, Clone)]
pub(crate) struct DispatchConfig {
    pub channel_capacity: usize,
    /// Max jobs executing at once. Runner jobs can be expensive, so this stays
    /// small; excess jobs wait in the channel.
    pub max_concurrent: usize,
    pub poll_interval: Duration,
    pub max_duration: Duration,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 256,
            max_concurrent: 2,
            poll_interval: Duration::from_secs(2),
            max_duration: Duration::from_secs(600),
        }
    }
}

/// Spawn the worker. Returns the sender to hang on `AppState`. The
/// `JoinHandle` is dropped here — the worker lives for the process; the
/// channel closing on shutdown ends the loop.
///
// TODO(mq): replace this in-process channel with a durable queue and
// requeue `Queued`/`Running` jobs on boot. `JobStatus` is already
// persisted, so requeue-on-boot is a clean future add.
pub(crate) fn spawn_dispatch_worker(
    runner: Arc<dyn RedteamRunner>,
    store: Arc<dyn RedteamJobStore>,
    config: DispatchConfig,
) -> (mpsc::Sender<DispatchJob>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(config.channel_capacity);
    let handle = tokio::spawn(worker_loop(runner, store, config, rx));
    (tx, handle)
}

async fn worker_loop(
    runner: Arc<dyn RedteamRunner>,
    store: Arc<dyn RedteamJobStore>,
    config: DispatchConfig,
    mut rx: mpsc::Receiver<DispatchJob>,
) {
    let limiter = Arc::new(Semaphore::new(config.max_concurrent));
    while let Some(job) = rx.recv().await {
        // Acquire before spawning so the channel applies backpressure
        // when at capacity instead of spawning unbounded tasks. The local
        // semaphore is never closed, so this error path is unreachable in
        // practice; log loudly rather than drop queued jobs silently if it
        // ever does fire.
        let Ok(permit) = limiter.clone().acquire_owned().await else {
            tracing::error!("redteam: dispatch semaphore closed unexpectedly; worker stopping");
            break;
        };
        let runner = runner.clone();
        let store = store.clone();
        let config = config.clone();
        tokio::spawn(async move {
            let _permit = permit;
            run_dispatch(runner.as_ref(), store.as_ref(), &config, job).await;
        });
    }
}

/// Final disposition of a single dispatch.
enum DispatchOutcome {
    Completed(JobCounts),
    /// The job was cancelled mid-flight; status is left as set by `cancel`.
    Cancelled,
    Failed(String),
}

pub(crate) async fn run_dispatch(
    runner: &dyn RedteamRunner,
    store: &dyn RedteamJobStore,
    config: &DispatchConfig,
    job: DispatchJob,
) {
    // A job cancelled (or otherwise finalized) while it sat in the queue
    // must not be revived into `Running`.
    match store.get(&job.workspace_id, &job.job_id).await {
        Ok(summary) if is_terminal(summary.status) => {
            tracing::info!(job_id = %job.job_id, "redteam: skipping already-finalized job");
            return;
        }
        Ok(_) => {}
        Err(e) => {
            tracing::error!(job_id = %job.job_id, error = %e, "redteam: cannot load queued job");
            return;
        }
    }

    if let Err(e) = store
        .set_status(
            &job.workspace_id,
            &job.job_id,
            JobStatus::Running,
            None,
            None,
        )
        .await
    {
        tracing::error!(job_id = %job.job_id, error = %e, "redteam: cannot mark job running");
        return;
    }

    let result = match drive(runner, store, config, &job).await {
        DispatchOutcome::Completed(counts) => {
            store
                .set_status(
                    &job.workspace_id,
                    &job.job_id,
                    JobStatus::Complete,
                    Some(counts),
                    None,
                )
                .await
        }
        DispatchOutcome::Cancelled => {
            tracing::info!(job_id = %job.job_id, "redteam: job cancelled");
            return;
        }
        DispatchOutcome::Failed(message) => {
            tracing::warn!(job_id = %job.job_id, error = %message, "redteam: job failed");
            store
                .set_status(
                    &job.workspace_id,
                    &job.job_id,
                    JobStatus::Error,
                    None,
                    Some(&message),
                )
                .await
        }
    };
    if let Err(e) = result {
        tracing::error!(job_id = %job.job_id, error = %e, "redteam: cannot write final status");
    }
}

/// Drive one job: dispatch to the runner, poll to completion (checking for
/// cancellation between polls), and persist results. Never returns an error
/// type — every failure path collapses into `DispatchOutcome`.
async fn drive(
    runner: &dyn RedteamRunner,
    store: &dyn RedteamJobStore,
    config: &DispatchConfig,
    job: &DispatchJob,
) -> DispatchOutcome {
    let dispatch = RunnerDispatch {
        target_url: job.request.target_url.clone(),
        profile: job.request.profile.clone(),
        mode: runner_mode(job.request.mode),
        attack_surface: runner_attack_surface(job.request.attack_surface),
        document_template: job.request.document_template.as_ref().map(|template| {
            RunnerDocumentTemplate {
                file_name: template.file_name.clone(),
                media_type: template.media_type.clone(),
                data_base64: template.data_base64.clone(),
                fields: template.fields.clone(),
                flatten: template.flatten,
            }
        }),
        // Forward the agent's planned vectors as seeds. Drop the product-side
        // `source_path` provenance — the runner only needs the seed itself.
        attack_vectors: job.request.attack_vectors.as_ref().map(|vectors| {
            vectors
                .iter()
                .map(|v| RunnerAttackVector {
                    goal: v.goal.clone(),
                    technique: v.technique.clone(),
                    target_operation: v.target_operation.clone(),
                    injection_payload: v.injection_payload.clone(),
                })
                .collect()
        }),
    };
    let handle = match runner.dispatch(&dispatch).await {
        Ok(handle) => handle,
        Err(e) => return DispatchOutcome::Failed(e.to_string()),
    };

    let deadline = Instant::now() + config.max_duration;
    loop {
        if is_cancelled(store, job).await {
            return DispatchOutcome::Cancelled;
        }
        match runner.poll(&handle.job_id).await {
            Ok(report) => match report.status {
                RunnerStatus::Complete => {
                    return match persist_sessions(store, job, &report.sessions).await {
                        Ok(counts) => DispatchOutcome::Completed(counts),
                        Err(e) => DispatchOutcome::Failed(e.to_string()),
                    };
                }
                RunnerStatus::Error => {
                    return DispatchOutcome::Failed(
                        report
                            .error
                            .unwrap_or_else(|| "runner reported error".to_string()),
                    );
                }
                RunnerStatus::Running => {
                    if Instant::now() >= deadline {
                        return DispatchOutcome::Failed(format!(
                            "timed out after {:?}",
                            config.max_duration
                        ));
                    }
                    tokio::time::sleep(config.poll_interval).await;
                }
            },
            Err(e) => return DispatchOutcome::Failed(e.to_string()),
        }
    }
}

fn runner_mode(mode: tl_core::RedteamRunMode) -> RunnerRunMode {
    match mode {
        tl_core::RedteamRunMode::OneOff => RunnerRunMode::OneOff,
        tl_core::RedteamRunMode::Learning => RunnerRunMode::Learning,
    }
}

fn runner_attack_surface(surface: tl_core::RedteamAttackSurface) -> RunnerAttackSurface {
    match surface {
        tl_core::RedteamAttackSurface::Chat => RunnerAttackSurface::Chat,
        tl_core::RedteamAttackSurface::DocumentWorkflow => RunnerAttackSurface::DocumentWorkflow,
    }
}

async fn is_cancelled(store: &dyn RedteamJobStore, job: &DispatchJob) -> bool {
    matches!(
        store.get(&job.workspace_id, &job.job_id).await,
        Ok(summary) if summary.status == JobStatus::Cancelled
    )
}

/// Persist every scored session and roll up the counts the summary carries.
async fn persist_sessions(
    store: &dyn RedteamJobStore,
    job: &DispatchJob,
    sessions: &[RunnerAttackSession],
) -> Result<JobCounts, super::RedteamJobStoreError> {
    let mut counts = JobCounts::default();
    for (index, session) in sessions.iter().enumerate() {
        let created_at = chrono::Utc::now().to_rfc3339();
        let persisted = RedteamAttackSession {
            session_id: session.session_id.clone(),
            runner_session_id: session.runner_session_id.clone(),
            seq: session.seq.max(index as i32),
            case_id: session.case_id.clone(),
            track: session.track.clone(),
            kind: session.kind.clone(),
            trial_index: session.trial_index,
            attack: session.attack.clone(),
            goal: session.goal.clone(),
            status: runner_status_label(session.status).to_string(),
            outcome: session.outcome.clone(),
            landed: session.landed,
            trace_id: session.trace_id.clone(),
            events: session
                .events
                .iter()
                .map(|event| RedteamSessionEvent {
                    event_id: event.event_id.clone(),
                    seq: event.seq,
                    kind: event.kind.clone(),
                    actor: event.actor.clone(),
                    label: event.label.clone(),
                    content_text: event.content_text.clone(),
                    payload: event.payload.clone(),
                    trace_id: event.trace_id.clone(),
                    created_at: created_at.clone(),
                })
                .collect(),
            error: session.error.clone(),
        };
        store
            .record_session(&job.workspace_id, &job.job_id, &persisted)
            .await?;
        counts.attacks += 1;
        if session.landed {
            counts.landed += 1;
        }
        if session.outcome == "blocked" {
            counts.blocked += 1;
        }
    }
    Ok(counts)
}

fn runner_status_label(status: RunnerStatus) -> &'static str {
    match status {
        RunnerStatus::Running => "running",
        RunnerStatus::Complete => "complete",
        RunnerStatus::Error => "error",
    }
}
