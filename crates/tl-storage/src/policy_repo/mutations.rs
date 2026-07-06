use std::sync::Arc;

use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_policy::{FamilyPolicy, Policy};

use super::{cache_key, records::policy_row_from_record, PolicyRepo, PolicyRow};
use crate::models::{NewEntityVersion, NewPolicy};
use crate::schema::{entity_versions, policies};
use crate::StorageError;

impl PolicyRepo {
    /// Insert or update a policy. Upsert resurrects soft-deleted rows
    /// and makes them enabled, which matches an author intentionally
    /// publishing the policy again.
    pub async fn upsert(&self, policy: &Policy, source_yaml: &str) -> Result<(), StorageError> {
        self.upsert_in(tl_core::DEFAULT_WORKSPACE_ID, policy, source_yaml)
            .await
    }

    pub async fn upsert_in(
        &self,
        workspace_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<(), StorageError> {
        let parsed_policy = serde_json::to_value(policy)
            .map_err(|e| StorageError::Internal(format!("policy serialize: {e}")))?;
        let new_policy = NewPolicy {
            workspace_id: workspace_id.to_string(),
            id: policy.id.clone(),
            policy_yaml: source_yaml.to_string(),
            parsed_policy,
            owner_agent_id: policy.owner_agent_id.clone(),
            family: None,
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
                ))
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("policy upsert: {e}")))?;

            let count: i64 = entity_versions::table
                .filter(entity_versions::workspace_id.eq(&new_policy.workspace_id))
                .filter(entity_versions::entity_type.eq("policy"))
                .filter(entity_versions::entity_id.eq(&new_policy.id))
                .count()
                .get_result(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("version count: {e}")))?;

            diesel::insert_into(entity_versions::table)
                .values(NewEntityVersion {
                    workspace_id: new_policy.workspace_id.clone(),
                    entity_type: "policy".to_string(),
                    entity_id: new_policy.id.clone(),
                    version: (count + 1) as i32,
                    content: new_policy.policy_yaml.clone(),
                })
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("version insert: {e}")))?;

            Ok(())
        })
        .await?;

        self.cache
            .insert(
                cache_key(workspace_id, &policy.id),
                Arc::new(policy.clone()),
            )
            .await;
        Ok(())
    }

    /// Insert or update a family policy (e.g. the financial family). Stored in
    /// the same `policies` table with the `family` tag set; `parsed_policy`
    /// holds the serialized `FamilyPolicy`. No content cache / version row —
    /// families are loaded fresh via `list_enabled_families_in`.
    pub async fn upsert_family_in(
        &self,
        workspace_id: &str,
        policy: &FamilyPolicy,
        source_yaml: &str,
    ) -> Result<(), StorageError> {
        let parsed_policy = serde_json::to_value(policy)
            .map_err(|e| StorageError::Internal(format!("family serialize: {e}")))?;
        // FamilyPolicy serializes with a `family` tag; use it as the column.
        let family_tag = parsed_policy
            .get("family")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let new_policy = NewPolicy {
            workspace_id: workspace_id.to_string(),
            id: policy.id().to_string(),
            policy_yaml: source_yaml.to_string(),
            parsed_policy,
            owner_agent_id: None,
            family: family_tag,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(policies::table)
            .values(&new_policy)
            .on_conflict((policies::workspace_id, policies::id))
            .do_update()
            .set((
                policies::policy_yaml.eq(excluded(policies::policy_yaml)),
                policies::parsed_policy.eq(excluded(policies::parsed_policy)),
                policies::family.eq(excluded(policies::family)),
                policies::enabled.eq(true),
                policies::updated_at.eq(now),
                policies::deleted_at.eq(None::<chrono::DateTime<chrono::Utc>>),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("family upsert: {e}")))?;
        Ok(())
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

    pub async fn set_enabled(&self, policy_id: &str, enabled: bool) -> Result<(), StorageError> {
        self.set_enabled_in(tl_core::DEFAULT_WORKSPACE_ID, policy_id, enabled)
            .await
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

    /// Soft delete: sets `deleted_at` and clears the cache.
    pub async fn delete(&self, policy_id: &str) -> Result<(), StorageError> {
        self.delete_in(tl_core::DEFAULT_WORKSPACE_ID, policy_id)
            .await
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

fn unique_policy_ids(policy_ids: &[String]) -> Vec<String> {
    let mut ids = Vec::new();
    for id in policy_ids {
        if !ids.iter().any(|existing: &String| existing == id) {
            ids.push(id.clone());
        }
    }
    ids
}
