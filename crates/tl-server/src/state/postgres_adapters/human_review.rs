use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::human_review::{HumanReviewAnalyticsFilter, HumanReviewStore, HumanReviewStoreError};

pub struct PostgresHumanReviewAdapter(pub Arc<tl_storage::HumanReviewRepo>);

impl PostgresHumanReviewAdapter {
    pub fn new(repo: Arc<tl_storage::HumanReviewRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl HumanReviewStore for PostgresHumanReviewAdapter {
    async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: tl_core::CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<tl_core::HumanReviewEvent, HumanReviewStoreError> {
        self.0
            .create_event(workspace_id, trace_id, input, reviewer_id)
            .await
            .map_err(human_review_store_error)
    }

    async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::HumanReviewEvent>, HumanReviewStoreError> {
        self.0
            .list_events(workspace_id, trace_id, limit as i64)
            .await
            .map_err(human_review_store_error)
    }

    async fn analytics(
        &self,
        workspace_id: &str,
        filter: HumanReviewAnalyticsFilter,
    ) -> Result<tl_core::HumanReviewAnalyticsResponse, HumanReviewStoreError> {
        self.0
            .analytics(
                workspace_id,
                tl_storage::HumanReviewAnalyticsFilter {
                    agent_id: filter.agent_id,
                    policy_id: filter.policy_id,
                    run_kind: filter.run_kind,
                    workflow_step: filter.workflow_step,
                },
            )
            .await
            .map_err(human_review_store_error)
    }
}

fn human_review_store_error(error: tl_storage::StorageError) -> HumanReviewStoreError {
    match error {
        tl_storage::StorageError::NotFound => HumanReviewStoreError::NotFound,
        tl_storage::StorageError::Conflict => HumanReviewStoreError::Internal("conflict".into()),
        tl_storage::StorageError::Internal(message) if message.contains("parse") => {
            HumanReviewStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => HumanReviewStoreError::Internal(message),
    }
}
