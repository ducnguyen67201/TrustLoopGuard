use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_policy::{FamilyPolicy, Policy};
use tl_storage::PolicyRepo;

use crate::policies::{PolicyStore, PolicyStoreError};

pub struct PostgresPolicyAdapter(pub Arc<PolicyRepo>);

impl PostgresPolicyAdapter {
    pub fn new(repo: Arc<PolicyRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl PolicyStore for PostgresPolicyAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .upsert_in(workspace_id, policy, source_yaml)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))?;
        self.0
            .set_enabled_in_environment(workspace_id, environment_id, &policy.id, true)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))?;
        self.get(workspace_id, environment_id, &policy.id).await
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .list_records_in_environment(workspace_id, environment_id)
            .await
            .map_or_else(
                |e| {
                    Err(match e {
                        tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                        other => PolicyStoreError::Internal(other.to_string()),
                    })
                },
                |rows| {
                    let row = rows
                        .into_iter()
                        .find(|row| row.policy.id == policy_id)
                        .ok_or(PolicyStoreError::NotFound)?;
                    Ok(tl_core::PolicyDocument {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        enabled: row.enabled,
                        source_yaml: row.source_yaml,
                    })
                },
            )
    }

    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records_in_environment(workspace_id, environment_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| rows.into_iter().map(policy_summary_from_row).collect())
    }

    async fn list_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Arc<Policy>>, PolicyStoreError> {
        self.0
            .list_enabled_in_environment(workspace_id, environment_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn upsert_family(
        &self,
        workspace_id: &str,
        _environment_id: &str,
        policy: &FamilyPolicy,
        source_yaml: &str,
    ) -> Result<(), PolicyStoreError> {
        self.0
            .upsert_family_in(workspace_id, policy, source_yaml)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn list_enabled_families(
        &self,
        workspace_id: &str,
        _environment_id: &str,
    ) -> Result<Vec<Arc<FamilyPolicy>>, PolicyStoreError> {
        self.0
            .list_enabled_families_in(workspace_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .set_enabled_in_environment(workspace_id, environment_id, policy_id, enabled)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })?;
        self.get(workspace_id, environment_id, policy_id).await
    }

    async fn batch_set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .batch_set_enabled_in_environment(workspace_id, environment_id, policy_ids, enabled)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })
            .map(|rows| rows.into_iter().map(policy_summary_from_row).collect())
    }

    async fn delete(&self, workspace_id: &str, policy_id: &str) -> Result<(), PolicyStoreError> {
        self.0
            .delete_in(workspace_id, policy_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })
    }

    async fn list_for_agent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records_in_environment(workspace_id, environment_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .filter(|row| row.owner_agent_id.as_deref() == Some(agent_id))
                    .map(policy_summary_from_row)
                    .collect()
            })
    }

    async fn delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, PolicyStoreError> {
        self.0
            .soft_delete_for_agent(workspace_id, agent_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn list_versions(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<tl_core::EntityVersionListResponse, PolicyStoreError> {
        self.0
            .list_versions_in(workspace_id, policy_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| tl_core::EntityVersionListResponse {
                versions: rows
                    .into_iter()
                    .map(|r| tl_core::EntityVersionSummary {
                        version: r.version,
                        created_at: r.created_at.to_rfc3339(),
                    })
                    .collect(),
            })
    }

    async fn get_version(
        &self,
        workspace_id: &str,
        policy_id: &str,
        version: i32,
    ) -> Result<tl_core::EntityVersionDetail, PolicyStoreError> {
        self.0
            .get_version_in(workspace_id, policy_id, version)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })
            .map(|r| tl_core::EntityVersionDetail {
                version: r.version,
                content: r.content,
                created_at: r.created_at.to_rfc3339(),
            })
    }
}

fn policy_summary_from_row(row: tl_storage::PolicyRow) -> tl_core::PolicySummary {
    tl_core::PolicySummary {
        id: row.policy.id,
        description: row.policy.description,
        severity: row.policy.severity,
        action: Some(policy_action(&row.policy.action)),
        enabled: row.enabled,
        owner_agent_id: row.owner_agent_id,
    }
}

fn policy_action(action: &tl_policy::Action) -> String {
    match action {
        tl_policy::Action::Allow => "allow",
        tl_policy::Action::Block => "block",
        tl_policy::Action::Rewrite => "rewrite",
        tl_policy::Action::Escalate => "escalate",
    }
    .to_string()
}
