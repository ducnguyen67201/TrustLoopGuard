use std::collections::HashMap;

use async_trait::async_trait;
use tl_core::{
    JobStatus, RedteamDispatchRequest, RedteamGenerator, RedteamJobResult, RedteamJobSummary,
};
use tokio::sync::RwLock;

use super::validation::clean_optional;
use super::{is_terminal, JobCounts, RedteamJobListFilter, RedteamJobStore, RedteamJobStoreError};

/// In-memory `RedteamJobStore` for tests and memory-only deployments.
/// Status transitions build new values rather than mutating in place where
/// practical; the `RwLock` guards the maps.
#[derive(Debug, Default)]
pub struct MemoryRedteamJobStore {
    jobs: RwLock<HashMap<String, RedteamJobSummary>>,
    results: RwLock<HashMap<String, Vec<RedteamJobResult>>>,
}

impl MemoryRedteamJobStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RedteamJobStore for MemoryRedteamJobStore {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: &RedteamDispatchRequest,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::now_v7().to_string();
        let job = RedteamJobSummary {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            status: JobStatus::Queued,
            target: request.target_url.trim().to_string(),
            profile: request.profile.trim().to_string(),
            generator: request.generator.unwrap_or(RedteamGenerator::Deterministic),
            agent_id: clean_optional(request.agent_id.clone()),
            attacks: 0,
            landed: 0,
            blocked: 0,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.jobs.write().await.insert(id, job.clone());
        Ok(job)
    }

    async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamJobListFilter,
    ) -> Result<Vec<RedteamJobSummary>, RedteamJobStoreError> {
        let mut rows: Vec<_> = self
            .jobs
            .read()
            .await
            .values()
            .filter(|job| job.workspace_id == workspace_id)
            .filter(|job| {
                filter
                    .agent_id
                    .as_deref()
                    .map_or(true, |id| job.agent_id.as_deref() == Some(id))
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        rows.truncate(filter.limit.clamp(1, 100));
        Ok(rows)
    }

    async fn get(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError> {
        self.jobs
            .read()
            .await
            .get(job_id)
            .filter(|job| job.workspace_id == workspace_id)
            .cloned()
            .ok_or(RedteamJobStoreError::NotFound)
    }

    async fn list_results(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<Vec<RedteamJobResult>, RedteamJobStoreError> {
        self.get(workspace_id, job_id).await?;
        let results = self.results.read().await;
        let mut rows = results.get(job_id).cloned().unwrap_or_default();
        rows.sort_by_key(|result| result.seq);
        Ok(rows)
    }

    async fn set_status(
        &self,
        workspace_id: &str,
        job_id: &str,
        status: JobStatus,
        counts: Option<JobCounts>,
        error: Option<&str>,
    ) -> Result<(), RedteamJobStoreError> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(job_id)
            .filter(|job| job.workspace_id == workspace_id)
            .ok_or(RedteamJobStoreError::NotFound)?;
        job.status = status;
        if let Some(counts) = counts {
            job.attacks = counts.attacks;
            job.landed = counts.landed;
            job.blocked = counts.blocked;
        }
        job.error = error.map(str::to_string);
        job.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    async fn record_result(
        &self,
        workspace_id: &str,
        job_id: &str,
        result: &RedteamJobResult,
    ) -> Result<(), RedteamJobStoreError> {
        self.get(workspace_id, job_id).await?;
        self.results
            .write()
            .await
            .entry(job_id.to_string())
            .or_default()
            .push(result.clone());
        Ok(())
    }

    async fn cancel(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<RedteamJobSummary, RedteamJobStoreError> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(job_id)
            .filter(|job| job.workspace_id == workspace_id)
            .ok_or(RedteamJobStoreError::NotFound)?;
        if !is_terminal(job.status) {
            job.status = JobStatus::Cancelled;
            job.updated_at = chrono::Utc::now().to_rfc3339();
        }
        Ok(job.clone())
    }
}
