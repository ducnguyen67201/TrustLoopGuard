//! HTTP client for the standalone attack runner (TrustLoopRed sidecar).
//!
//! The runner is the only thing that can execute hackagent. Rust dispatches
//! a job, polls until it finishes, and persists the scored results. The
//! `RedteamRunner` trait is the seam the orchestrator depends on, so tests
//! drive the worker with a fake runner instead of a live sidecar.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Per-request timeout. The runner creates/queries a job quickly; the
/// long-running `.hack()` happens between polls, so individual calls are
/// short. Overall job duration is bounded by the orchestrator poll loop.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub(crate) enum RunnerError {
    #[error("runner transport: {0}")]
    Transport(String),
    #[error("runner returned status {0}")]
    Status(u16),
}

/// Body of `POST /redteam/jobs` (camelCase to match the sidecar contract).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunnerDispatch {
    pub target_url: String,
    pub profile: String,
    pub generator: String,
}

/// Response from `POST /redteam/jobs`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunnerHandle {
    pub job_id: String,
}

/// Lifecycle the runner reports for one of its jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunnerStatus {
    Running,
    Complete,
    Error,
}

/// Response from `GET /redteam/jobs/{id}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunnerReport {
    pub status: RunnerStatus,
    #[serde(default)]
    pub attacks: Vec<RunnerAttack>,
    #[serde(default)]
    pub error: Option<String>,
}

/// One scored attack from the runner. The runner owns scoring; Rust copies
/// the verdict verbatim and never re-scores.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunnerAttack {
    pub attack: String,
    pub goal: String,
    pub outcome: String,
    pub landed: bool,
    #[serde(default)]
    pub prompt: Option<String>,
    pub reply: String,
    #[serde(default)]
    pub trace_id: Option<String>,
}

#[async_trait]
pub(crate) trait RedteamRunner: Send + Sync {
    async fn dispatch(&self, request: &RunnerDispatch) -> Result<RunnerHandle, RunnerError>;
    async fn poll(&self, runner_job_id: &str) -> Result<RunnerReport, RunnerError>;
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
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| RunnerError::Transport(e.to_string()))?;
        let base_url = base_url.into().trim_end_matches('/').to_string();
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

fn transport(e: reqwest::Error) -> RunnerError {
    RunnerError::Transport(e.to_string())
}
