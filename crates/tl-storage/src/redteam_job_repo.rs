//! Durable storage for red-team dispatch jobs + per-attack results.
//!
//! Unlike `run_repo`, the job summary is stored directly (no stats aggregation):
//! the orchestrator writes rolled-up counts when a job finishes.

use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{
    JobStatus, RedteamAttackRecord, RedteamDispatchRequest, RedteamGenerator, RedteamJobResult,
    RedteamJobSummary,
};
use uuid::Uuid;

use crate::models::{
    NewRedteamJob, NewRedteamJobResult, RedteamAttackRecordRow, RedteamJobRecord,
    RedteamJobResultRecord,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{redteam_job_results, redteam_jobs};
use crate::StorageError;

#[derive(Clone)]
pub struct RedteamJobRepo {
    pool: DbPool,
}

#[derive(Debug, Default)]
pub struct RedteamJobFilter {
    pub agent_id: Option<String>,
    pub limit: i64,
}

/// Filter for the workspace-wide attack-record query. Workspace scoping is implicit.
#[derive(Debug, Default)]
pub struct RedteamAttackRecordFilter {
    pub attack: Option<String>,
    pub outcome: Option<String>,
    pub limit: i64,
}

/// Rolled-up attack counts written when a job finishes.
#[derive(Debug, Default, Clone, Copy)]
pub struct JobCounts {
    pub attacks: i64,
    pub landed: i64,
    pub blocked: i64,
}

impl RedteamJobRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: &RedteamDispatchRequest,
    ) -> Result<RedteamJobSummary, StorageError> {
        let id = Uuid::now_v7();
        let generator = request.generator.unwrap_or(RedteamGenerator::Deterministic);
        let new_job = NewRedteamJob {
            workspace_id: workspace_id.to_string(),
            id,
            environment_id: environment_id.to_string(),
            status: status_text(JobStatus::Queued).to_string(),
            target: request.target_url.clone(),
            profile: request.profile.clone(),
            generator: generator_text(generator).to_string(),
            agent_id: request
                .agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(redteam_jobs::table)
            .values(&new_job)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam job create: {e}")))?;
        self.get(workspace_id, &id.to_string()).await
    }

    pub async fn get(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<RedteamJobSummary, StorageError> {
        let id = parse_uuid(job_id)?;
        let mut conn = self.connection().await?;
        let record = redteam_jobs::table
            .filter(redteam_jobs::workspace_id.eq(workspace_id))
            .filter(redteam_jobs::id.eq(id))
            .select(RedteamJobRecord::as_select())
            .first::<RedteamJobRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("redteam job get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(job_summary(record))
    }

    pub async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamJobFilter,
    ) -> Result<Vec<RedteamJobSummary>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = redteam_jobs::table
            .filter(redteam_jobs::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(agent_id) = filter.agent_id.as_deref() {
            query = query.filter(redteam_jobs::agent_id.eq(agent_id));
        }
        let records = query
            .select(RedteamJobRecord::as_select())
            .order(redteam_jobs::created_at.desc())
            .limit(filter.limit.clamp(1, 100))
            .load::<RedteamJobRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam job list: {e}")))?;
        Ok(records.into_iter().map(job_summary).collect())
    }

    /// Update a job's status (and, on completion, the rolled-up counts).
    ///
    /// Terminal states are final: the update is gated on the current status not
    /// already being terminal, so a completing job can never clobber a concurrent
    /// `cancel` (and vice versa) — the first terminal write wins. `counts` is only
    /// written when supplied (completion); a status-only transition leaves any
    /// partial counts intact. A no-op (`rows == 0`, i.e. absent or already
    /// terminal) is not an error here — callers that need absence detection
    /// re-read via `get`.
    pub async fn set_status(
        &self,
        workspace_id: &str,
        job_id: &str,
        status: JobStatus,
        counts: Option<JobCounts>,
        error: Option<&str>,
    ) -> Result<(), StorageError> {
        let id = parse_uuid(job_id)?;
        let mut conn = self.connection().await?;
        let result = match counts {
            Some(counts) => {
                diesel::update(
                    redteam_jobs::table
                        .filter(redteam_jobs::workspace_id.eq(workspace_id))
                        .filter(redteam_jobs::id.eq(id))
                        .filter(redteam_jobs::status.ne_all(TERMINAL_STATUSES)),
                )
                .set((
                    redteam_jobs::status.eq(status_text(status)),
                    redteam_jobs::attacks.eq(counts.attacks),
                    redteam_jobs::landed.eq(counts.landed),
                    redteam_jobs::blocked.eq(counts.blocked),
                    redteam_jobs::error.eq(error.map(str::to_string)),
                    redteam_jobs::updated_at.eq(now),
                ))
                .execute(&mut conn)
                .await
            }
            None => {
                diesel::update(
                    redteam_jobs::table
                        .filter(redteam_jobs::workspace_id.eq(workspace_id))
                        .filter(redteam_jobs::id.eq(id))
                        .filter(redteam_jobs::status.ne_all(TERMINAL_STATUSES)),
                )
                .set((
                    redteam_jobs::status.eq(status_text(status)),
                    redteam_jobs::error.eq(error.map(str::to_string)),
                    redteam_jobs::updated_at.eq(now),
                ))
                .execute(&mut conn)
                .await
            }
        };
        result.map_err(|e| StorageError::Internal(format!("redteam job status: {e}")))?;
        Ok(())
    }

    pub async fn record_result(
        &self,
        workspace_id: &str,
        job_id: &str,
        result: &RedteamJobResult,
    ) -> Result<(), StorageError> {
        let id = parse_uuid(job_id)?;
        let new_result = NewRedteamJobResult {
            workspace_id: workspace_id.to_string(),
            job_id: id,
            seq: result.seq,
            attack: result.attack.clone(),
            goal: result.goal.clone(),
            outcome: result.outcome.clone(),
            landed: result.landed,
            prompt: result.prompt.clone(),
            reply: result.reply.clone(),
            trace_id: result.trace_id.clone(),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(redteam_job_results::table)
            .values(&new_result)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam result record: {e}")))?;
        Ok(())
    }

    pub async fn list_results(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<Vec<RedteamJobResult>, StorageError> {
        let id = parse_uuid(job_id)?;
        let mut conn = self.connection().await?;
        let records = redteam_job_results::table
            .filter(redteam_job_results::workspace_id.eq(workspace_id))
            .filter(redteam_job_results::job_id.eq(id))
            .select(RedteamJobResultRecord::as_select())
            .order(redteam_job_results::seq.asc())
            .load::<RedteamJobResultRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam result list: {e}")))?;
        Ok(records.into_iter().map(result_summary).collect())
    }

    /// Every attack result in the workspace, flattened with its parent job's
    /// context (target/profile/created_at), newest job first then attack `seq`.
    ///
    /// The join is explicit (`.on(... .and(...))`): the tables share a composite
    /// key (`workspace_id`, `job_id` → `id`) with no single-column FK, so there is
    /// no `joinable!` to lean on. `allow_tables_to_appear_in_same_query!` in
    /// `schema.rs` is what makes the join compile.
    pub async fn list_attack_records(
        &self,
        workspace_id: &str,
        filter: RedteamAttackRecordFilter,
    ) -> Result<Vec<RedteamAttackRecord>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = redteam_job_results::table
            .inner_join(
                redteam_jobs::table.on(redteam_job_results::job_id
                    .eq(redteam_jobs::id)
                    .and(redteam_job_results::workspace_id.eq(redteam_jobs::workspace_id))),
            )
            .filter(redteam_job_results::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(attack) = filter.attack.as_deref() {
            query = query.filter(redteam_job_results::attack.eq(attack.to_string()));
        }
        if let Some(outcome) = filter.outcome.as_deref() {
            query = query.filter(redteam_job_results::outcome.eq(outcome.to_string()));
        }
        let rows = query
            .select((
                redteam_jobs::id,
                redteam_jobs::target,
                redteam_jobs::profile,
                redteam_jobs::created_at,
                redteam_job_results::seq,
                redteam_job_results::attack,
                redteam_job_results::goal,
                redteam_job_results::outcome,
                redteam_job_results::landed,
                redteam_job_results::prompt,
                redteam_job_results::reply,
                redteam_job_results::trace_id,
            ))
            .order(redteam_jobs::created_at.desc())
            .then_order_by(redteam_job_results::seq.asc())
            .limit(filter.limit.clamp(1, 100))
            .load::<RedteamAttackRecordRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam attack records: {e}")))?;
        Ok(rows.into_iter().map(attack_record).collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for RedteamJobRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedteamJobRepo").finish_non_exhaustive()
    }
}

/// Statuses a job cannot transition out of once reached (the first terminal
/// write wins). Kept as text to match the stored column.
const TERMINAL_STATUSES: [&str; 3] = ["complete", "error", "cancelled"];

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::NotFound)
}

fn status_text(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Complete => "complete",
        JobStatus::Error => "error",
        JobStatus::Cancelled => "cancelled",
    }
}

fn parse_status(text: &str) -> JobStatus {
    match text {
        "running" => JobStatus::Running,
        "complete" => JobStatus::Complete,
        "error" => JobStatus::Error,
        "cancelled" => JobStatus::Cancelled,
        _ => JobStatus::Queued,
    }
}

fn generator_text(generator: RedteamGenerator) -> &'static str {
    match generator {
        RedteamGenerator::Deterministic => "deterministic",
        RedteamGenerator::Hackagent => "hackagent",
    }
}

fn parse_generator(text: &str) -> RedteamGenerator {
    match text {
        "hackagent" => RedteamGenerator::Hackagent,
        _ => RedteamGenerator::Deterministic,
    }
}

fn job_summary(record: RedteamJobRecord) -> RedteamJobSummary {
    RedteamJobSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        environment_id: record.environment_id,
        status: parse_status(&record.status),
        target: record.target,
        profile: record.profile,
        generator: parse_generator(&record.generator),
        agent_id: record.agent_id,
        attacks: record.attacks,
        landed: record.landed,
        blocked: record.blocked,
        error: record.error,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

fn result_summary(record: RedteamJobResultRecord) -> RedteamJobResult {
    RedteamJobResult {
        seq: record.seq,
        case_id: None,
        track: None,
        kind: None,
        trial_index: None,
        attack: record.attack,
        goal: record.goal,
        outcome: record.outcome,
        landed: record.landed,
        prompt: record.prompt,
        reply: record.reply,
        trace_id: record.trace_id,
    }
}

fn attack_record(row: RedteamAttackRecordRow) -> RedteamAttackRecord {
    RedteamAttackRecord {
        job_id: row.job_id.to_string(),
        target: row.target,
        profile: row.profile,
        created_at: row.created_at.to_rfc3339(),
        seq: row.seq,
        attack: row.attack,
        goal: row.goal,
        outcome: row.outcome,
        landed: row.landed,
        prompt: row.prompt,
        reply: row.reply,
        trace_id: row.trace_id,
    }
}
