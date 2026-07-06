use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    Origin, PolicyFamily, Severity, SourceLabelPolicy, SourceLabelPolicyEntry,
    DEFAULT_ENVIRONMENT_ID,
};
use tl_policy::{AnyPolicy, FamilyPolicy, SourceLabelFamilyPolicy};
use tl_storage::PolicyRepo;

use crate::label_policy::{LabelPolicyStore, LabelPolicyStoreError};

/// Compatibility adapter for `/v1/label-policies` backed by the unified
/// policy registry (`policies.family = source_label`).
pub struct PostgresLabelPolicyAdapter(pub Arc<PolicyRepo>);

impl PostgresLabelPolicyAdapter {
    pub fn new(repo: Arc<PolicyRepo>) -> Arc<Self> {
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
        let family = source_label_family_policy(policy);
        let source_yaml = serde_yaml::to_string(&family)
            .map_err(|error| LabelPolicyStoreError::Internal(error.to_string()))?;
        self.0
            .upsert_family_in(workspace_id, &family, &source_yaml)
            .await
            .map_err(label_policy_store_error)?;
        self.0
            .set_enabled_in_environment(workspace_id, DEFAULT_ENVIRONMENT_ID, family.id(), enabled)
            .await
            .map_err(label_policy_store_error)
    }

    async fn get(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<SourceLabelPolicyEntry, LabelPolicyStoreError> {
        self.0
            .list_any_records_in_environment(
                workspace_id,
                DEFAULT_ENVIRONMENT_ID,
                Some(PolicyFamily::SourceLabel),
            )
            .await
            .map_err(label_policy_store_error)?
            .into_iter()
            .find_map(|row| source_label_entry(row.policy, row.enabled))
            .filter(|entry| entry.policy.origin == origin)
            .ok_or(LabelPolicyStoreError::NotFound)
    }

    async fn delete(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<(), LabelPolicyStoreError> {
        self.0
            .delete_in(workspace_id, &source_label_registry_id(origin))
            .await
            .map_err(label_policy_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicyEntry>, LabelPolicyStoreError> {
        let mut entries: Vec<_> = self
            .0
            .list_any_records_in_environment(
                workspace_id,
                DEFAULT_ENVIRONMENT_ID,
                Some(PolicyFamily::SourceLabel),
            )
            .await
            .map_err(label_policy_store_error)?
            .into_iter()
            .filter_map(|row| source_label_entry(row.policy, row.enabled))
            .collect();
        entries.sort_by_key(|entry| origin_key(entry.policy.origin));
        Ok(entries)
    }
}

#[async_trait]
impl tl_engine::LabelPolicyProvider for PostgresLabelPolicyAdapter {
    async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicy>, tl_engine::LabelPolicyUnavailable> {
        match self.list(workspace_id).await {
            Ok(entries) => Ok(entries
                .into_iter()
                .filter(|entry| entry.enabled)
                .map(|entry| entry.policy)
                .collect()),
            Err(error) => {
                tracing::warn!(
                    workspace_id,
                    error = %error,
                    "label policy resolution failed"
                );
                Err(tl_engine::LabelPolicyUnavailable)
            }
        }
    }
}

fn source_label_entry(policy: AnyPolicy, enabled: bool) -> Option<SourceLabelPolicyEntry> {
    let AnyPolicy::Family(FamilyPolicy::SourceLabel(policy)) = policy else {
        return None;
    };
    Some(SourceLabelPolicyEntry {
        policy: SourceLabelPolicy {
            origin: policy.origin,
            trust: policy.trust,
            confidentiality: policy.confidentiality,
            integrity: policy.integrity,
        },
        enabled,
    })
}

fn source_label_family_policy(policy: &SourceLabelPolicy) -> FamilyPolicy {
    FamilyPolicy::SourceLabel(SourceLabelFamilyPolicy {
        id: source_label_registry_id(policy.origin),
        description: Some(format!(
            "Source label override for {}",
            origin_key(policy.origin)
        )),
        severity: Severity::Low,
        origin: policy.origin,
        trust: policy.trust,
        confidentiality: policy.confidentiality,
        integrity: policy.integrity,
    })
}

fn source_label_registry_id(origin: Origin) -> String {
    format!("source-label-{}", origin_key(origin))
}

fn origin_key(origin: Origin) -> String {
    match serde_json::to_value(origin) {
        Ok(serde_json::Value::String(value)) => value,
        _ => "unknown".to_string(),
    }
}

fn label_policy_store_error(error: tl_storage::StorageError) -> LabelPolicyStoreError {
    match error {
        tl_storage::StorageError::NotFound => LabelPolicyStoreError::NotFound,
        other => LabelPolicyStoreError::Internal(other.to_string()),
    }
}
