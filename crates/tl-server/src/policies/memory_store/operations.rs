use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{EntityVersionDetail, EntityVersionListResponse, PolicyDocument, PolicySummary};
use tl_policy::{FamilyPolicy, Policy};

use super::{MemoryPolicyRecord, MemoryPolicyStore};
use crate::policies::{policy_document, policy_summary, PolicyStore, PolicyStoreError};

#[async_trait]
impl PolicyStore for MemoryPolicyStore {
    async fn upsert(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let record = MemoryPolicyRecord {
            policy: policy.clone(),
            source_yaml: source_yaml.to_string(),
        };
        self.inner.write().await.insert(
            (workspace_id.to_string(), policy.id.clone()),
            record.clone(),
        );
        self.deployments.write().await.insert(
            (
                workspace_id.to_string(),
                environment_id.to_string(),
                policy.id.clone(),
            ),
            true,
        );
        Ok(policy_document(&record.policy, &record.source_yaml, true))
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let guard = self.inner.read().await;
        let record = guard
            .get(&(workspace_id.to_string(), policy_id.to_string()))
            .ok_or(PolicyStoreError::NotFound)?;
        let enabled = self
            .deployments
            .read()
            .await
            .get(&(
                workspace_id.to_string(),
                environment_id.to_string(),
                policy_id.to_string(),
            ))
            .copied()
            .unwrap_or(false);
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            enabled,
        ))
    }

    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError> {
        let deployments = self.deployments.read().await;
        let mut policies: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|((_, policy_id), record)| {
                let enabled = deployments
                    .get(&(
                        workspace_id.to_string(),
                        environment_id.to_string(),
                        policy_id.clone(),
                    ))
                    .copied()
                    .unwrap_or(false);
                policy_summary(&record.policy, enabled)
            })
            .collect();
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn list_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Arc<Policy>>, PolicyStoreError> {
        let deployments = self.deployments.read().await;
        let mut policies: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .filter(|((_, policy_id), _)| {
                deployments
                    .get(&(
                        workspace_id.to_string(),
                        environment_id.to_string(),
                        policy_id.clone(),
                    ))
                    .copied()
                    .unwrap_or(false)
            })
            .map(|(_, record)| Arc::new(record.policy.clone()))
            .collect();
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn upsert_family(
        &self,
        workspace_id: &str,
        _environment_id: &str,
        policy: &FamilyPolicy,
        _source_yaml: &str,
    ) -> Result<(), PolicyStoreError> {
        self.families.write().await.insert(
            (workspace_id.to_string(), policy.id().to_string()),
            Arc::new(policy.clone()),
        );
        Ok(())
    }

    async fn list_enabled_families(
        &self,
        workspace_id: &str,
        _environment_id: &str,
    ) -> Result<Vec<Arc<FamilyPolicy>>, PolicyStoreError> {
        let mut families: Vec<_> = self
            .families
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, p)| p.clone())
            .collect();
        families.sort_by(|a, b| a.id().cmp(b.id()));
        Ok(families)
    }

    async fn set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let guard = self.inner.read().await;
        let record = guard
            .get(&(workspace_id.to_string(), policy_id.to_string()))
            .ok_or(PolicyStoreError::NotFound)?;
        self.deployments.write().await.insert(
            (
                workspace_id.to_string(),
                environment_id.to_string(),
                policy_id.to_string(),
            ),
            enabled,
        );
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            enabled,
        ))
    }

    async fn batch_set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError> {
        let guard = self.inner.read().await;
        let workspace = workspace_id.to_string();
        if policy_ids
            .iter()
            .any(|id| !guard.contains_key(&(workspace.clone(), id.to_string())))
        {
            return Err(PolicyStoreError::NotFound);
        }

        let mut policies = Vec::with_capacity(policy_ids.len());
        for id in policy_ids {
            let record = guard
                .get(&(workspace.clone(), id.to_string()))
                .ok_or(PolicyStoreError::NotFound)?;
            self.deployments.write().await.insert(
                (
                    workspace.clone(),
                    environment_id.to_string(),
                    id.to_string(),
                ),
                enabled,
            );
            policies.push(policy_summary(&record.policy, enabled));
        }
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn delete(&self, workspace_id: &str, policy_id: &str) -> Result<(), PolicyStoreError> {
        if self
            .inner
            .write()
            .await
            .remove(&(workspace_id.to_string(), policy_id.to_string()))
            .is_none()
        {
            return Err(PolicyStoreError::NotFound);
        }
        Ok(())
    }

    async fn list_for_agent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError> {
        let deployments = self.deployments.read().await;
        let mut owned: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, record)| record)
            .filter(|record| record.policy.owner_agent_id.as_deref() == Some(agent_id))
            .map(|record| {
                let enabled = deployments
                    .get(&(
                        workspace_id.to_string(),
                        environment_id.to_string(),
                        record.policy.id.clone(),
                    ))
                    .copied()
                    .unwrap_or(false);
                policy_summary(&record.policy, enabled)
            })
            .collect();
        owned.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(owned)
    }

    async fn delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, PolicyStoreError> {
        // Memory store has no soft-delete state, so cascade = remove.
        // Matches the Postgres-side semantics from the caller's point of view:
        // deleted rows no longer surface in list_for_agent.
        let mut guard = self.inner.write().await;
        let owned_keys: Vec<(String, String)> = guard
            .iter()
            .filter(|((workspace, _), record)| {
                workspace == workspace_id
                    && record.policy.owner_agent_id.as_deref() == Some(agent_id)
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in &owned_keys {
            guard.remove(key);
        }
        Ok(owned_keys.into_iter().map(|(_, id)| id).collect())
    }

    async fn list_versions(
        &self,
        _workspace_id: &str,
        _policy_id: &str,
    ) -> Result<EntityVersionListResponse, PolicyStoreError> {
        Ok(EntityVersionListResponse { versions: vec![] })
    }

    async fn get_version(
        &self,
        _workspace_id: &str,
        _policy_id: &str,
        _version: i32,
    ) -> Result<EntityVersionDetail, PolicyStoreError> {
        Err(PolicyStoreError::NotFound)
    }
}
