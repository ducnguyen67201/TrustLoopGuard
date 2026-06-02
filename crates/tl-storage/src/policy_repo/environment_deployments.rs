use std::sync::Arc;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_policy::Policy;

use crate::models::{NewPolicyEnvironmentDeployment, PolicyRecord};
use crate::schema::{policies, policy_environment_deployments};
use crate::StorageError;

use super::{records::policy_row_from_record, PolicyRepo, PolicyRow};

impl PolicyRepo {
    pub async fn list_records_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<PolicyRow>, StorageError> {
        let mut rows = self.list_records_in(workspace_id).await?;
        let mut conn = self.connection().await?;
        let deployments = policy_environment_deployments::table
            .filter(policy_environment_deployments::workspace_id.eq(workspace_id))
            .filter(policy_environment_deployments::environment_id.eq(environment_id))
            .select((
                policy_environment_deployments::policy_id,
                policy_environment_deployments::enabled,
            ))
            .load::<(String, bool)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy deployments list: {e}")))?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();

        for row in &mut rows {
            row.enabled = deployments.get(&row.policy.id).copied().unwrap_or(false);
        }
        Ok(rows)
    }

    pub async fn list_enabled_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Arc<Policy>>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = policies::table
            .inner_join(
                policy_environment_deployments::table.on(
                    policy_environment_deployments::workspace_id
                        .eq(policies::workspace_id)
                        .and(policy_environment_deployments::policy_id.eq(policies::id)),
                ),
            )
            .filter(policies::workspace_id.eq(workspace_id))
            .filter(policies::deleted_at.is_null())
            .filter(policy_environment_deployments::environment_id.eq(environment_id))
            .filter(policy_environment_deployments::enabled.eq(true))
            .select(policies::parsed_policy)
            .order(policies::id.asc())
            .load::<serde_json::Value>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("enabled policy deployments: {e}")))?;

        rows.into_iter()
            .map(|value| {
                serde_json::from_value(value)
                    .map(Arc::new)
                    .map_err(|e| StorageError::Internal(format!("policy deserialize: {e}")))
            })
            .collect()
    }

    pub async fn set_enabled_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<(), StorageError> {
        // Ensure the policy exists and is not deleted before creating deployment state.
        self.get_record_in(workspace_id, policy_id).await?;
        let mut conn = self.connection().await?;
        diesel::insert_into(policy_environment_deployments::table)
            .values(NewPolicyEnvironmentDeployment {
                workspace_id: workspace_id.to_string(),
                environment_id: environment_id.to_string(),
                policy_id: policy_id.to_string(),
                enabled,
                deployed_version: None,
            })
            .on_conflict((
                policy_environment_deployments::workspace_id,
                policy_environment_deployments::environment_id,
                policy_environment_deployments::policy_id,
            ))
            .do_update()
            .set((
                policy_environment_deployments::enabled
                    .eq(excluded(policy_environment_deployments::enabled)),
                policy_environment_deployments::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("policy deployment set enabled: {e}")))?;
        Ok(())
    }

    pub async fn batch_set_enabled_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<PolicyRow>, StorageError> {
        let ids = unique_policy_ids(policy_ids);
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            ensure_all_policies_exist(conn, workspace_id, &ids).await?;

            for policy_id in &ids {
                upsert_deployment(conn, workspace_id, environment_id, policy_id, enabled).await?;
            }

            load_deployment_records(conn, workspace_id, environment_id, &ids).await
        })
        .await
    }
}

fn unique_policy_ids(policy_ids: &[String]) -> Vec<String> {
    let mut ids = Vec::new();
    for id in policy_ids {
        if !ids.iter().any(|existing: &String| existing == id) {
            ids.push(id.clone());
        }
    }
    ids
}

async fn ensure_all_policies_exist(
    conn: &mut crate::postgres::DbConnection<'_>,
    workspace_id: &str,
    ids: &[String],
) -> Result<(), StorageError> {
    let existing = policies::table
        .filter(policies::workspace_id.eq(workspace_id))
        .filter(policies::id.eq_any(ids))
        .filter(policies::deleted_at.is_null())
        .select(policies::id)
        .load::<String>(conn)
        .await?;
    if existing.len() != ids.len() {
        return Err(StorageError::NotFound);
    }
    Ok(())
}

async fn upsert_deployment(
    conn: &mut crate::postgres::DbConnection<'_>,
    workspace_id: &str,
    environment_id: &str,
    policy_id: &str,
    enabled: bool,
) -> Result<(), StorageError> {
    diesel::insert_into(policy_environment_deployments::table)
        .values(NewPolicyEnvironmentDeployment {
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            policy_id: policy_id.to_string(),
            enabled,
            deployed_version: None,
        })
        .on_conflict((
            policy_environment_deployments::workspace_id,
            policy_environment_deployments::environment_id,
            policy_environment_deployments::policy_id,
        ))
        .do_update()
        .set((
            policy_environment_deployments::enabled
                .eq(excluded(policy_environment_deployments::enabled)),
            policy_environment_deployments::updated_at.eq(now),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

async fn load_deployment_records(
    conn: &mut crate::postgres::DbConnection<'_>,
    workspace_id: &str,
    environment_id: &str,
    ids: &[String],
) -> Result<Vec<PolicyRow>, StorageError> {
    let records = policies::table
        .inner_join(
            policy_environment_deployments::table.on(policy_environment_deployments::workspace_id
                .eq(policies::workspace_id)
                .and(policy_environment_deployments::policy_id.eq(policies::id))),
        )
        .filter(policies::workspace_id.eq(workspace_id))
        .filter(policies::id.eq_any(ids))
        .filter(policies::deleted_at.is_null())
        .filter(policy_environment_deployments::environment_id.eq(environment_id))
        .select((
            policies::parsed_policy,
            policies::policy_yaml,
            policy_environment_deployments::enabled,
            policies::owner_agent_id,
        ))
        .order(policies::id.asc())
        .load::<PolicyRecord>(conn)
        .await?;

    records.into_iter().map(policy_row_from_record).collect()
}
