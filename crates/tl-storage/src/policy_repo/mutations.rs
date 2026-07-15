use std::sync::Arc;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::PolicyFamily;
use tl_policy::{AnyPolicy, FamilyPolicy, Policy};

use super::{cache_key, records::policy_row_from_record, PolicyRepo, PolicyRow};
use crate::models::{NewEntityVersion, NewPolicy};
use crate::schema::{entity_versions, policies};
use crate::StorageError;

impl PolicyRepo {
    pub async fn upsert_in(
        &self,
        workspace_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<(), StorageError> {
        let any = AnyPolicy::Content(policy.clone());
        self.upsert_any_in(workspace_id, &any, source_yaml).await?;

        self.cache
            .insert(
                cache_key(workspace_id, &policy.id),
                Arc::new(policy.clone()),
            )
            .await;
        Ok(())
    }

    pub async fn upsert_any_in(
        &self,
        workspace_id: &str,
        policy: &AnyPolicy,
        source_yaml: &str,
    ) -> Result<(), StorageError> {
        let parsed_policy = match policy {
            AnyPolicy::Content(policy) => serde_json::to_value(policy),
            AnyPolicy::Family(policy) => serde_json::to_value(policy),
        }
        .map_err(|e| StorageError::Internal(format!("policy serialize: {e}")))?;
        let new_policy = NewPolicy {
            workspace_id: workspace_id.to_string(),
            id: policy.id().to_string(),
            policy_yaml: source_yaml.to_string(),
            parsed_policy,
            owner_agent_id: policy.owner_agent_id().map(str::to_string),
            family: stored_family(policy.family()),
        };
        let mut conn = self.connection().await?;

        conn.transaction::<_, StorageError, _>(async |conn| {
            diesel::insert_into(policies::table)
                .values(&new_policy)
                .on_conflict((policies::workspace_id, policies::id))
                .do_update()
                .set((
                    policies::policy_yaml.eq(excluded(policies::policy_yaml)),
                    policies::parsed_policy.eq(excluded(policies::parsed_policy)),
                    policies::enabled.eq(true),
                    policies::updated_at.eq(now),
                    policies::deleted_at.eq(None::<chrono::DateTime<chrono::Utc>>),
                    policies::owner_agent_id.eq(excluded(policies::owner_agent_id)),
                    policies::family.eq(excluded(policies::family)),
                ))
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("policy upsert: {e}")))?;

            insert_policy_version(conn, &new_policy).await?;
            Ok(())
        })
        .await?;

        if let AnyPolicy::Content(policy) = policy {
            self.cache
                .insert(
                    cache_key(workspace_id, &policy.id),
                    Arc::new(policy.clone()),
                )
                .await;
        }
        Ok(())
    }

    /// Insert or update a family policy (e.g. the financial family). Stored in
    /// the same `policies` table with the `family` tag set and the same version
    /// history lifecycle as content policies.
    pub async fn upsert_family_in(
        &self,
        workspace_id: &str,
        policy: &FamilyPolicy,
        source_yaml: &str,
    ) -> Result<(), StorageError> {
        self.upsert_any_in(
            workspace_id,
            &AnyPolicy::Family(policy.clone()),
            source_yaml,
        )
        .await
    }

    /// Soft-delete every active policy owned by `agent_id`. Returns the
    /// list of policy ids that were marked deleted so callers can also
    /// invalidate their caches. Used by the cascade-delete handler in
    /// the server when an agent is soft-deleted.
    pub async fn soft_delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        let mut conn = self.connection().await?;
        let now_ts = chrono::Utc::now();
        let deleted_ids: Vec<String> = diesel::update(
            policies::table
                .filter(policies::workspace_id.eq(workspace_id))
                .filter(policies::owner_agent_id.eq(agent_id))
                .filter(policies::deleted_at.is_null()),
        )
        .set(policies::deleted_at.eq(Some(now_ts)))
        .returning(policies::id)
        .get_results(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("policy cascade delete: {e}")))?;

        for id in &deleted_ids {
            self.cache.invalidate(&cache_key(workspace_id, id)).await;
        }
        Ok(deleted_ids)
    }

    pub async fn set_enabled_in(
        &self,
        workspace_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows_affected = diesel::update(
            policies::table
                .filter(policies::workspace_id.eq(workspace_id))
                .filter(policies::id.eq(policy_id))
                .filter(policies::deleted_at.is_null()),
        )
        .set((policies::enabled.eq(enabled), policies::updated_at.eq(now)))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("policy set enabled: {e}")))?;

        if rows_affected == 0 {
            return Err(StorageError::NotFound);
        }
        self.cache
            .invalidate(&cache_key(workspace_id, policy_id))
            .await;
        Ok(())
    }

    pub async fn batch_set_enabled_in(
        &self,
        workspace_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<PolicyRow>, StorageError> {
        let ids = unique_policy_ids(policy_ids);
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut conn = self.connection().await?;
        let rows = conn
            .transaction::<_, StorageError, _>(async |conn| {
                let existing = policies::table
                    .filter(policies::workspace_id.eq(workspace_id))
                    .filter(policies::id.eq_any(&ids))
                    .filter(policies::deleted_at.is_null())
                    .select(policies::id)
                    .load::<String>(conn)
                    .await?;
                if existing.len() != ids.len() {
                    return Err(StorageError::NotFound);
                }

                diesel::update(
                    policies::table
                        .filter(policies::workspace_id.eq(workspace_id))
                        .filter(policies::id.eq_any(&ids))
                        .filter(policies::deleted_at.is_null()),
                )
                .set((policies::enabled.eq(enabled), policies::updated_at.eq(now)))
                .execute(conn)
                .await?;

                let records = policies::table
                    .filter(policies::workspace_id.eq(workspace_id))
                    .filter(policies::id.eq_any(&ids))
                    .filter(policies::deleted_at.is_null())
                    .select((
                        policies::parsed_policy,
                        policies::policy_yaml,
                        policies::enabled,
                        policies::owner_agent_id,
                        policies::family,
                    ))
                    .order(policies::id.asc())
                    .load::<crate::models::PolicyRecord>(conn)
                    .await?;

                records.into_iter().map(policy_row_from_record).collect()
            })
            .await?;

        for id in ids {
            self.cache.invalidate(&cache_key(workspace_id, &id)).await;
        }
        Ok(rows)
    }

    pub async fn delete_in(&self, workspace_id: &str, policy_id: &str) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows_affected = diesel::update(
            policies::table
                .filter(policies::workspace_id.eq(workspace_id))
                .filter(policies::id.eq(policy_id))
                .filter(policies::deleted_at.is_null()),
        )
        .set(policies::deleted_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("policy delete: {e}")))?;

        self.cache
            .invalidate(&cache_key(workspace_id, policy_id))
            .await;

        if rows_affected == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

fn stored_family(family: PolicyFamily) -> Option<String> {
    Some(family.as_str().to_string())
}

async fn insert_policy_version(
    conn: &mut crate::postgres::DbConnection<'_>,
    policy: &NewPolicy,
) -> Result<(), StorageError> {
    let count: i64 = entity_versions::table
        .filter(entity_versions::workspace_id.eq(&policy.workspace_id))
        .filter(entity_versions::entity_type.eq("policy"))
        .filter(entity_versions::entity_id.eq(&policy.id))
        .count()
        .get_result(conn)
        .await
        .map_err(|e| StorageError::Internal(format!("version count: {e}")))?;

    diesel::insert_into(entity_versions::table)
        .values(NewEntityVersion {
            workspace_id: policy.workspace_id.clone(),
            entity_type: "policy".to_string(),
            entity_id: policy.id.clone(),
            version: (count + 1) as i32,
            content: policy.policy_yaml.clone(),
        })
        .execute(conn)
        .await
        .map_err(|e| StorageError::Internal(format!("version insert: {e}")))?;
    Ok(())
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
