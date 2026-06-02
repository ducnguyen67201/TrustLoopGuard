use std::sync::Arc;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_policy::Policy;

use super::{PolicyRepo, PolicyRow};
use crate::{models::PolicyRecord, schema::policies, StorageError};

impl PolicyRepo {
    /// Full authoring record for API/editor views.
    pub async fn get_record(&self, policy_id: &str) -> Result<PolicyRow, StorageError> {
        self.get_record_in(tl_core::DEFAULT_WORKSPACE_ID, policy_id)
            .await
    }

    pub async fn get_record_in(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<PolicyRow, StorageError> {
        let mut conn = self.connection().await?;
        let row = policies::table
            .filter(policies::workspace_id.eq(workspace_id))
            .filter(policies::id.eq(policy_id))
            .filter(policies::deleted_at.is_null())
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
            ))
            .first::<PolicyRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("policy record get: {e}")))?;

        row.map(policy_row_from_record)
            .transpose()?
            .ok_or(StorageError::NotFound)
    }

    /// All non-deleted policies. Bypasses the cache because this is an
    /// admin/editor path, not the hot path.
    pub async fn list(&self) -> Result<Vec<Arc<Policy>>, StorageError> {
        self.list_in(tl_core::DEFAULT_WORKSPACE_ID).await
    }

    pub async fn list_in(&self, workspace_id: &str) -> Result<Vec<Arc<Policy>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::workspace_id.eq(workspace_id))
            .filter(policies::deleted_at.is_null())
            .select(policies::parsed_policy)
            .order(policies::id.asc())
            .load::<serde_json::Value>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy list: {e}")))?;
        rows.into_iter().map(policy_from_json).collect()
    }

    /// All non-deleted authoring records. Bypasses the cache.
    pub async fn list_records(&self) -> Result<Vec<PolicyRow>, StorageError> {
        self.list_records_in(tl_core::DEFAULT_WORKSPACE_ID).await
    }

    pub async fn list_records_in(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PolicyRow>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::workspace_id.eq(workspace_id))
            .filter(policies::deleted_at.is_null())
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
            ))
            .order(policies::id.asc())
            .load::<PolicyRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy record list: {e}")))?;
        rows.into_iter().map(policy_row_from_record).collect()
    }

    /// All non-deleted policies owned by a given agent. Bypasses the
    /// cache - admin-list path, not the hot policy-resolution path.
    pub async fn list_records_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<PolicyRow>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::workspace_id.eq(workspace_id))
            .filter(policies::deleted_at.is_null())
            .filter(policies::owner_agent_id.eq(agent_id))
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
            ))
            .order(policies::id.asc())
            .load::<PolicyRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy by-agent list: {e}")))?;
        rows.into_iter().map(policy_row_from_record).collect()
    }

    /// Runtime policy set: active, enabled policies only.
    pub async fn list_enabled(&self) -> Result<Vec<Arc<Policy>>, StorageError> {
        self.list_enabled_in(tl_core::DEFAULT_WORKSPACE_ID).await
    }

    pub async fn list_enabled_in(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<Arc<Policy>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .filter(policies::workspace_id.eq(workspace_id))
            .filter(policies::deleted_at.is_null())
            .filter(policies::enabled.eq(true))
            .select((policies::parsed_policy, policies::policy_yaml))
            .order(policies::id.asc())
            .load::<(serde_json::Value, String)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("enabled policy list: {e}")))?;
        rows.into_iter()
            .map(|(parsed_policy, policy_yaml)| policy_from_storage(parsed_policy, &policy_yaml))
            .collect()
    }
}

pub(super) fn policy_row_from_record(record: PolicyRecord) -> Result<PolicyRow, StorageError> {
    Ok(PolicyRow {
        policy: serde_json::from_value(record.parsed_policy)
            .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))?,
        source_yaml: record.policy_yaml,
        enabled: record.enabled,
        owner_agent_id: record.owner_agent_id,
    })
}

fn policy_from_json(value: serde_json::Value) -> Result<Arc<Policy>, StorageError> {
    serde_json::from_value(value)
        .map(Arc::new)
        .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))
}

fn policy_from_storage(
    parsed_policy: serde_json::Value,
    policy_yaml: &str,
) -> Result<Arc<Policy>, StorageError> {
    match serde_yaml::from_str::<Policy>(policy_yaml) {
        Ok(policy) => Ok(Arc::new(policy)),
        Err(_) => policy_from_json(parsed_policy),
    }
}
