use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{RegressionCaseSummary, RegressionResultSnapshotSummary};
use tl_storage::{
    NewRegressionCaseParams, NewRegressionResultSnapshotParams,
    RedteamRegressionCaseFilter as StorageRegressionCaseFilter, RedteamRegressionRepo,
    RedteamRegressionResultFilter as StorageRegressionResultFilter, StorageError,
};

use crate::redteam::{
    NewRegressionCase, NewRegressionResultSnapshot, RedteamRegressionCaseFilter,
    RedteamRegressionResultFilter, RedteamRegressionStore, RedteamRegressionStoreError,
};

pub struct PostgresRedteamRegressionAdapter(pub Arc<RedteamRegressionRepo>);

impl PostgresRedteamRegressionAdapter {
    pub fn new(repo: Arc<RedteamRegressionRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl RedteamRegressionStore for PostgresRedteamRegressionAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        input: NewRegressionCase,
    ) -> Result<RegressionCaseSummary, RedteamRegressionStoreError> {
        self.0
            .upsert(
                workspace_id,
                NewRegressionCaseParams {
                    case_key: input.case_key,
                    environment_id: input.environment_id,
                    agent_id: input.agent_id,
                    source: input.source,
                    source_job_id: input.source_job_id,
                    source_session_seqs: input.source_session_seqs,
                    substrate: input.substrate,
                    artifact_id: input.artifact_id,
                    expected_outcome: input.expected_outcome,
                    attack: input.attack,
                    goal: input.goal,
                },
            )
            .await
            .map_err(regression_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionCaseFilter,
    ) -> Result<Vec<RegressionCaseSummary>, RedteamRegressionStoreError> {
        self.0
            .list(
                workspace_id,
                StorageRegressionCaseFilter {
                    agent_id: filter.agent_id,
                    source_job_id: filter.source_job_id,
                    case_keys: filter.case_keys,
                    limit: filter.limit.clamp(1, 100) as i64,
                },
            )
            .await
            .map_err(regression_store_error)
    }

    async fn record_result_snapshot(
        &self,
        workspace_id: &str,
        input: NewRegressionResultSnapshot,
    ) -> Result<RegressionResultSnapshotSummary, RedteamRegressionStoreError> {
        self.0
            .record_result_snapshot(
                workspace_id,
                NewRegressionResultSnapshotParams {
                    job_id: input.job_id,
                    source_job_id: input.source_job_id,
                    environment_id: input.environment_id,
                    agent_id: input.agent_id,
                    case_keys: input.case_keys,
                    total: input.total,
                    passed: input.passed,
                    failed: input.failed,
                    missing: input.missing,
                    inconclusive: input.inconclusive,
                },
            )
            .await
            .map_err(regression_store_error)
    }

    async fn list_result_snapshots(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionResultFilter,
    ) -> Result<Vec<RegressionResultSnapshotSummary>, RedteamRegressionStoreError> {
        self.0
            .list_result_snapshots(
                workspace_id,
                StorageRegressionResultFilter {
                    source_job_id: filter.source_job_id,
                    job_id: filter.job_id,
                    agent_id: filter.agent_id,
                    limit: filter.limit.clamp(1, 100) as i64,
                },
            )
            .await
            .map_err(regression_store_error)
    }
}

fn regression_store_error(error: StorageError) -> RedteamRegressionStoreError {
    match error {
        StorageError::NotFound => RedteamRegressionStoreError::NotFound,
        StorageError::Conflict => RedteamRegressionStoreError::Internal("conflict".into()),
        StorageError::Internal(message) => RedteamRegressionStoreError::Internal(message),
    }
}
