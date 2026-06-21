use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    JobStatus, RedteamAttackRecord, RedteamAttackSession, RedteamDispatchRequest, RedteamJobSummary,
};
use tl_storage::{
    JobCounts as StorageJobCounts, RedteamAttackRecordFilter as StorageAttackRecordFilter,
    RedteamJobFilter, RedteamJobRepo, StorageError,
};

use crate::redteam::{
    JobCounts, RedteamAttackRecordFilter, RedteamJobListFilter, RedteamJobStore,
    RedteamJobStoreError,
};

pub struct PostgresRedteamJobAdapter(pub Arc<RedteamJobRepo>);

impl PostgresRedteamJobAdapter {
    pub fn new(repo: Arc<RedteamJobRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl RedteamJobStore for PostgresRedteamJobAdapter {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: &RedteamDispatchRequest,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError> {
        self.0
            .create(workspace_id, environment_id, request)
            .await
            .map_err(job_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamJobListFilter,
    ) -> Result<Vec<RedteamJobSummary>, RedteamJobStoreError> {
        self.0
            .list(
                workspace_id,
                RedteamJobFilter {
                    agent_id: filter.agent_id,
                    limit: clamp_limit(filter.limit),
                },
            )
            .await
            .map_err(job_store_error)
    }

    async fn get(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError> {
        self.0
            .get(workspace_id, job_id)
            .await
            .map_err(job_store_error)
    }

    async fn list_sessions(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<Vec<RedteamAttackSession>, RedteamJobStoreError> {
        self.0
            .list_sessions(workspace_id, job_id)
            .await
            .map_err(job_store_error)
    }

    async fn list_attack_records(
        &self,
        workspace_id: &str,
        filter: RedteamAttackRecordFilter,
    ) -> Result<Vec<RedteamAttackRecord>, RedteamJobStoreError> {
        self.0
            .list_attack_records(
                workspace_id,
                StorageAttackRecordFilter {
                    attack: filter.attack,
                    outcome: filter.outcome,
                    limit: clamp_limit(filter.limit),
                },
            )
            .await
            .map_err(job_store_error)
    }

    async fn set_status(
        &self,
        workspace_id: &str,
        job_id: &str,
        status: JobStatus,
        counts: Option<JobCounts>,
        error: Option<&str>,
    ) -> Result<(), RedteamJobStoreError> {
        let counts = counts.map(|c| StorageJobCounts {
            attacks: c.attacks,
            landed: c.landed,
            blocked: c.blocked,
        });
        self.0
            .set_status(workspace_id, job_id, status, counts, error)
            .await
            .map_err(job_store_error)
    }

    async fn record_session(
        &self,
        workspace_id: &str,
        job_id: &str,
        session: &RedteamAttackSession,
    ) -> Result<(), RedteamJobStoreError> {
        self.0
            .record_session(workspace_id, job_id, session)
            .await
            .map_err(job_store_error)
    }

    async fn cancel(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError> {
        // `set_status`'s terminal guard makes this race-free: a queued/running
        // job transitions to cancelled; an already-terminal job is left
        // untouched (no get→check→set TOCTOU). Re-read for the resulting summary.
        self.0
            .set_status(workspace_id, job_id, JobStatus::Cancelled, None, None)
            .await
            .map_err(job_store_error)?;
        self.0
            .get(workspace_id, job_id)
            .await
            .map_err(job_store_error)
    }
}

/// Clamp a `usize` page limit into the storage range *before* the `i64` cast.
/// A caller-supplied limit above `i64::MAX` would otherwise wrap negative, and
/// the storage-side `clamp(1, 100)` would then read it as 1 — silently returning
/// a single row instead of the intended page. Clamping first keeps the cast safe
/// and matches the in-memory store, which clamps the `usize` directly.
fn clamp_limit(limit: usize) -> i64 {
    limit.clamp(1, 100) as i64
}

fn job_store_error(error: StorageError) -> RedteamJobStoreError {
    match error {
        StorageError::NotFound => RedteamJobStoreError::NotFound,
        StorageError::Conflict => RedteamJobStoreError::Internal("conflict".into()),
        StorageError::Internal(message) => RedteamJobStoreError::Internal(message),
    }
}
