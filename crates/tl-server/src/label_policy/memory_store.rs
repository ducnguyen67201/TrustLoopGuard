use async_trait::async_trait;
use tl_core::{Origin, SourceLabelPolicy, SourceLabelPolicyEntry};
use tokio::sync::RwLock;

use super::{LabelPolicyStore, LabelPolicyStoreError};

/// Process-local label policy registry. Useful for local dev, tests,
/// and the "no database configured" boot path. Not durable across
/// restarts. Implements both the CRUD store and the engine's runtime
/// `LabelPolicyProvider` so one instance backs the control plane and
/// the event pipeline in memory mode.
#[derive(Debug, Default)]
pub struct MemoryLabelPolicyStore {
    inner: RwLock<std::collections::HashMap<(String, String), SourceLabelPolicyEntry>>,
}

impl MemoryLabelPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Origins key the map by their serde snake_case name so memory and
/// Postgres stores agree on identity.
fn origin_key(origin: Origin) -> String {
    match serde_json::to_value(origin) {
        Ok(serde_json::Value::String(s)) => s,
        _ => "unknown".to_string(),
    }
}

#[async_trait]
impl LabelPolicyStore for MemoryLabelPolicyStore {
    async fn upsert(
        &self,
        workspace_id: &str,
        policy: &SourceLabelPolicy,
        enabled: bool,
    ) -> Result<(), LabelPolicyStoreError> {
        self.inner.write().await.insert(
            (workspace_id.to_string(), origin_key(policy.origin)),
            SourceLabelPolicyEntry {
                policy: policy.clone(),
                enabled,
            },
        );
        Ok(())
    }

    async fn get(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<SourceLabelPolicyEntry, LabelPolicyStoreError> {
        self.inner
            .read()
            .await
            .get(&(workspace_id.to_string(), origin_key(origin)))
            .cloned()
            .ok_or(LabelPolicyStoreError::NotFound)
    }

    async fn delete(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<(), LabelPolicyStoreError> {
        if self
            .inner
            .write()
            .await
            .remove(&(workspace_id.to_string(), origin_key(origin)))
            .is_none()
        {
            return Err(LabelPolicyStoreError::NotFound);
        }
        Ok(())
    }

    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicyEntry>, LabelPolicyStoreError> {
        let mut all: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, entry)| entry.clone())
            .collect();
        all.sort_by_key(|entry| origin_key(entry.policy.origin));
        Ok(all)
    }
}

#[async_trait]
impl tl_engine::LabelPolicyProvider for MemoryLabelPolicyStore {
    async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicy>, tl_engine::LabelPolicyUnavailable> {
        Ok(self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), entry)| workspace == workspace_id && entry.enabled)
            .map(|(_, entry)| entry.policy.clone())
            .collect())
    }
}
