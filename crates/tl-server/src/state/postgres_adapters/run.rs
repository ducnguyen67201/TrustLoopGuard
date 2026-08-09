use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_storage::{RunFilter, RunRepo};

use crate::runs::{RunListFilter, RunStore, RunStoreError};

pub struct PostgresRunAdapter(pub Arc<RunRepo>);

impl PostgresRunAdapter {
    pub fn new(repo: Arc<RunRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl RunStore for PostgresRunAdapter {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: tl_core::CreateRunRequest,
    ) -> Result<tl_core::RunSummary, RunStoreError> {
        self.0
            .create(workspace_id, environment_id, input)
            .await
            .map_err(run_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
        filter: RunListFilter,
    ) -> Result<Vec<tl_core::RunSummary>, RunStoreError> {
        self.0
            .list(
                workspace_id,
                RunFilter {
                    environment_id: Some(environment_id.to_string()),
                    agent_id: filter.agent_id,
                    status: filter.status,
                    kind: filter.kind,
                    external_id: filter.external_id,
                    limit: filter.limit as i64,
                },
            )
            .await
            .map_err(run_store_error)
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<tl_core::RunSummary, RunStoreError> {
        self.0
            .get(workspace_id, run_id)
            .await
            .and_then(|run| {
                if run.environment_id == environment_id {
                    Ok(run)
                } else {
                    Err(tl_storage::StorageError::NotFound)
                }
            })
            .map_err(run_store_error)
    }

    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: tl_core::UpdateRunRequest,
    ) -> Result<tl_core::RunSummary, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .update(workspace_id, run_id, input)
            .await
            .map_err(run_store_error)
    }

    async fn finalize(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: tl_core::FinalizeRunRequest,
        capture_wait_ms: u64,
    ) -> Result<tl_core::FinalizeRunResponse, RunStoreError> {
        self.0
            .finalize(workspace_id, environment_id, run_id, input, capture_wait_ms)
            .await
            .map_err(run_store_error)
    }

    async fn finalization(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Option<tl_core::RunFinalizationSummary>, RunStoreError> {
        self.0
            .finalization(workspace_id, environment_id, run_id)
            .await
            .map_err(run_store_error)
    }

    async fn create_event(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: tl_core::CreateRunEventRequest,
    ) -> Result<tl_core::RunEventSummary, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .create_event(workspace_id, run_id, input)
            .await
            .map_err(run_store_error)
    }

    async fn events(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::RunEventSummary>, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .events(workspace_id, run_id, limit as i64)
            .await
            .map_err(run_store_error)
    }

    async fn traces(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::TraceSummary>, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .traces(workspace_id, run_id, limit as i64)
            .await
            .map_err(run_store_error)
    }

    async fn event_belongs_to_run(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        run_event_id: &str,
    ) -> Result<(), RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .event_belongs_to_run(workspace_id, run_id, run_event_id)
            .await
            .map_err(run_store_error)
    }
}

fn run_store_error(error: tl_storage::StorageError) -> RunStoreError {
    match error {
        tl_storage::StorageError::NotFound => RunStoreError::NotFound,
        tl_storage::StorageError::Conflict => RunStoreError::Conflict,
        tl_storage::StorageError::Internal(message) if message.contains("parse") => {
            RunStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => RunStoreError::Internal(message),
    }
}
