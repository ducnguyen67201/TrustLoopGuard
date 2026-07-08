//! Red-team dispatch orchestrator endpoints (`/v1/redteam/*`).
//!
//! A *dispatch* creates a durable *job* (`Queued`) and hands it to an
//! in-process worker that drives a compatible private runner, persists
//! per-attack sessions, and rolls up final counts. Rust owns the job +
//! sessions; the runner owns nothing. The store is the source of truth,
//! so cancellation and (future) requeue-on-boot are clean status reads.

mod context;
pub(crate) mod handlers;
pub(crate) mod harden;
mod harden_draft;
mod memory_store;
mod orchestrator;
pub(crate) mod plan;
pub(crate) mod plan_store;
mod rate_limit;
mod regression_store;
mod report;
mod report_diagnostic;
mod response;
mod runner_client;
mod share;
mod validation;
mod verify;
mod workflow_analyzer;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    JobStatus, RedteamAttackRecord, RedteamAttackSession, RedteamDispatchRequest, RedteamJobSummary,
};

use crate::agents::AgentStore;
use crate::environments::EnvironmentStore;
use crate::tool_metadata::ToolMetadataStore;

pub use handlers::{
    cancel_job, create_report, dispatch_job, get_job, get_public_report, get_regression_results,
    get_report, list_attack_records, list_jobs, list_regression_cases,
    list_regression_result_snapshots, revoke_report, run_regression_cases,
};
pub use harden::harden_job;
pub(crate) use harden_draft::{HardenDraftError, HardenDraftInput, HardenDrafter};
pub use memory_store::MemoryRedteamJobStore;
pub use orchestrator::DispatchJob;
pub(crate) use orchestrator::{spawn_dispatch_worker, DispatchConfig};
pub use plan::{delete_plan, generate_static_policies, list_plans, plan_attack_vectors, PlanState};
pub use plan_store::{MemoryRedteamPlanStore, RedteamPlanStore, RedteamPlanStoreError};
pub use rate_limit::ReportRateLimiter;
pub use regression_store::{
    MemoryRedteamRegressionStore, NewRegressionCase, NewRegressionResultSnapshot,
    RedteamRegressionCaseFilter, RedteamRegressionResultFilter, RedteamRegressionStore,
    RedteamRegressionStoreError,
};
pub(crate) use report_diagnostic::enrich_report_diagnostics;
pub(crate) use runner_client::RedteamRunnerClient;
pub use runner_client::{RedteamPlanner, RunnerError};
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

/// Filter for `GET /v1/redteam/attacks`. Workspace scoping is implicit. Mirrors
/// `tl_storage::RedteamAttackRecordFilter` but lives server-side (`usize` limit)
/// so the trait stays storage-agnostic.
#[derive(Debug, Clone, Default)]
pub struct RedteamAttackRecordFilter {
    pub attack: Option<String>,
    pub outcome: Option<String>,
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

/// Durable storage for red-team jobs + per-attack sessions.
///
/// Handlers use `create`/`list`/`get`/`list_sessions`/`list_attack_records`/
/// `cancel`; the orchestrator additionally uses `set_status`/`record_session`.
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
    async fn list_sessions(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<Vec<RedteamAttackSession>, RedteamJobStoreError>;
    /// Every attack result in the workspace, flattened with parent-job context,
    /// newest job first. Powers the workspace-wide records browser.
    async fn list_attack_records(
        &self,
        workspace_id: &str,
        filter: RedteamAttackRecordFilter,
    ) -> Result<Vec<RedteamAttackRecord>, RedteamJobStoreError>;
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
    async fn record_session(
        &self,
        workspace_id: &str,
        job_id: &str,
        session: &RedteamAttackSession,
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
    pub agent_store: Option<Arc<dyn AgentStore>>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    /// Durable store for shareable report tokens.
    pub report_share_store: Arc<dyn RedteamReportShareStore>,
    /// Sender into the in-process dispatch worker. `None` when
    /// `REDTEAM_RUNNER_URL` is unset — dispatch returns `503`.
    pub dispatch_tx: Option<tokio::sync::mpsc::Sender<DispatchJob>>,
    /// Policy store used by the harden endpoint to persist recommended
    /// guardrails (`enabled = false`).
    pub policy_store: Arc<dyn crate::policies::PolicyStore>,
    /// Tool registry used by the harden endpoint to persist verified
    /// event-level guardrails such as approval requirements.
    pub tool_metadata_store: Arc<dyn ToolMetadataStore>,
    /// Source-label policy registry used by the harden endpoint to persist
    /// verified provenance/information-flow hardening recommendations.
    pub label_policy_store: Arc<dyn crate::label_policy::LabelPolicyStore>,
    /// Durable regression cases promoted from verified harden survivors.
    pub regression_store: Arc<dyn RedteamRegressionStore>,
    /// Runtime LLM judge, reused by the harden verify loop so a candidate's
    /// verdict matches production exactly.
    pub llm: Arc<tl_llm::LlmRouter>,
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
