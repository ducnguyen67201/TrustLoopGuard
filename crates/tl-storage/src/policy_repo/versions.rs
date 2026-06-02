use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::PolicyRepo;
use crate::{models::EntityVersionRecord, schema::entity_versions, StorageError};

impl PolicyRepo {
    /// All saved versions for a policy, newest first.
    pub async fn list_versions_in(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<Vec<EntityVersionRecord>, StorageError> {
        let mut conn = self.connection().await?;
        entity_versions::table
            .filter(entity_versions::workspace_id.eq(workspace_id))
            .filter(entity_versions::entity_type.eq("policy"))
            .filter(entity_versions::entity_id.eq(policy_id))
            .order(entity_versions::version.desc())
            .load::<EntityVersionRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list versions: {e}")))
    }

    /// Single version by number.
    pub async fn get_version_in(
        &self,
        workspace_id: &str,
        policy_id: &str,
        version: i32,
    ) -> Result<EntityVersionRecord, StorageError> {
        let mut conn = self.connection().await?;
        entity_versions::table
            .filter(entity_versions::workspace_id.eq(workspace_id))
            .filter(entity_versions::entity_type.eq("policy"))
            .filter(entity_versions::entity_id.eq(policy_id))
            .filter(entity_versions::version.eq(version))
            .first::<EntityVersionRecord>(&mut conn)
            .await
            .map_err(|e| match e {
                diesel::result::Error::NotFound => StorageError::NotFound,
                other => StorageError::Internal(format!("get version: {other}")),
            })
    }
}
