use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{BenchArm, BenchRunCreateRequest, BenchRunDetail, BenchRunStatus, BenchRunSummary};
use tl_storage::{BenchRunArmRowInput, BenchRunFilter, BenchRunRepo, StorageError};

use crate::bench::{BenchRunArmInput, BenchRunStore, BenchRunStoreError};

pub struct PostgresBenchRunAdapter(pub Arc<BenchRunRepo>);

impl PostgresBenchRunAdapter {
    pub fn new(repo: Arc<BenchRunRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl BenchRunStore for PostgresBenchRunAdapter {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: &BenchRunCreateRequest,
    ) -> Result<BenchRunSummary, BenchRunStoreError> {
        self.0
            .create(workspace_id, environment_id, request)
            .await
            .map_err(bench_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<BenchRunSummary>, BenchRunStoreError> {
        self.0
            .list(
                workspace_id,
                BenchRunFilter {
                    limit: clamp_limit(limit),
                },
            )
            .await
            .map_err(bench_store_error)
    }

    async fn get(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<BenchRunSummary, BenchRunStoreError> {
        self.0
            .get(workspace_id, run_id)
            .await
            .map_err(bench_store_error)
    }

    async fn get_detail(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<BenchRunDetail, BenchRunStoreError> {
        self.0
            .get_detail(workspace_id, run_id)
            .await
            .map_err(bench_store_error)
    }

    async fn attach_arm(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: BenchRunArmInput,
    ) -> Result<tl_core::BenchRunArmSummary, BenchRunStoreError> {
        self.0
            .attach_arm(
                workspace_id,
                run_id,
                BenchRunArmRowInput {
                    arm: arm_text(input.arm).to_string(),
                    label: input.label,
                    target: input.target,
                    redteam_job_id: input.redteam_job_id,
                    checker_config: input.checker_config,
                },
            )
            .await
            .map_err(bench_store_error)
    }

    async fn set_status(
        &self,
        workspace_id: &str,
        run_id: &str,
        status: BenchRunStatus,
        error: Option<&str>,
    ) -> Result<(), BenchRunStoreError> {
        self.0
            .set_status(workspace_id, run_id, status, error)
            .await
            .map_err(bench_store_error)
    }

    async fn cancel(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<BenchRunSummary, BenchRunStoreError> {
        self.0
            .set_status(workspace_id, run_id, BenchRunStatus::Cancelled, None)
            .await
            .map_err(bench_store_error)?;
        self.0
            .get(workspace_id, run_id)
            .await
            .map_err(bench_store_error)
    }
}

fn arm_text(arm: BenchArm) -> &'static str {
    match arm {
        BenchArm::Raw => "raw",
        BenchArm::Guarded => "guarded",
    }
}

fn clamp_limit(limit: usize) -> i64 {
    limit.clamp(1, 100) as i64
}

fn bench_store_error(error: StorageError) -> BenchRunStoreError {
    match error {
        StorageError::NotFound => BenchRunStoreError::NotFound,
        StorageError::Conflict => BenchRunStoreError::Internal("conflict".into()),
        StorageError::Internal(message) => BenchRunStoreError::Internal(message),
    }
}
