use std::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;
use tl_core::DashboardApiKey;
use uuid::Uuid;

use crate::auth::{WorkspaceApiKeyVerifier, WorkspaceApiKeyVerifyError, WorkspaceKeyContext};

use super::{default_settings, ApiKeyStore, DashboardAdminStoreError, NewApiKey, SettingsStore};

#[derive(Debug, Default)]
pub struct MemoryApiKeyStore {
    keys: RwLock<Vec<MemoryApiKeyRecord>>,
}

#[derive(Debug, Clone)]
struct MemoryApiKeyRecord {
    id: String,
    workspace_id: String,
    environment_id: String,
    name: String,
    key_prefix: String,
    key_hash: String,
    status: String,
    created_by_user_id: Option<Uuid>,
    created_at: String,
    last_used_at: Option<String>,
    revoked_at: Option<String>,
}

impl MemoryApiKeyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ApiKeyStore for MemoryApiKeyStore {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError> {
        let keys = self
            .keys
            .read()
            .map_err(|_| DashboardAdminStoreError::Internal("api key lock poisoned".into()))?;
        Ok(keys
            .iter()
            .filter(|key| key.workspace_id == workspace_id)
            .map(memory_api_key_to_wire)
            .collect())
    }

    async fn create(&self, input: NewApiKey) -> Result<DashboardApiKey, DashboardAdminStoreError> {
        let record = MemoryApiKeyRecord {
            id: input.id,
            workspace_id: input.workspace_id,
            environment_id: input.environment_id,
            name: input.name,
            key_prefix: input.key_prefix,
            key_hash: input.key_hash,
            status: "active".to_string(),
            created_by_user_id: input.created_by_user_id,
            created_at: Utc::now().to_rfc3339(),
            last_used_at: None,
            revoked_at: None,
        };
        let wire = memory_api_key_to_wire(&record);
        let mut keys = self
            .keys
            .write()
            .map_err(|_| DashboardAdminStoreError::Internal("api key lock poisoned".into()))?;
        keys.push(record);
        Ok(wire)
    }

    async fn batch_revoke(
        &self,
        workspace_id: &str,
        ids: &[String],
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError> {
        let ids = normalize_ids(ids);
        let mut keys = self
            .keys
            .write()
            .map_err(|_| DashboardAdminStoreError::Internal("api key lock poisoned".into()))?;
        if ids.iter().any(|id| {
            !keys
                .iter()
                .any(|key| key.workspace_id == workspace_id && key.id == *id)
        }) {
            return Err(DashboardAdminStoreError::NotFound);
        }

        let revoked_at = Utc::now().to_rfc3339();
        for key in keys
            .iter_mut()
            .filter(|key| key.workspace_id == workspace_id && ids.iter().any(|id| id == &key.id))
        {
            key.status = "revoked".to_string();
            key.revoked_at = Some(revoked_at.clone());
        }

        Ok(keys
            .iter()
            .filter(|key| key.workspace_id == workspace_id && ids.iter().any(|id| id == &key.id))
            .map(memory_api_key_to_wire)
            .collect())
    }
}

#[async_trait]
impl WorkspaceApiKeyVerifier for MemoryApiKeyStore {
    async fn verify_workspace_api_key(
        &self,
        key_hash: &str,
    ) -> Result<Option<WorkspaceKeyContext>, WorkspaceApiKeyVerifyError> {
        let mut keys = self
            .keys
            .write()
            .map_err(|_| WorkspaceApiKeyVerifyError::Internal("api key lock poisoned".into()))?;
        let Some(key) = keys
            .iter_mut()
            .find(|key| key.key_hash == key_hash && key.status == "active")
        else {
            return Ok(None);
        };
        key.last_used_at = Some(Utc::now().to_rfc3339());
        Ok(Some(WorkspaceKeyContext {
            api_key_id: key.id.clone(),
            workspace_id: key.workspace_id.clone(),
            environment_id: key.environment_id.clone(),
        }))
    }
}

#[derive(Debug, Default)]
pub struct MemorySettingsStore;

#[async_trait]
impl SettingsStore for MemorySettingsStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<tl_core::WorkspaceSettings, DashboardAdminStoreError> {
        Ok(default_settings())
    }
}

fn normalize_ids(ids: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for id in ids {
        if !normalized.iter().any(|existing: &String| existing == id) {
            normalized.push(id.clone());
        }
    }
    normalized
}

fn memory_api_key_to_wire(row: &MemoryApiKeyRecord) -> DashboardApiKey {
    DashboardApiKey {
        id: row.id.clone(),
        name: row.name.clone(),
        prefix: row.key_prefix.clone(),
        environment_id: row.environment_id.clone(),
        environment: row.environment_id.clone(),
        status: row.status.clone(),
        created_at: row.created_at.clone(),
        last_used_at: row.last_used_at.clone(),
        created_by: row.created_by_user_id.map(|value| value.to_string()),
    }
}
