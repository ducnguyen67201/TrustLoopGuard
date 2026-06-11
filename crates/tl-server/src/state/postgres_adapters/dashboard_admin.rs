use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_storage::DashboardAdminRepo;

use crate::auth::{WorkspaceApiKeyVerifier, WorkspaceApiKeyVerifyError, WorkspaceKeyContext};
use crate::dashboard_admin::{ApiKeyStore, DashboardAdminStoreError, NewApiKey, SettingsStore};

pub struct PostgresDashboardAdminAdapter(pub Arc<DashboardAdminRepo>);

impl PostgresDashboardAdminAdapter {
    pub fn new(repo: Arc<DashboardAdminRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl ApiKeyStore for PostgresDashboardAdminAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::DashboardApiKey>, DashboardAdminStoreError> {
        self.0
            .list_api_keys(workspace_id)
            .await
            .map_err(|error| DashboardAdminStoreError::Internal(error.to_string()))
    }

    async fn create(
        &self,
        input: NewApiKey,
    ) -> Result<tl_core::DashboardApiKey, DashboardAdminStoreError> {
        self.0
            .create_api_key(
                &input.id,
                &input.workspace_id,
                &input.environment_id,
                &input.name,
                &input.key_prefix,
                &input.key_hash,
                input.created_by_user_id,
            )
            .await
            .map_err(|error| DashboardAdminStoreError::Internal(error.to_string()))
    }

    async fn batch_revoke(
        &self,
        workspace_id: &str,
        ids: &[String],
    ) -> Result<Vec<tl_core::DashboardApiKey>, DashboardAdminStoreError> {
        self.0
            .batch_revoke_api_keys(workspace_id, ids)
            .await
            .map_err(dashboard_admin_store_error)
    }
}

#[async_trait]
impl WorkspaceApiKeyVerifier for PostgresDashboardAdminAdapter {
    async fn verify_workspace_api_key(
        &self,
        key_hash: &str,
    ) -> Result<Option<WorkspaceKeyContext>, WorkspaceApiKeyVerifyError> {
        self.0
            .verify_api_key_hash(key_hash)
            .await
            .map(|row| {
                row.map(|row| WorkspaceKeyContext {
                    api_key_id: row.id,
                    workspace_id: row.workspace_id,
                    environment_id: row.environment_id,
                })
            })
            .map_err(|error| WorkspaceApiKeyVerifyError::Internal(error.to_string()))
    }
}

#[async_trait]
impl SettingsStore for PostgresDashboardAdminAdapter {
    async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::WorkspaceSettings, DashboardAdminStoreError> {
        self.0
            .get_settings(workspace_id)
            .await
            .map_err(|error| DashboardAdminStoreError::Internal(error.to_string()))
            .map(|settings| settings.unwrap_or_else(crate::dashboard_admin::default_settings))
    }

    async fn update(
        &self,
        workspace_id: &str,
        update: tl_core::UpdateWorkspaceSettingsRequest,
    ) -> Result<tl_core::WorkspaceSettings, DashboardAdminStoreError> {
        let current = SettingsStore::get(self, workspace_id).await?;
        let merged = crate::dashboard_admin::apply_settings_update(&current, &update);
        self.0
            .put_settings(workspace_id, &merged)
            .await
            .map_err(dashboard_admin_store_error)
    }

    async fn get_environment_modes(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Option<tl_core::EnvironmentCheckerModes>, DashboardAdminStoreError> {
        self.0
            .get_environment_checker_modes(workspace_id, environment_id)
            .await
            .map_err(dashboard_admin_store_error)
    }

    async fn put_environment_modes(
        &self,
        workspace_id: &str,
        environment_id: &str,
        modes: tl_core::EnvironmentCheckerModes,
    ) -> Result<tl_core::EnvironmentCheckerModes, DashboardAdminStoreError> {
        self.0
            .put_environment_checker_modes(workspace_id, environment_id, &modes)
            .await
            .map_err(dashboard_admin_store_error)
    }
}

fn dashboard_admin_store_error(error: tl_storage::StorageError) -> DashboardAdminStoreError {
    match error {
        tl_storage::StorageError::NotFound => DashboardAdminStoreError::NotFound,
        other => DashboardAdminStoreError::Internal(other.to_string()),
    }
}
