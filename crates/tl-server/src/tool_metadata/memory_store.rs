use async_trait::async_trait;
use tl_core::{ToolMetadata, ToolMetadataEntry};
use tokio::sync::RwLock;

use super::{ToolMetadataStore, ToolMetadataStoreError};

/// Process-local tool metadata registry. Useful for local dev, tests,
/// and the "no database configured" boot path. Not durable across
/// restarts. Implements both the CRUD store and the engine's runtime
/// `ToolMetadataProvider` so one instance backs the control plane and
/// the event pipeline in memory mode.
#[derive(Debug, Default)]
pub struct MemoryToolMetadataStore {
    inner: RwLock<std::collections::HashMap<(String, String), ToolMetadataEntry>>,
}

impl MemoryToolMetadataStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ToolMetadataStore for MemoryToolMetadataStore {
    async fn upsert(
        &self,
        workspace_id: &str,
        metadata: &ToolMetadata,
        enabled: bool,
    ) -> Result<(), ToolMetadataStoreError> {
        self.inner.write().await.insert(
            (workspace_id.to_string(), metadata.tool.clone()),
            ToolMetadataEntry {
                metadata: metadata.clone(),
                enabled,
            },
        );
        Ok(())
    }

    async fn get(
        &self,
        workspace_id: &str,
        tool: &str,
    ) -> Result<ToolMetadataEntry, ToolMetadataStoreError> {
        self.inner
            .read()
            .await
            .get(&(workspace_id.to_string(), tool.to_string()))
            .cloned()
            .ok_or(ToolMetadataStoreError::NotFound)
    }

    async fn delete(&self, workspace_id: &str, tool: &str) -> Result<(), ToolMetadataStoreError> {
        if self
            .inner
            .write()
            .await
            .remove(&(workspace_id.to_string(), tool.to_string()))
            .is_none()
        {
            return Err(ToolMetadataStoreError::NotFound);
        }
        Ok(())
    }

    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ToolMetadataEntry>, ToolMetadataStoreError> {
        let mut all: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, entry)| entry.clone())
            .collect();
        all.sort_by(|a, b| a.metadata.tool.cmp(&b.metadata.tool));
        Ok(all)
    }
}

#[async_trait]
impl tl_engine::ToolMetadataProvider for MemoryToolMetadataStore {
    async fn get(
        &self,
        workspace_id: &str,
        tool: &str,
    ) -> Result<Option<ToolMetadata>, tl_engine::ToolMetadataUnavailable> {
        Ok(self
            .inner
            .read()
            .await
            .get(&(workspace_id.to_string(), tool.to_string()))
            .filter(|entry| entry.enabled)
            .map(|entry| entry.metadata.clone()))
    }
}
