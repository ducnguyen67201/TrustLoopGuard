//! Red-team dispatch orchestrator endpoints (`/v1/redteam/*`).
//!
//! A *dispatch* creates a durable *job* (`Queued`) and hands it to an
//! in-process worker that drives the standalone attack runner, persists
//! per-attack results, and rolls up final counts. Rust owns the job +
//! results; the runner owns nothing. The store is the source of truth,
//! so cancellation and (future) requeue-on-boot are clean status reads.

mod context;
pub(crate) mod handlers;
mod memory_store;
mod orchestrator;
mod rate_limit;
mod report;
mod response;
mod runner_client;
mod share;
mod validation;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{JobStatus, RedteamDispatchRequest, RedteamJobResult, RedteamJobSummary};

use crate::environments::EnvironmentStore;

pub use handlers::{
    cancel_job, create_report, dispatch_job, get_job, get_public_report, get_report, list_jobs,
    list_results, revoke_report,
};
pub use memory_store::MemoryRedteamJobStore;
pub use orchestrator::DispatchJob;
pub(crate) use orchestrator::{spawn_dispatch_worker, DispatchConfig};
pub use rate_limit::ReportRateLimiter;
pub(crate) use runner_client::RedteamRunnerClient;
pub use share::{
    generate_share_token, MemoryRedteamReportShareStore, NewReportShare, RedteamReportShareStore,
    ReportShare,
};

#[derive(Debug, thiserror::Error)]
pub enum RedteamJobStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    /// The dispatch worker is not configured or its queue is saturated.
    /// Maps to `503` so callers retry with backoff.
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Filter for `GET /v1/redteam/jobs`. Workspace scoping is implicit.
#[derive(Debug, Clone, Default)]
pub struct RedteamJobListFilter {
    pub agent_id: Option<String>,
    pub limit: usize,
}

/// Rolled-up attack counts written when a job finishes. Mirrors
/// `tl_storage::JobCounts` but lives server-side so the trait stays
/// usable in memory-only (non-Postgres) builds.
#[derive(Debug, Default, Clone, Copy)]
pub struct JobCounts {
    pub attacks: i64,
    pub landed: i64,
    pub blocked: i64,
}

/// Whether a job has reached a final state and must not be transitioned
/// further (the worker checks this before writing `Complete`).
pub(crate) fn is_terminal(status: JobStatus) -> bool {
    matches!(
        status,
        JobStatus::Complete | JobStatus::Error | JobStatus::Cancelled
    )
}

/// Durable storage for red-team jobs + per-attack results.
///
/// Handlers use `create`/`list`/`get`/`list_results`/`cancel`; the
/// orchestrator additionally uses `set_status`/`record_result`.
#[async_trait]
pub trait RedteamJobStore: Send + Sync {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: &RedteamDispatchRequest,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError>;
    async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamJobListFilter,
    ) -> Result<Vec<RedteamJobSummary>, RedteamJobStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError>;
    async fn list_results(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<Vec<RedteamJobResult>, RedteamJobStoreError>;
    /// Transition a job and, on completion, persist rolled-up counts.
    ///
    /// Terminal states (`Complete`/`Error`/`Cancelled`) are final: the first
    /// terminal write wins, so a late completion cannot clobber a concurrent
    /// cancel. The orchestrator drives `Queued → Running → {Complete, Error}`;
    /// `Cancelled` is reached only via [`cancel`](Self::cancel). `counts` is
    /// applied only when supplied (completion); a status-only transition leaves
    /// existing counts intact.
    async fn set_status(
        &self,
        workspace_id: &str,
        job_id: &str,
        status: JobStatus,
        counts: Option<JobCounts>,
        error: Option<&str>,
    ) -> Result<(), RedteamJobStoreError>;
    async fn record_result(
        &self,
        workspace_id: &str,
        job_id: &str,
        result: &RedteamJobResult,
    ) -> Result<(), RedteamJobStoreError>;
    /// Cooperatively cancel a job. No-op (returns the job unchanged) when
    /// it has already reached a terminal state.
    async fn cancel(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError>;
}

#[derive(Clone)]
pub struct RedteamState {
    pub store: Arc<dyn RedteamJobStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    /// Durable store for shareable report tokens.
    pub report_share_store: Arc<dyn RedteamReportShareStore>,
    /// Sender into the in-process dispatch worker. `None` when
    /// `REDTEAM_RUNNER_URL` is unset — dispatch returns `503`.
    pub dispatch_tx: Option<tokio::sync::mpsc::Sender<DispatchJob>>,
}

/// State for the public, unauthenticated report endpoint. Carries only what the
/// token-scoped read needs: the share store (to resolve the token) and the job
/// store (to fetch the report data within the token's workspace).
#[derive(Clone)]
pub struct PublicReportState {
    pub store: Arc<dyn RedteamJobStore>,
    pub report_share_store: Arc<dyn RedteamReportShareStore>,
    /// Per-token rate limiter for the unauthenticated public read.
    pub rate_limiter: Arc<ReportRateLimiter>,
}
