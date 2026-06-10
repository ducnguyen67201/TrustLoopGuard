use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{ToolMetadata, ToolMetadataEntry};
use tl_storage::ToolMetadataRepo;

use crate::tool_metadata::{ToolMetadataStore, ToolMetadataStoreError};

/// Adapter newtype for implementing local traits around
/// `tl_storage::ToolMetadataRepo`. Implements both the control-plane
/// `ToolMetadataStore` and the engine's runtime `ToolMetadataProvider`.
pub struct PostgresToolMetadataAdapter(pub Arc<ToolMetadataRepo>);

impl PostgresToolMetadataAdapter {
    pub fn new(repo: Arc<ToolMetadataRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl ToolMetadataStore for PostgresToolMetadataAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        metadata: &ToolMetadata,
        enabled: bool,
    ) -> Result<(), ToolMetadataStoreError> {
        self.0
            .upsert(workspace_id, metadata, enabled)
            .await
            .map_err(|error| ToolMetadataStoreError::Internal(error.to_string()))
    }

    async fn get(
        &self,
        workspace_id: &str,
        tool: &str,
    ) -> Result<ToolMetadataEntry, ToolMetadataStoreError> {
        self.0
            .get(workspace_id, tool)
            .await
            .map(|stored| ToolMetadataEntry {
                metadata: stored.metadata.clone(),
                enabled: stored.enabled,
            })
            .map_err(tool_metadata_store_error)
    }

    async fn delete(&self, workspace_id: &str, tool: &str) -> Result<(), ToolMetadataStoreError> {
        self.0
            .delete(workspace_id, tool)
            .await
            .map_err(tool_metadata_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ToolMetadataEntry>, ToolMetadataStoreError> {
        self.0
            .list(workspace_id)
            .await
            .map(|rows| {
                rows.iter()
                    .map(|stored| ToolMetadataEntry {
                        metadata: stored.metadata.clone(),
                        enabled: stored.enabled,
                    })
                    .collect()
            })
            .map_err(|error| ToolMetadataStoreError::Internal(error.to_string()))
    }
}

#[async_trait]
impl tl_engine::ToolMetadataProvider for PostgresToolMetadataAdapter {
    async fn get(
        &self,
        workspace_id: &str,
        tool: &str,
    ) -> Result<Option<ToolMetadata>, tl_engine::ToolMetadataUnavailable> {
        match self.0.get(workspace_id, tool).await {
            Ok(stored) if stored.enabled => Ok(Some(stored.metadata.clone())),
            // Disabled tools resolve as unregistered at runtime.
            Ok(_) => Ok(None),
            Err(tl_storage::StorageError::NotFound) => Ok(None),
            Err(e) => {
                // Fail open: resolution is evidence, never a gate. The
                // pipeline records the outage as `resolution_failed` so a
                // storage error never masquerades as a missing registration.
                tracing::warn!(workspace_id, tool, error = %e, "tool metadata resolution failed");
                Err(tl_engine::ToolMetadataUnavailable)
            }
        }
    }
}

fn tool_metadata_store_error(error: tl_storage::StorageError) -> ToolMetadataStoreError {
    match error {
        tl_storage::StorageError::NotFound => ToolMetadataStoreError::NotFound,
        other => ToolMetadataStoreError::Internal(other.to_string()),
    }
}
