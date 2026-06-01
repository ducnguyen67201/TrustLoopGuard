//! Human review event and analytics endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{CreateHumanReviewEventRequest, HumanReviewAnalyticsResponse, HumanReviewEvent};

mod handlers;
mod memory_store;
mod query;
mod response;
mod validation;

pub use handlers::{
    __path_create_review_event, __path_human_review_analytics, __path_list_review_events,
    create_review_event, human_review_analytics, list_review_events,
};
pub use memory_store::MemoryHumanReviewStore;

#[derive(Debug, thiserror::Error)]
pub enum HumanReviewStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Default)]
pub struct HumanReviewAnalyticsFilter {
    pub agent_id: Option<String>,
    pub policy_id: Option<String>,
    pub run_kind: Option<String>,
    pub workflow_step: Option<String>,
}

#[async_trait]
pub trait HumanReviewStore: Send + Sync {
    async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<HumanReviewEvent, HumanReviewStoreError>;

    async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: usize,
    ) -> Result<Vec<HumanReviewEvent>, HumanReviewStoreError>;

    async fn analytics(
        &self,
        workspace_id: &str,
        filter: HumanReviewAnalyticsFilter,
    ) -> Result<HumanReviewAnalyticsResponse, HumanReviewStoreError>;
}

#[derive(Clone)]
pub struct HumanReviewState {
    pub store: Arc<dyn HumanReviewStore>,
}
