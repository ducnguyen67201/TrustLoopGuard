use std::sync::Arc;

use async_trait::async_trait;
use tl_core::AgentProfile;
use tokio::sync::RwLock;

use super::{AgentStore, AgentStoreError};

/// Process-local agent store. Useful for local dev, tests, and the
/// "no database configured" boot path. Not durable across restarts.
#[derive(Debug, Default)]
pub struct MemoryAgentStore {
    inner: RwLock<std::collections::HashMap<(String, String), Arc<AgentProfile>>>,
}

impl MemoryAgentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AgentStore for MemoryAgentStore {
    async fn upsert(
        &self,
        workspace_id: &str,
        profile: &AgentProfile,
        _source_yaml: &str,
    ) -> Result<(), AgentStoreError> {
        self.inner.write().await.insert(
            (workspace_id.to_string(), profile.agent_id.clone()),
            Arc::new(profile.clone()),
        );
        Ok(())
    }

    async fn get(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Arc<AgentProfile>, AgentStoreError> {
        self.inner
            .read()
            .await
            .get(&(workspace_id.to_string(), agent_id.to_string()))
            .cloned()
            .ok_or(AgentStoreError::NotFound)
    }

    async fn delete(&self, workspace_id: &str, agent_id: &str) -> Result<(), AgentStoreError> {
        if self
            .inner
            .write()
            .await
            .remove(&(workspace_id.to_string(), agent_id.to_string()))
            .is_none()
        {
            return Err(AgentStoreError::NotFound);
        }
        Ok(())
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError> {
        let mut all: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, profile)| profile.clone())
            .collect();
        all.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
        Ok(all)
    }
}
