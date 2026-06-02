use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_storage::EnvironmentRepo;

use crate::environments::{EnvironmentStore, EnvironmentStoreError};

pub struct PostgresEnvironmentAdapter(pub Arc<EnvironmentRepo>);

impl PostgresEnvironmentAdapter {
    pub fn new(repo: Arc<EnvironmentRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl EnvironmentStore for PostgresEnvironmentAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::WorkspaceEnvironment>, EnvironmentStoreError> {
        self.0
            .list(workspace_id)
            .await
            .map_err(environment_store_error)
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<tl_core::WorkspaceEnvironment, EnvironmentStoreError> {
        self.0
            .get(workspace_id, environment_id)
            .await
            .map_err(environment_store_error)
    }

    async fn default_environment_id(
        &self,
        workspace_id: &str,
    ) -> Result<String, EnvironmentStoreError> {
        self.0
            .default_environment_id(workspace_id)
            .await
            .map_err(environment_store_error)
    }

    async fn create(
        &self,
        workspace_id: &str,
        input: tl_core::CreateWorkspaceEnvironmentRequest,
    ) -> Result<tl_core::WorkspaceEnvironment, EnvironmentStoreError> {
        self.0
            .create(workspace_id, input)
            .await
            .map_err(environment_store_error)
    }

    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: tl_core::UpdateWorkspaceEnvironmentRequest,
    ) -> Result<tl_core::WorkspaceEnvironment, EnvironmentStoreError> {
        self.0
            .update(workspace_id, environment_id, input)
            .await
            .map_err(environment_store_error)
    }

    async fn delete(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<(), EnvironmentStoreError> {
        self.0
            .delete(workspace_id, environment_id)
            .await
            .map_err(environment_store_error)
    }
}

fn environment_store_error(error: tl_storage::StorageError) -> EnvironmentStoreError {
    match error {
        tl_storage::StorageError::NotFound => EnvironmentStoreError::NotFound,
        tl_storage::StorageError::Conflict => {
            EnvironmentStoreError::Validation("environment conflicts with an existing row".into())
        }
        tl_storage::StorageError::Internal(message)
            if message.contains("environment is still referenced")
                || message.contains("default environment cannot be deleted")
                || message.contains("workspace must have one default environment") =>
        {
            EnvironmentStoreError::Validation(message)
        }
        other => EnvironmentStoreError::Internal(other.to_string()),
    }
}
