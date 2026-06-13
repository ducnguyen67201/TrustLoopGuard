use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_storage::{NewShare, RedteamReportShareRepo, ReportShareRow, StorageError};

use crate::redteam::{NewReportShare, RedteamJobStoreError, RedteamReportShareStore, ReportShare};

pub struct PostgresRedteamReportShareAdapter(pub Arc<RedteamReportShareRepo>);

impl PostgresRedteamReportShareAdapter {
    pub fn new(repo: Arc<RedteamReportShareRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl RedteamReportShareStore for PostgresRedteamReportShareAdapter {
    async fn create(&self, new: NewReportShare<'_>) -> Result<ReportShare, RedteamJobStoreError> {
        let expires_at = match new.expires_at {
            Some(ts) => Some(parse_rfc3339(ts)?),
            None => None,
        };
        let row = self
            .0
            .create(NewShare {
                token: new.token,
                workspace_id: new.workspace_id,
                job_id: new.job_id,
                compare_job_id: new.compare_job_id,
                expires_at,
            })
            .await
            .map_err(share_store_error)?;
        Ok(report_share(row))
    }

    async fn get_by_token(&self, token: &str) -> Result<ReportShare, RedteamJobStoreError> {
        self.0
            .get_by_token(token)
            .await
            .map(report_share)
            .map_err(share_store_error)
    }

    async fn revoke(&self, workspace_id: &str, token: &str) -> Result<(), RedteamJobStoreError> {
        self.0
            .revoke(workspace_id, token)
            .await
            .map_err(share_store_error)
    }
}

fn report_share(row: ReportShareRow) -> ReportShare {
    ReportShare {
        token: row.token,
        workspace_id: row.workspace_id,
        job_id: row.job_id,
        compare_job_id: row.compare_job_id,
        created_at: row.created_at,
        expires_at: row.expires_at,
    }
}

fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>, RedteamJobStoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| RedteamJobStoreError::Internal(format!("invalid expiry timestamp: {e}")))
}

fn share_store_error(error: StorageError) -> RedteamJobStoreError {
    match error {
        StorageError::NotFound => RedteamJobStoreError::NotFound,
        StorageError::Conflict => RedteamJobStoreError::Internal("conflict".into()),
        StorageError::Internal(message) => RedteamJobStoreError::Internal(message),
    }
}
