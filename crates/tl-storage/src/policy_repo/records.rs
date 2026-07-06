use std::sync::Arc;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::PolicyFamily;
use tl_policy::{AnyPolicy, FamilyPolicy, Policy};

use super::{AnyPolicyRow, PolicyRepo, PolicyRow};
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
                policies::family,
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
            .filter(
                policies::family
                    .is_null()
                    .or(policies::family.eq("content")),
            )
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
            .filter(
                policies::family
                    .is_null()
                    .or(policies::family.eq("content")),
            )
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
                policies::family,
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
            .filter(
                policies::family
                    .is_null()
                    .or(policies::family.eq("content")),
            )
            .filter(policies::owner_agent_id.eq(agent_id))
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
                policies::family,
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
            .filter(
                policies::family
                    .is_null()
                    .or(policies::family.eq("content")),
            )
            .select((policies::parsed_policy, policies::policy_yaml))
            .order(policies::id.asc())
            .load::<(serde_json::Value, String)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("enabled policy list: {e}")))?;
        rows.into_iter()
            .map(|(parsed_policy, policy_yaml)| policy_from_storage(parsed_policy, &policy_yaml))
            .collect()
    }

    /// All non-deleted authoring records across all policy families.
    pub async fn list_any_records_in(
        &self,
        workspace_id: &str,
        family: Option<PolicyFamily>,
    ) -> Result<Vec<AnyPolicyRow>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = policies::table
            .filter(policies::workspace_id.eq(workspace_id))
            .filter(policies::deleted_at.is_null())
            .into_boxed();

        if let Some(family) = family {
            query = match family {
                PolicyFamily::Content => query.filter(
                    policies::family
                        .is_null()
                        .or(policies::family.eq(family.as_str())),
                ),
                other => query.filter(policies::family.eq(other.as_str())),
            };
        }

        let rows = query
            .select((
                policies::parsed_policy,
                policies::policy_yaml,
                policies::enabled,
                policies::owner_agent_id,
                policies::family,
            ))
            .order(policies::id.asc())
            .load::<PolicyRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy any record list: {e}")))?;

        rows.into_iter().map(any_policy_row_from_record).collect()
    }
}

pub(super) fn policy_row_from_record(record: PolicyRecord) -> Result<PolicyRow, StorageError> {
    let family = policy_family_from_storage(record.family.as_deref())?;
    if family != PolicyFamily::Content {
        return Err(StorageError::Internal(format!(
            "expected content policy, got `{family}`"
        )));
    }
    Ok(PolicyRow {
        policy: serde_json::from_value(record.parsed_policy)
            .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))?,
        source_yaml: record.policy_yaml,
        enabled: record.enabled,
        owner_agent_id: record.owner_agent_id,
    })
}

pub(super) fn any_policy_row_from_record(
    record: PolicyRecord,
) -> Result<AnyPolicyRow, StorageError> {
    let family = policy_family_from_storage(record.family.as_deref())?;
    let policy = match family {
        PolicyFamily::Content => {
            let policy: Policy = serde_json::from_value(record.parsed_policy)
                .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))?;
            AnyPolicy::Content(policy)
        }
        _ => {
            let policy: FamilyPolicy = serde_json::from_value(record.parsed_policy)
                .map_err(|e| StorageError::Internal(format!("family deserialize: {e}")))?;
            AnyPolicy::Family(policy)
        }
    };
    Ok(AnyPolicyRow {
        policy,
        family,
        source_yaml: record.policy_yaml,
        enabled: record.enabled,
        owner_agent_id: record.owner_agent_id,
    })
}

pub(super) fn policy_family_from_storage(raw: Option<&str>) -> Result<PolicyFamily, StorageError> {
    match raw.unwrap_or("content") {
        "content" => Ok(PolicyFamily::Content),
        "flow" => Ok(PolicyFamily::Flow),
        "parameter_source" => Ok(PolicyFamily::ParameterSource),
        "approval" => Ok(PolicyFamily::Approval),
        "memory" => Ok(PolicyFamily::Memory),
        "financial" => Ok(PolicyFamily::Financial),
        "source_label" => Ok(PolicyFamily::SourceLabel),
        other => Err(StorageError::Internal(format!(
            "unknown policy family `{other}`"
        ))),
    }
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
