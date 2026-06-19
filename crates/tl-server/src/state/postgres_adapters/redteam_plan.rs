use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{AttackVector, RedteamPlanResponse, WorkflowPath};
use tl_storage::{RedteamPlanRepo, StorageError};

use crate::redteam::{RedteamPlanStore, RedteamPlanStoreError};

/// Bridges the `tl_storage::RedteamPlanRepo` to the server-side
/// `RedteamPlanStore` trait. Mirrors `PostgresRedteamJobAdapter`.
pub struct PostgresRedteamPlanAdapter(pub Arc<RedteamPlanRepo>);

impl PostgresRedteamPlanAdapter {
    pub fn new(repo: Arc<RedteamPlanRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl RedteamPlanStore for PostgresRedteamPlanAdapter {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        name: &str,
        vectors: Vec<AttackVector>,
        paths: Vec<WorkflowPath>,
        unmapped_node_types: Vec<String>,
    ) -> Result<RedteamPlanResponse, RedteamPlanStoreError> {
        self.0
            .create(
                workspace_id,
                environment_id,
                agent_id,
                name,
                vectors,
                paths,
                unmapped_node_types,
            )
            .await
            .map_err(plan_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<RedteamPlanResponse>, RedteamPlanStoreError> {
        self.0
            .list(workspace_id, agent_id, limit as i64)
            .await
            .map_err(plan_store_error)
    }

    async fn delete(&self, workspace_id: &str, plan_id: &str) -> Result<(), RedteamPlanStoreError> {
        self.0
            .delete(workspace_id, plan_id)
            .await
            .map_err(plan_store_error)
    }
}

fn plan_store_error(error: StorageError) -> RedteamPlanStoreError {
    match error {
        StorageError::NotFound => RedteamPlanStoreError::NotFound,
        other => RedteamPlanStoreError::Internal(other.to_string()),
    }
}
