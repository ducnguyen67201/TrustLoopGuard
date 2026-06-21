//! Durable storage for red-team dispatch jobs + per-attack results.
//!
//! Unlike `run_repo`, the job summary is stored directly (no stats aggregation):
//! the orchestrator writes rolled-up counts when a job finishes.

use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{
    JobStatus, RedteamAttackRecord, RedteamAttackSession, RedteamDispatchRequest, RedteamJobResult,
    RedteamJobSummary, RedteamSessionEvent,
};
use uuid::Uuid;

use crate::models::{
    NewRedteamAttackSession, NewRedteamJob, NewRedteamSessionEvent, RedteamAttackRecordRow,
    RedteamAttackSessionRecord, RedteamJobRecord, RedteamSessionEventRecord,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{redteam_attack_sessions, redteam_jobs, redteam_session_events};
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

const INTERNAL_RUNNER_LABEL: &str = "runner_default";

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
        let new_job = NewRedteamJob {
            workspace_id: workspace_id.to_string(),
            id,
            environment_id: environment_id.to_string(),
            status: status_text(JobStatus::Queued).to_string(),
            target: request.target_url.clone(),
            profile: request.profile.clone(),
            generator: INTERNAL_RUNNER_LABEL.to_string(),
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
        let session = RedteamAttackSession {
            session_id: format!("session-{}", result.seq),
            runner_session_id: None,
            seq: result.seq,
            case_id: result.case_id.clone(),
            track: result.track.clone(),
            kind: result.kind.clone(),
            trial_index: result.trial_index,
            attack: result.attack.clone(),
            goal: result.goal.clone(),
            status: "complete".into(),
            outcome: result.outcome.clone(),
            landed: result.landed,
            trace_id: result.trace_id.clone(),
            events: result_events(result),
            error: None,
        };
        self.record_session(workspace_id, job_id, &session).await
    }

    pub async fn record_session(
        &self,
        workspace_id: &str,
        job_id: &str,
        session: &RedteamAttackSession,
    ) -> Result<(), StorageError> {
        let id = parse_uuid(job_id)?;
        let new_session = NewRedteamAttackSession {
            workspace_id: workspace_id.to_string(),
            job_id: id,
            session_id: session.session_id.clone(),
            runner_session_id: session.runner_session_id.clone(),
            seq: session.seq,
            case_id: session.case_id.clone(),
            track: session.track.clone(),
            kind: session.kind.clone(),
            trial_index: session.trial_index,
            attack: session.attack.clone(),
            goal: session.goal.clone(),
            status: session.status.clone(),
            outcome: session.outcome.clone(),
            landed: session.landed,
            trace_id: session.trace_id.clone(),
            error: session.error.clone(),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(redteam_attack_sessions::table)
            .values(&new_session)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam session record: {e}")))?;
        let events: Vec<_> = session
            .events
            .iter()
            .map(|event| NewRedteamSessionEvent {
                workspace_id: workspace_id.to_string(),
                job_id: id,
                session_id: session.session_id.clone(),
                event_id: event.event_id.clone(),
                seq: event.seq,
                kind: event.kind.clone(),
                actor: event.actor.clone(),
                label: event.label.clone(),
                content_text: event.content_text.clone(),
                payload: event.payload.clone(),
                trace_id: event.trace_id.clone(),
            })
            .collect();
        if !events.is_empty() {
            diesel::insert_into(redteam_session_events::table)
                .values(&events)
                .execute(&mut conn)
                .await
                .map_err(|e| StorageError::Internal(format!("redteam event record: {e}")))?;
        }
        Ok(())
    }

    pub async fn list_results(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<Vec<RedteamJobResult>, StorageError> {
        Ok(self
            .list_sessions(workspace_id, job_id)
            .await?
            .into_iter()
            .map(result_from_session)
            .collect())
    }

    pub async fn list_sessions(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<Vec<RedteamAttackSession>, StorageError> {
        let id = parse_uuid(job_id)?;
        let mut conn = self.connection().await?;
        let session_records = redteam_attack_sessions::table
            .filter(redteam_attack_sessions::workspace_id.eq(workspace_id))
            .filter(redteam_attack_sessions::job_id.eq(id))
            .select(RedteamAttackSessionRecord::as_select())
            .order(redteam_attack_sessions::seq.asc())
            .load::<RedteamAttackSessionRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam session list: {e}")))?;
        let event_records = redteam_session_events::table
            .filter(redteam_session_events::workspace_id.eq(workspace_id))
            .filter(redteam_session_events::job_id.eq(id))
            .select(RedteamSessionEventRecord::as_select())
            .order((
                redteam_session_events::session_id.asc(),
                redteam_session_events::seq.asc(),
            ))
            .load::<RedteamSessionEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam event list: {e}")))?;
        Ok(sessions_from_records(session_records, event_records))
    }

    /// Every attack session in the workspace, flattened with its parent job's
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
        let mut query = redteam_attack_sessions::table
            .inner_join(
                redteam_jobs::table.on(redteam_attack_sessions::job_id
                    .eq(redteam_jobs::id)
                    .and(redteam_attack_sessions::workspace_id.eq(redteam_jobs::workspace_id))),
            )
            .filter(redteam_attack_sessions::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(attack) = filter.attack.as_deref() {
            query = query.filter(redteam_attack_sessions::attack.eq(attack.to_string()));
        }
        if let Some(outcome) = filter.outcome.as_deref() {
            query = query.filter(redteam_attack_sessions::outcome.eq(outcome.to_string()));
        }
        let rows = query
            .select((
                redteam_jobs::id,
                redteam_jobs::target,
                redteam_jobs::profile,
                redteam_jobs::created_at,
                redteam_attack_sessions::session_id,
                redteam_attack_sessions::seq,
                redteam_attack_sessions::attack,
                redteam_attack_sessions::goal,
                redteam_attack_sessions::outcome,
                redteam_attack_sessions::landed,
                redteam_attack_sessions::trace_id,
            ))
            .order(redteam_jobs::created_at.desc())
            .then_order_by(redteam_attack_sessions::seq.asc())
            .limit(filter.limit.clamp(1, 100))
            .load::<RedteamAttackRecordRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam attack records: {e}")))?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let events = redteam_session_events::table
                .filter(redteam_session_events::workspace_id.eq(workspace_id))
                .filter(redteam_session_events::job_id.eq(row.job_id))
                .filter(redteam_session_events::session_id.eq(row.session_id.clone()))
                .select(RedteamSessionEventRecord::as_select())
                .order(redteam_session_events::seq.asc())
                .load::<RedteamSessionEventRecord>(&mut conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("redteam attack record events: {e}"))
                })?;
            records.push(attack_record(row, &events));
        }
        Ok(records)
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

fn job_summary(record: RedteamJobRecord) -> RedteamJobSummary {
    RedteamJobSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        environment_id: record.environment_id,
        status: parse_status(&record.status),
        target: record.target,
        profile: record.profile,
        agent_id: record.agent_id,
        attacks: record.attacks,
        landed: record.landed,
        blocked: record.blocked,
        error: record.error,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

fn sessions_from_records(
    sessions: Vec<RedteamAttackSessionRecord>,
    events: Vec<RedteamSessionEventRecord>,
) -> Vec<RedteamAttackSession> {
    sessions
        .into_iter()
        .map(|session| {
            let session_events = events
                .iter()
                .filter(|event| {
                    event.workspace_id == session.workspace_id
                        && event.job_id == session.job_id
                        && event.session_id == session.session_id
                })
                .map(event_summary)
                .collect();
            RedteamAttackSession {
                session_id: session.session_id,
                runner_session_id: session.runner_session_id,
                seq: session.seq,
                case_id: session.case_id,
                track: session.track,
                kind: session.kind,
                trial_index: session.trial_index,
                attack: session.attack,
                goal: session.goal,
                status: session.status,
                outcome: session.outcome,
                landed: session.landed,
                trace_id: session.trace_id,
                events: session_events,
                error: session.error,
            }
        })
        .collect()
}

fn event_summary(record: &RedteamSessionEventRecord) -> RedteamSessionEvent {
    RedteamSessionEvent {
        event_id: record.event_id.clone(),
        seq: record.seq,
        kind: record.kind.clone(),
        actor: record.actor.clone(),
        label: record.label.clone(),
        content_text: record.content_text.clone(),
        payload: record.payload.clone(),
        trace_id: record.trace_id.clone(),
        created_at: record.created_at.to_rfc3339(),
    }
}

fn result_from_session(session: RedteamAttackSession) -> RedteamJobResult {
    RedteamJobResult {
        seq: session.seq,
        case_id: session.case_id,
        track: session.track,
        kind: session.kind,
        trial_index: session.trial_index,
        attack: session.attack,
        goal: session.goal,
        outcome: session.outcome,
        landed: session.landed,
        prompt: event_text(&session.events, "attack_prompt"),
        reply: event_text(&session.events, "target_reply").unwrap_or_default(),
        trace_id: session.trace_id,
    }
}

fn result_events(result: &RedteamJobResult) -> Vec<RedteamSessionEvent> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut events = Vec::new();
    if let Some(prompt) = result.prompt.clone() {
        events.push(RedteamSessionEvent {
            event_id: format!("{}-prompt", result.seq),
            seq: 0,
            kind: "attack_prompt".into(),
            actor: "attacker".into(),
            label: None,
            content_text: Some(prompt),
            payload: serde_json::json!({}),
            trace_id: None,
            created_at: timestamp.clone(),
        });
    }
    events.push(RedteamSessionEvent {
        event_id: format!("{}-reply", result.seq),
        seq: 1,
        kind: "target_reply".into(),
        actor: "target".into(),
        label: None,
        content_text: Some(result.reply.clone()),
        payload: serde_json::json!({}),
        trace_id: result.trace_id.clone(),
        created_at: timestamp,
    });
    events
}

fn event_text(events: &[RedteamSessionEvent], kind: &str) -> Option<String> {
    events
        .iter()
        .find(|event| event.kind == kind)
        .and_then(|event| event.content_text.clone())
}

fn event_record_text(events: &[RedteamSessionEventRecord], kind: &str) -> Option<String> {
    events
        .iter()
        .find(|event| event.kind == kind)
        .and_then(|event| event.content_text.clone())
}

fn attack_record(
    row: RedteamAttackRecordRow,
    events: &[RedteamSessionEventRecord],
) -> RedteamAttackRecord {
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
        prompt: event_record_text(events, "attack_prompt"),
        reply: event_record_text(events, "target_reply").unwrap_or_default(),
        trace_id: row.trace_id,
    }
}
