use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_core::AgentProfile;
use tl_engine::ProfileResolver;
use tl_storage::AgentRepo;

use crate::agents::{AgentStore, AgentStoreError};

/// Adapter newtype for implementing local traits around `tl_storage::AgentRepo`.
pub struct PostgresAgentAdapter(pub Arc<AgentRepo>);

impl PostgresAgentAdapter {
    pub fn new(repo: Arc<AgentRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl ProfileResolver for PostgresAgentAdapter {
    async fn resolve(&self, workspace_id: &str, agent_id: &str) -> Option<Arc<AgentProfile>> {
        self.0.get(workspace_id, agent_id).await.ok()
    }
}

#[async_trait]
impl AgentStore for PostgresAgentAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        profile: &AgentProfile,
        source_yaml: &str,
    ) -> Result<(), AgentStoreError> {
        self.0
            .upsert(workspace_id, profile, source_yaml)
            .await
            .map_err(|error| AgentStoreError::Internal(error.to_string()))
    }

    async fn get(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Arc<AgentProfile>, AgentStoreError> {
        self.0
            .get(workspace_id, agent_id)
            .await
            .map_err(agent_store_error)
    }

    async fn delete(&self, workspace_id: &str, agent_id: &str) -> Result<(), AgentStoreError> {
        self.0
            .delete(workspace_id, agent_id)
            .await
            .map_err(agent_store_error)
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError> {
        self.0
            .list(workspace_id)
            .await
            .map_err(|error| AgentStoreError::Internal(error.to_string()))
    }
}

fn agent_store_error(error: tl_storage::StorageError) -> AgentStoreError {
    match error {
        tl_storage::StorageError::NotFound => AgentStoreError::NotFound,
        other => AgentStoreError::Internal(other.to_string()),
    }
}
