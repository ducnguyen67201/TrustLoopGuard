//! Dashboard runtime admin endpoints.

mod authorization;
pub(crate) mod handlers;
mod memory_store;
mod response;
mod settings;

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    DashboardApiKey, EnvironmentCheckerModes, UpdateWorkspaceSettingsRequest, WorkspaceSettings,
};
use uuid::Uuid;

use crate::environments::EnvironmentStore;
use crate::{auth::WorkspaceApiKeyVerifier, team::TeamStore};

pub use handlers::{
    batch_revoke_api_keys, create_api_key, get_environment_checker_modes, get_settings,
    list_api_keys, put_environment_checker_modes, update_settings,
};
pub use memory_store::{MemoryApiKeyStore, MemorySettingsStore};
pub use settings::default_settings;

#[derive(Debug, thiserror::Error)]
pub enum DashboardAdminStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait ApiKeyStore: WorkspaceApiKeyVerifier + Send + Sync {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError>;

    async fn create(&self, input: NewApiKey) -> Result<DashboardApiKey, DashboardAdminStoreError>;

    async fn batch_revoke(
        &self,
        workspace_id: &str,
        ids: &[String],
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError>;
}

#[derive(Debug, Clone)]
pub struct NewApiKey {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub created_by_user_id: Option<Uuid>,
    pub principal_id: Option<String>,
}

#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get(&self, workspace_id: &str) -> Result<WorkspaceSettings, DashboardAdminStoreError>;

    /// Apply a partial settings update and return the persisted settings.
    /// Default: unsupported, so read-only stores (tests, fixtures) keep
    /// implementing only `get`.
    async fn update(
        &self,
        workspace_id: &str,
        update: UpdateWorkspaceSettingsRequest,
    ) -> Result<WorkspaceSettings, DashboardAdminStoreError> {
        let _ = (workspace_id, update);
        Err(DashboardAdminStoreError::Internal(
            "settings updates are not supported by this store".into(),
        ))
    }

    /// Per-environment checker-mode overrides. Default: no override, so
    /// existing stores inherit workspace-level modes unchanged.
    async fn get_environment_modes(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Option<EnvironmentCheckerModes>, DashboardAdminStoreError> {
        let _ = (workspace_id, environment_id);
        Ok(None)
    }

    /// UPSERT per-environment checker-mode overrides. Must return
    /// [`DashboardAdminStoreError::NotFound`] for unknown environments.
    async fn put_environment_modes(
        &self,
        workspace_id: &str,
        environment_id: &str,
        modes: EnvironmentCheckerModes,
    ) -> Result<EnvironmentCheckerModes, DashboardAdminStoreError> {
        let _ = (workspace_id, environment_id, modes);
        Err(DashboardAdminStoreError::Internal(
            "environment checker-mode overrides are not supported by this store".into(),
        ))
    }
}

#[derive(Clone)]
pub struct DashboardAdminState {
    pub api_key_store: Arc<dyn ApiKeyStore>,
    pub settings_store: Arc<dyn SettingsStore>,
    pub team_store: Arc<dyn TeamStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
}
