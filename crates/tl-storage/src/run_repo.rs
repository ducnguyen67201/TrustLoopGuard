use chrono::{DateTime, Utc};
use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{CreateRunRequest, RunKind, RunStatus, RunSummary, UpdateRunRequest};
use uuid::Uuid;

use crate::models::{NewRun, RunRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{run_participants, runs};
use crate::StorageError;

mod events;
mod finalization;
mod reviews;
mod spans;
mod summary;
mod text;
mod traces;
mod validation;

use summary::run_summary;
use text::{kind_text, status_text};
use validation::{non_empty_string, normalize_metadata, parse_run_id};

#[derive(Clone)]
pub struct RunRepo {
    pool: DbPool,
}

impl RunRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateRunRequest,
    ) -> Result<RunSummary, StorageError> {
        let id = Uuid::now_v7();
        let new_run = NewRun {
            workspace_id: workspace_id.to_string(),
            id,
            environment_id: environment_id.to_string(),
            agent_id: input.agent_id.trim().to_string(),
            kind: kind_text(input.kind).to_string(),
            status: status_text(input.status.unwrap_or(RunStatus::Running)).to_string(),
            external_id: input
                .external_id
                .and_then(|value| non_empty_string(value.trim())),
            metadata: normalize_metadata(input.metadata),
        };
        let mut conn = self.connection().await?;
        conn.transaction::<(), StorageError, _>(async |conn| {
            diesel::insert_into(runs::table)
                .values(&new_run)
                .execute(conn)
                .await?;
            diesel::insert_into(run_participants::table)
                .values((
                    run_participants::workspace_id.eq(workspace_id),
                    run_participants::environment_id.eq(environment_id),
                    run_participants::run_id.eq(id),
                    run_participants::agent_id.eq(&new_run.agent_id),
                    run_participants::role.eq("primary"),
                ))
                .execute(conn)
                .await?;
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Internal(format!("run create: {e}")))?;

        self.get(workspace_id, &id.to_string()).await
    }

    pub async fn list(
        &self,
        workspace_id: &str,
        filter: RunFilter,
    ) -> Result<Vec<RunSummary>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = runs::table
            .filter(runs::workspace_id.eq(workspace_id))
            .into_boxed();

        if let Some(agent_id) = filter.agent_id.as_deref() {
            query = query.filter(runs::agent_id.eq(agent_id));
        }
        if let Some(environment_id) = filter.environment_id.as_deref() {
            query = query.filter(runs::environment_id.eq(environment_id));
        }
        if let Some(status) = filter.status {
            query = query.filter(runs::status.eq(status_text(status)));
        }
        if let Some(kind) = filter.kind {
            query = query.filter(runs::kind.eq(kind_text(kind)));
        }
        if let Some(external_id) = filter.external_id.as_deref() {
            query = query.filter(runs::external_id.eq(external_id));
        }

        let records = query
            .select(RunRecord::as_select())
            .order(runs::created_at.desc())
            .limit(filter.limit.clamp(1, 100))
            .load::<RunRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("run list: {e}")))?;

        let mut summaries = Vec::with_capacity(records.len());
        for record in records {
            summaries.push(self.summarize_record(record).await?);
        }
        Ok(summaries)
    }

    pub async fn get(&self, workspace_id: &str, run_id: &str) -> Result<RunSummary, StorageError> {
        let id = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let record = runs::table
            .filter(runs::workspace_id.eq(workspace_id))
            .filter(runs::id.eq(id))
            .select(RunRecord::as_select())
            .first::<RunRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("run get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        self.summarize_record(record).await
    }

    pub async fn update(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: UpdateRunRequest,
    ) -> Result<RunSummary, StorageError> {
        let id = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let current = runs::table
            .filter(runs::workspace_id.eq(workspace_id))
            .filter(runs::id.eq(id))
            .select(RunRecord::as_select())
            .first::<RunRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("run update get: {e}")))?
            .ok_or(StorageError::NotFound)?;

        let next_status = input
            .status
            .map(status_text)
            .map(str::to_string)
            .unwrap_or(current.status);
        let ended_at = match input.ended_at {
            Some(value) => Some(
                DateTime::parse_from_rfc3339(&value)
                    .map_err(|e| StorageError::Internal(format!("ended_at parse: {e}")))?
                    .with_timezone(&Utc),
            ),
            None if input.status.is_some_and(|status| {
                matches!(
                    status,
                    RunStatus::Completed
                        | RunStatus::Failed
                        | RunStatus::Canceled
                        | RunStatus::TimedOut
                )
            }) =>
            {
                Some(Utc::now())
            }
            None => None,
        };
        let next_metadata = input
            .metadata
            .map(normalize_metadata)
            .unwrap_or(current.metadata);
        let next_ended_at = ended_at.or(current.ended_at);
        let rows = diesel::update(
            runs::table
                .filter(runs::workspace_id.eq(workspace_id))
                .filter(runs::id.eq(id)),
        )
        .set((
            runs::status.eq(next_status),
            runs::metadata.eq(next_metadata),
            runs::ended_at.eq(next_ended_at),
            runs::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("run update: {e}")))?;

        if rows == 0 {
            return Err(StorageError::NotFound);
        }
        self.get(workspace_id, run_id).await
    }

    pub(super) async fn summarize_record(
        &self,
        record: RunRecord,
    ) -> Result<RunSummary, StorageError> {
        let stats = self.stats(&record.workspace_id, record.id).await?;
        run_summary(record, stats)
    }

    pub(super) async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

#[derive(Debug, Default)]
pub struct RunFilter {
    pub environment_id: Option<String>,
    pub agent_id: Option<String>,
    pub status: Option<RunStatus>,
    pub kind: Option<RunKind>,
    pub external_id: Option<String>,
    pub limit: i64,
}

impl std::fmt::Debug for RunRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunRepo").finish_non_exhaustive()
    }
}
