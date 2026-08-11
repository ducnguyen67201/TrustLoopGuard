//! Dashboard run grouping endpoints.

mod context;
pub(crate) mod handlers;
mod memory_store;
mod query;
mod response;
mod validation;

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    CreateRunEventRequest, CreateRunRequest, FinalizeRunRequest, FinalizeRunResponse,
    RunEventSummary, RunFinalizationSummary, RunKind, RunSpanSummary, RunStatus, RunSummary,
    TraceSummary, UpdateRunRequest,
};

use crate::environments::EnvironmentStore;
pub use handlers::{
    create_run, create_run_event, finalize_run, get_run, list_run_events, list_run_traces,
    list_runs, update_run,
};
pub use memory_store::MemoryRunStore;

#[derive(Debug, thiserror::Error)]
pub enum RunStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("conflict")]
    Conflict,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Default)]
pub struct RunListFilter {
    pub environment_id: Option<String>,
    pub agent_id: Option<String>,
    pub status: Option<RunStatus>,
    pub kind: Option<RunKind>,
    pub external_id: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct StaleGatewayRun {
    pub workspace_id: String,
    pub environment_id: String,
    pub run_id: String,
    pub agent_id: String,
    pub max_duration_exceeded: bool,
}

#[async_trait]
pub trait RunStore: Send + Sync {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateRunRequest,
    ) -> Result<RunSummary, RunStoreError>;
    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
        filter: RunListFilter,
    ) -> Result<Vec<RunSummary>, RunStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<RunSummary, RunStoreError>;
    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: UpdateRunRequest,
    ) -> Result<RunSummary, RunStoreError>;
    async fn finalize(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: FinalizeRunRequest,
        capture_wait_ms: u64,
    ) -> Result<FinalizeRunResponse, RunStoreError>;
    async fn finalization(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Option<RunFinalizationSummary>, RunStoreError>;
    async fn create_event(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: CreateRunEventRequest,
    ) -> Result<RunEventSummary, RunStoreError>;
    async fn events(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<RunEventSummary>, RunStoreError>;
    async fn traces(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<TraceSummary>, RunStoreError>;
    async fn spans(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<RunSpanSummary>, RunStoreError>;

    async fn event_belongs_to_run(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        run_event_id: &str,
    ) -> Result<(), RunStoreError>;

    /// Update run stats after a guard check completes.
    /// Called from the check handler for every decision that carries a run_id.
    /// Postgres recomputes stats dynamically from the traces table, so the
    /// default no-op is correct for that path. Only MemoryRunStore overrides.
    async fn record_check(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _run_id: &str,
        _verdict: &str,
        _elapsed_ms: i32,
    ) -> Result<(), RunStoreError> {
        Ok(())
    }

    async fn touch_gateway_activity(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _run_id: &str,
    ) -> Result<(), RunStoreError> {
        Ok(())
    }

    async fn list_stale_gateway_runs(
        &self,
        _idle_before: chrono::DateTime<chrono::Utc>,
        _max_started_before: chrono::DateTime<chrono::Utc>,
        _limit: usize,
    ) -> Result<Vec<StaleGatewayRun>, RunStoreError> {
        Ok(vec![])
    }
}

#[derive(Clone)]
pub struct RunState {
    pub store: Arc<dyn RunStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    pub evaluation_store: Arc<dyn crate::evaluations::EvaluationStore>,
}
