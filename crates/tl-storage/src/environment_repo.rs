use chrono::Utc;
use diesel::dsl::{count_star, now};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{
    CreateWorkspaceEnvironmentRequest, UpdateWorkspaceEnvironmentRequest, WorkspaceEnvironment,
    DEFAULT_ENVIRONMENT_ID,
};

use crate::models::{NewWorkspaceEnvironment, WorkspaceEnvironmentRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{runs, traces, workspace_api_keys, workspace_environments};
use crate::StorageError;

#[derive(Clone)]
pub struct EnvironmentRepo {
    pool: DbPool,
}

impl EnvironmentRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceEnvironment>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = workspace_environments::table
            .filter(workspace_environments::workspace_id.eq(workspace_id))
            .filter(workspace_environments::deleted_at.is_null())
            .select(WorkspaceEnvironmentRecord::as_select())
            .order((
                workspace_environments::is_default.desc(),
                workspace_environments::created_at.asc(),
            ))
            .load::<WorkspaceEnvironmentRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("environment list: {e}")))?;
        Ok(rows.into_iter().map(environment_to_wire).collect())
    }

    pub async fn default_environment_id(&self, workspace_id: &str) -> Result<String, StorageError> {
        let mut conn = self.connection().await?;
        workspace_environments::table
            .filter(workspace_environments::workspace_id.eq(workspace_id))
            .filter(workspace_environments::deleted_at.is_null())
            .filter(workspace_environments::is_default.eq(true))
            .select(workspace_environments::id)
            .first::<String>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("environment default: {e}")))?
            .ok_or(StorageError::NotFound)
    }

    pub async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<WorkspaceEnvironment, StorageError> {
        let mut conn = self.connection().await?;
        let row = workspace_environments::table
            .filter(workspace_environments::workspace_id.eq(workspace_id))
            .filter(workspace_environments::id.eq(environment_id))
            .filter(workspace_environments::deleted_at.is_null())
            .select(WorkspaceEnvironmentRecord::as_select())
            .first::<WorkspaceEnvironmentRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("environment get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(environment_to_wire(row))
    }

    pub async fn create(
        &self,
        workspace_id: &str,
        request: CreateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, StorageError> {
        let id = if request.slug == DEFAULT_ENVIRONMENT_ID {
            DEFAULT_ENVIRONMENT_ID.to_string()
        } else {
            format!("env_{}", uuid::Uuid::now_v7())
        };
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            if request.is_default {
                clear_default(conn, workspace_id).await?;
            }
            diesel::insert_into(workspace_environments::table)
                .values(NewWorkspaceEnvironment {
                    workspace_id: workspace_id.to_string(),
                    id: id.clone(),
                    slug: request.slug,
                    name: request.name,
                    description: request.description,
                    is_default: request.is_default,
                })
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("environment create: {e}")))?;
            Ok(())
        })
        .await?;
        drop(conn);
        self.get(workspace_id, &id).await
    }

    pub async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: UpdateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            let exists = workspace_environments::table
                .filter(workspace_environments::workspace_id.eq(workspace_id))
                .filter(workspace_environments::id.eq(environment_id))
                .filter(workspace_environments::deleted_at.is_null())
                .select(workspace_environments::id)
                .first::<String>(conn)
                .await
                .optional()?
                .is_some();
            if !exists {
                return Err(StorageError::NotFound);
            }
            if request.is_default == Some(false) {
                let is_current_default = workspace_environments::table
                    .filter(workspace_environments::workspace_id.eq(workspace_id))
                    .filter(workspace_environments::id.eq(environment_id))
                    .filter(workspace_environments::deleted_at.is_null())
                    .filter(workspace_environments::is_default.eq(true))
                    .select(workspace_environments::id)
                    .first::<String>(conn)
                    .await
                    .optional()?
                    .is_some();
                if is_current_default {
                    return Err(StorageError::Internal(
                        "workspace must have one default environment".into(),
                    ));
                }
            }
            if request.is_default == Some(true) {
                clear_default(conn, workspace_id).await?;
            }
            if let Some(slug) = request.slug {
                diesel::update(
                    workspace_environments::table
                        .filter(workspace_environments::workspace_id.eq(workspace_id))
                        .filter(workspace_environments::id.eq(environment_id)),
                )
                .set((
                    workspace_environments::slug.eq(slug),
                    workspace_environments::updated_at.eq(now),
                ))
                .execute(conn)
                .await?;
            }
            if let Some(name) = request.name {
                diesel::update(
                    workspace_environments::table
                        .filter(workspace_environments::workspace_id.eq(workspace_id))
                        .filter(workspace_environments::id.eq(environment_id)),
                )
                .set((
                    workspace_environments::name.eq(name),
                    workspace_environments::updated_at.eq(now),
                ))
                .execute(conn)
                .await?;
            }
            if let Some(description) = request.description {
                diesel::update(
                    workspace_environments::table
                        .filter(workspace_environments::workspace_id.eq(workspace_id))
                        .filter(workspace_environments::id.eq(environment_id)),
                )
                .set((
                    workspace_environments::description.eq(Some(description)),
                    workspace_environments::updated_at.eq(now),
                ))
                .execute(conn)
                .await?;
            }
            if let Some(is_default) = request.is_default {
                diesel::update(
                    workspace_environments::table
                        .filter(workspace_environments::workspace_id.eq(workspace_id))
                        .filter(workspace_environments::id.eq(environment_id)),
                )
                .set((
                    workspace_environments::is_default.eq(is_default),
                    workspace_environments::updated_at.eq(now),
                ))
                .execute(conn)
                .await?;
            }
            Ok(())
        })
        .await?;
        drop(conn);
        self.get(workspace_id, environment_id).await
    }

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

            let api_keys = workspace_api_keys::table
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
            if api_keys + runs + traces > 0 {
                return Err(StorageError::Internal(
                    "environment is still referenced by runtime data".into(),
                ));
            }
            let rows = diesel::update(
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
            if rows == 0 {
                return Err(StorageError::NotFound);
            }
            Ok(())
        })
        .await
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

async fn clear_default(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
) -> Result<(), StorageError> {
    diesel::update(
        workspace_environments::table
            .filter(workspace_environments::workspace_id.eq(workspace_id))
            .filter(workspace_environments::is_default.eq(true)),
    )
    .set(workspace_environments::is_default.eq(false))
    .execute(conn)
    .await?;
    Ok(())
}

fn environment_to_wire(row: WorkspaceEnvironmentRecord) -> WorkspaceEnvironment {
    WorkspaceEnvironment {
        id: row.id,
        slug: row.slug,
        name: row.name,
        description: row.description,
        is_default: row.is_default,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

impl std::fmt::Debug for EnvironmentRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvironmentRepo").finish_non_exhaustive()
    }
}
