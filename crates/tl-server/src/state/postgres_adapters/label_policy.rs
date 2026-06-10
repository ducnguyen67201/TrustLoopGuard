use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{Origin, SourceLabelPolicy, SourceLabelPolicyEntry};
use tl_storage::SourceLabelPolicyRepo;

use crate::label_policy::{LabelPolicyStore, LabelPolicyStoreError};

/// Adapter newtype for implementing local traits around
/// `tl_storage::SourceLabelPolicyRepo`. Implements both the
/// control-plane `LabelPolicyStore` and the engine's runtime
/// `LabelPolicyProvider`.
pub struct PostgresLabelPolicyAdapter(pub Arc<SourceLabelPolicyRepo>);

impl PostgresLabelPolicyAdapter {
    pub fn new(repo: Arc<SourceLabelPolicyRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl LabelPolicyStore for PostgresLabelPolicyAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        policy: &SourceLabelPolicy,
        enabled: bool,
    ) -> Result<(), LabelPolicyStoreError> {
        self.0
            .upsert(workspace_id, policy, enabled)
            .await
            .map_err(|error| LabelPolicyStoreError::Internal(error.to_string()))
    }

    async fn get(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<SourceLabelPolicyEntry, LabelPolicyStoreError> {
        self.0
            .get(workspace_id, origin)
            .await
            .map(|stored| SourceLabelPolicyEntry {
                policy: stored.policy.clone(),
                enabled: stored.enabled,
            })
            .map_err(label_policy_store_error)
    }

    async fn delete(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<(), LabelPolicyStoreError> {
        self.0
            .delete(workspace_id, origin)
            .await
            .map_err(label_policy_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicyEntry>, LabelPolicyStoreError> {
        self.0
            .list(workspace_id)
            .await
            .map(|rows| {
                rows.iter()
                    .map(|stored| SourceLabelPolicyEntry {
                        policy: stored.policy.clone(),
                        enabled: stored.enabled,
                    })
                    .collect()
            })
            .map_err(|error| LabelPolicyStoreError::Internal(error.to_string()))
    }
}

#[async_trait]
impl tl_engine::LabelPolicyProvider for PostgresLabelPolicyAdapter {
    async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicy>, tl_engine::LabelPolicyUnavailable> {
        match self.0.list(workspace_id).await {
            // Disabled policies are skipped at runtime but stay
            // manageable in the control plane.
            Ok(rows) => Ok(rows
                .iter()
                .filter(|r| r.enabled)
                .map(|r| r.policy.clone())
                .collect()),
            Err(e) => {
                // Fail open: label resolution is evidence, never a gate.
                // The resolver records the outage as
                // `policy_status: unavailable` so a storage error never
                // masquerades as "no policies configured".
                tracing::warn!(workspace_id, error = %e, "label policy resolution failed");
                Err(tl_engine::LabelPolicyUnavailable)
            }
        }
    }
}

fn label_policy_store_error(error: tl_storage::StorageError) -> LabelPolicyStoreError {
    match error {
        tl_storage::StorageError::NotFound => LabelPolicyStoreError::NotFound,
        other => LabelPolicyStoreError::Internal(other.to_string()),
    }
}
