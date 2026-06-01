use chrono::Utc;
use diesel::dsl::{count_star, now};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};

use super::EnvironmentRepo;
use crate::schema::{runs, traces, workspace_api_keys, workspace_environments};
use crate::StorageError;

impl EnvironmentRepo {
    pub async fn delete(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            let target = workspace_environments::table
                .filter(workspace_environments::workspace_id.eq(workspace_id))
                .filter(workspace_environments::id.eq(environment_id))
                .filter(workspace_environments::deleted_at.is_null())
                .select((
                    workspace_environments::id,
                    workspace_environments::is_default,
                ))
                .first::<(String, bool)>(conn)
                .await
                .optional()?
                .ok_or(StorageError::NotFound)?;

            if target.1 {
                return Err(StorageError::Internal(
                    "default environment cannot be deleted".into(),
                ));
            }

            let active_api_keys = workspace_api_keys::table
                .filter(workspace_api_keys::workspace_id.eq(workspace_id))
                .filter(workspace_api_keys::environment_id.eq(environment_id))
                .filter(workspace_api_keys::revoked_at.is_null())
                .select(count_star())
                .first::<i64>(conn)
                .await?;
            let runs = runs::table
                .filter(runs::workspace_id.eq(workspace_id))
                .filter(runs::environment_id.eq(environment_id))
                .select(count_star())
                .first::<i64>(conn)
                .await?;
            let traces = traces::table
                .filter(traces::workspace_id.eq(workspace_id))
                .filter(traces::environment_id.eq(environment_id))
                .select(count_star())
                .first::<i64>(conn)
                .await?;

            if active_api_keys + runs + traces > 0 {
                return Err(StorageError::Internal(
                    "environment is still referenced by runtime data".into(),
                ));
            }

            let deleted_rows = diesel::update(
                workspace_environments::table
                    .filter(workspace_environments::workspace_id.eq(workspace_id))
                    .filter(workspace_environments::id.eq(environment_id))
                    .filter(workspace_environments::deleted_at.is_null()),
            )
            .set((
                workspace_environments::deleted_at.eq(Some(Utc::now())),
                workspace_environments::is_default.eq(false),
                workspace_environments::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;

            if deleted_rows == 0 {
                return Err(StorageError::NotFound);
            }

            Ok(())
        })
        .await
    }
}
