//! HTTP client for a compatible private red-team runner.
//!
//! Rust dispatches a job, polls until it finishes, and persists the scored
//! results. The `RedteamRunner` trait is the boundary the orchestrator depends
//! on, so tests drive the worker with a fake runner instead of a live service.

use std::time::Duration;

use async_trait::async_trait;

pub(crate) use tl_core::redteam_runner::{
    RunnerAttackSession, RunnerAttackSurface, RunnerAttackVector, RunnerDispatch,
    RunnerDocumentTemplate, RunnerHandle, RunnerPlanRequest, RunnerPlanResponse, RunnerReport,
    RunnerRunMode, RunnerStatus, RunnerWorkflowPath,
};

/// Per-request timeout. The runner creates/queries a job quickly; the
/// long-running `.hack()` happens between polls, so individual calls are
/// short. Overall job duration is bounded by the orchestrator poll loop.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("runner transport: {0}")]
    Transport(String),
    #[error("runner returned status {0}")]
    Status(u16),
}

#[async_trait]
pub(crate) trait RedteamRunner: Send + Sync {
    async fn dispatch(&self, request: &RunnerDispatch) -> Result<RunnerHandle, RunnerError>;
    async fn poll(&self, runner_job_id: &str) -> Result<RunnerReport, RunnerError>;
}

#[async_trait]
pub trait RedteamPlanner: Send + Sync {
    async fn plan(&self, request: &RunnerPlanRequest) -> Result<RunnerPlanResponse, RunnerError>;
}

pub(crate) struct RedteamRunnerClient {
    http: reqwest::Client,
    base_url: String,
}

impl RedteamRunnerClient {
    /// Build a client from `REDTEAM_RUNNER_URL`. Returns `None` when the
    /// var is unset/empty (dispatch disabled) or the client cannot be
    /// built — the caller surfaces a clear `503` in that case.
    pub(crate) fn from_env() -> Option<Self> {
        let base = std::env::var("REDTEAM_RUNNER_URL").ok()?;
        if base.trim().is_empty() {
            return None;
        }
        match Self::new(base) {
            Ok(client) => Some(client),
            Err(e) => {
                tracing::warn!(error = %e, "redteam runner client init failed; dispatch disabled");
                None
            }
        }
    }

    pub(crate) fn new(base_url: impl Into<String>) -> Result<Self, RunnerError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        // reqwest::ClientBuilder::build() does not parse the base URL, so validate
        // it here. An invalid REDTEAM_RUNNER_URL must fail init so `from_env` takes
        // the dispatch-disabled (503) path instead of spawning a dead worker.
        let parsed = url::Url::parse(&base_url)
            .map_err(|e| RunnerError::Transport(format!("invalid REDTEAM_RUNNER_URL: {e}")))?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(RunnerError::Transport(format!(
                "REDTEAM_RUNNER_URL must be http(s), got scheme {:?}",
                parsed.scheme()
            )));
        }
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| RunnerError::Transport(e.to_string()))?;
        Ok(Self { http, base_url })
    }
}

#[async_trait]
impl RedteamRunner for RedteamRunnerClient {
    async fn dispatch(&self, request: &RunnerDispatch) -> Result<RunnerHandle, RunnerError> {
        let resp = self
            .http
            .post(format!("{}/redteam/jobs", self.base_url))
            .json(request)
            .send()
            .await
            .map_err(transport)?;
        if !resp.status().is_success() {
            return Err(RunnerError::Status(resp.status().as_u16()));
        }
        resp.json::<RunnerHandle>().await.map_err(transport)
    }

    async fn poll(&self, runner_job_id: &str) -> Result<RunnerReport, RunnerError> {
        let resp = self
            .http
            .get(format!("{}/redteam/jobs/{}", self.base_url, runner_job_id))
            .send()
            .await
            .map_err(transport)?;
        if !resp.status().is_success() {
            return Err(RunnerError::Status(resp.status().as_u16()));
        }
        resp.json::<RunnerReport>().await.map_err(transport)
    }
}

#[async_trait]
impl RedteamPlanner for RedteamRunnerClient {
    async fn plan(&self, request: &RunnerPlanRequest) -> Result<RunnerPlanResponse, RunnerError> {
        let resp = self
            .http
            .post(format!("{}/redteam/plan", self.base_url))
            .json(request)
            .send()
            .await
            .map_err(transport)?;
        if !resp.status().is_success() {
            return Err(RunnerError::Status(resp.status().as_u16()));
        }
        resp.json::<RunnerPlanResponse>().await.map_err(transport)
    }
}

fn transport(e: reqwest::Error) -> RunnerError {
    RunnerError::Transport(e.to_string())
}
