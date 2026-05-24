use chrono::{DateTime, Utc};
use diesel::dsl::{max, now};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{
    CreateRunEventRequest, CreateRunRequest, HumanReviewOutcome, RunEventKind, RunEventSummary,
    RunKind, RunStatus, RunSummary, TraceSummary, UpdateRunRequest,
};
use uuid::Uuid;

use crate::models::{NewRun, NewRunEvent, RunEventRecord, RunRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{human_review_events, run_events, runs, traces};
use crate::StorageError;

type TraceReviewLookupRow = (
    Uuid,
    Option<Uuid>,
    Option<Uuid>,
    String,
    String,
    String,
    i32,
    serde_json::Value,
    DateTime<Utc>,
);

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
        diesel::insert_into(runs::table)
            .values(&new_run)
            .execute(&mut conn)
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
                    RunStatus::Completed | RunStatus::Failed | RunStatus::Canceled
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

    pub async fn create_event(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: CreateRunEventRequest,
    ) -> Result<RunEventSummary, StorageError> {
        validate_create_run_event(&input)?;
        let run_uuid = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let id = conn
            .transaction::<Uuid, StorageError, _>(async |conn| {
                let locked_rows = diesel::update(
                    runs::table
                        .filter(runs::workspace_id.eq(workspace_id))
                        .filter(runs::id.eq(run_uuid)),
                )
                .set(runs::updated_at.eq(runs::updated_at))
                .execute(conn)
                .await?;
                if locked_rows == 0 {
                    return Err(StorageError::NotFound);
                }

                let sequence = match input.sequence {
                    Some(sequence) => sequence,
                    None => {
                        let current = run_events::table
                            .filter(run_events::workspace_id.eq(workspace_id))
                            .filter(run_events::run_id.eq(run_uuid))
                            .select(max(run_events::sequence))
                            .first::<Option<i32>>(conn)
                            .await?;
                        current.unwrap_or(0) + 1
                    }
                };
                let occurred_at = match input.occurred_at {
                    Some(value) => DateTime::parse_from_rfc3339(&value)
                        .map_err(|e| StorageError::Internal(format!("occurred_at parse: {e}")))?
                        .with_timezone(&Utc),
                    None => Utc::now(),
                };
                let id = Uuid::now_v7();
                let event = NewRunEvent {
                    workspace_id: workspace_id.to_string(),
                    id,
                    run_id: run_uuid,
                    sequence,
                    kind: event_kind_text(input.kind).to_string(),
                    label: input.label.and_then(|value| non_empty_string(value.trim())),
                    input_summary: input
                        .input_summary
                        .and_then(|value| non_empty_string(value.trim())),
                    output_summary: input
                        .output_summary
                        .and_then(|value| non_empty_string(value.trim())),
                    metadata: normalize_metadata(input.metadata),
                    occurred_at,
                };
                diesel::insert_into(run_events::table)
                    .values(&event)
                    .execute(conn)
                    .await?;
                Ok(id)
            })
            .await?;

        drop(conn);
        self.event(workspace_id, &id.to_string()).await
    }

    pub async fn events(
        &self,
        workspace_id: &str,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<RunEventSummary>, StorageError> {
        let id = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let records = run_events::table
            .filter(run_events::workspace_id.eq(workspace_id))
            .filter(run_events::run_id.eq(id))
            .select(RunEventRecord::as_select())
            .order((run_events::sequence.asc(), run_events::occurred_at.asc()))
            .limit(limit.clamp(1, 200))
            .load::<RunEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("run events: {e}")))?;
        records.into_iter().map(event_summary).collect()
    }

    pub async fn traces(
        &self,
        workspace_id: &str,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<TraceSummary>, StorageError> {
        let id = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let rows = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .filter(traces::run_id.eq(Some(id)))
            .select((
                traces::trace_id,
                traces::run_id,
                traces::run_event_id,
                traces::environment_id,
                traces::domain,
                traces::decision,
                traces::elapsed_ms,
                traces::payload,
                traces::created_at,
            ))
            .order(traces::created_at.desc())
            .limit(limit.clamp(1, 100))
            .load::<(
                Uuid,
                Option<Uuid>,
                Option<Uuid>,
                String,
                String,
                String,
                i32,
                serde_json::Value,
                DateTime<Utc>,
            )>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("run traces: {e}")))?;

        let latest_reviews = latest_review_outcomes(&mut conn, workspace_id, &rows).await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    trace_id,
                    run_id,
                    run_event_id,
                    environment_id,
                    domain,
                    decision,
                    elapsed_ms,
                    payload,
                    created_at,
                )| {
                    let latest_review = latest_reviews.get(&trace_id);
                    TraceSummary {
                        trace_id: trace_id.to_string(),
                        run_id: run_id.map(|id| id.to_string()),
                        run_event_id: run_event_id.map(|id| id.to_string()),
                        environment_id: environment_id.clone(),
                        environment: environment_id,
                        domain,
                        decision,
                        elapsed_ms,
                        latest_review_outcome: latest_review.map(|row| row.0),
                        latest_reviewed_at: latest_review.map(|row| row.1.to_rfc3339()),
                        payload,
                        created_at: created_at.to_rfc3339(),
                    }
                },
            )
            .collect())
    }

    async fn event(
        &self,
        workspace_id: &str,
        event_id: &str,
    ) -> Result<RunEventSummary, StorageError> {
        let id = parse_run_id(event_id)?;
        let mut conn = self.connection().await?;
        let record = run_events::table
            .filter(run_events::workspace_id.eq(workspace_id))
            .filter(run_events::id.eq(id))
            .select(RunEventRecord::as_select())
            .first::<RunEventRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("run event get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        event_summary(record)
    }

    async fn summarize_record(&self, record: RunRecord) -> Result<RunSummary, StorageError> {
        let stats = self.stats(&record.workspace_id, record.id).await?;
        Ok(RunSummary {
            id: record.id.to_string(),
            workspace_id: record.workspace_id,
            environment_id: record.environment_id.clone(),
            environment: record.environment_id,
            agent_id: record.agent_id,
            kind: parse_kind(&record.kind)?,
            status: parse_status(&record.status)?,
            external_id: record.external_id,
            metadata: record.metadata,
            started_at: record.started_at.to_rfc3339(),
            ended_at: record.ended_at.map(|value| value.to_rfc3339()),
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
            trace_count: stats.trace_count,
            blocked_count: stats.blocked_count,
            rewritten_count: stats.rewritten_count,
            escalated_count: stats.escalated_count,
            p95_latency_ms: stats.p95_latency_ms,
        })
    }

    async fn stats(&self, workspace_id: &str, run_id: Uuid) -> Result<RunStats, StorageError> {
        let mut conn = self.connection().await?;
        let rows = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .filter(traces::run_id.eq(Some(run_id)))
            .select((traces::decision, traces::elapsed_ms))
            .load::<(String, i32)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("run stats: {e}")))?;

        let mut latencies = Vec::with_capacity(rows.len());
        let mut stats = RunStats::default();
        for (decision, elapsed_ms) in rows {
            stats.trace_count += 1;
            latencies.push(elapsed_ms);
            match decision.as_str() {
                "block" => stats.blocked_count += 1,
                "rewrite" => stats.rewritten_count += 1,
                "escalate" => stats.escalated_count += 1,
                _ => {}
            }
        }
        stats.p95_latency_ms = p95(latencies);
        Ok(stats)
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
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

#[derive(Default)]
struct RunStats {
    trace_count: i64,
    blocked_count: i64,
    rewritten_count: i64,
    escalated_count: i64,
    p95_latency_ms: Option<i32>,
}

fn parse_run_id(id: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(id).map_err(|e| StorageError::Internal(format!("run_id parse: {e}")))
}

fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        serde_json::json!({})
    } else {
        value
    }
}

fn validate_create_run_event(input: &CreateRunEventRequest) -> Result<(), StorageError> {
    if input.sequence.is_some_and(|sequence| sequence < 1) {
        return Err(StorageError::Internal(
            "sequence must be greater than 0".into(),
        ));
    }
    if let Some(occurred_at) = input.occurred_at.as_ref() {
        DateTime::parse_from_rfc3339(occurred_at)
            .map_err(|_| StorageError::Internal("occurred_at must be RFC 3339".into()))?;
    }
    validate_metadata(&input.metadata)
}

fn validate_metadata(value: &serde_json::Value) -> Result<(), StorageError> {
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(StorageError::Internal(
            "metadata must be a JSON object".into(),
        ))
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn p95(mut latencies: Vec<i32>) -> Option<i32> {
    if latencies.is_empty() {
        return None;
    }
    latencies.sort_unstable();
    let index = ((latencies.len() as f64) * 0.95).ceil() as usize;
    latencies.get(index.saturating_sub(1)).copied()
}

fn kind_text(kind: RunKind) -> &'static str {
    match kind {
        RunKind::ChatSession => "chat_session",
        RunKind::LiveCall => "live_call",
        RunKind::Workflow => "workflow",
        RunKind::Job => "job",
        RunKind::Other => "other",
    }
}

fn parse_kind(value: &str) -> Result<RunKind, StorageError> {
    match value {
        "chat_session" => Ok(RunKind::ChatSession),
        "live_call" => Ok(RunKind::LiveCall),
        "workflow" => Ok(RunKind::Workflow),
        "job" => Ok(RunKind::Job),
        "other" => Ok(RunKind::Other),
        other => Err(StorageError::Internal(format!("unknown run kind: {other}"))),
    }
}

fn status_text(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Warming => "warming",
        RunStatus::Running => "running",
        RunStatus::Completed => "completed",
        RunStatus::Failed => "failed",
        RunStatus::Canceled => "canceled",
    }
}

fn parse_status(value: &str) -> Result<RunStatus, StorageError> {
    match value {
        "warming" => Ok(RunStatus::Warming),
        "running" => Ok(RunStatus::Running),
        "completed" => Ok(RunStatus::Completed),
        "failed" => Ok(RunStatus::Failed),
        "canceled" => Ok(RunStatus::Canceled),
        other => Err(StorageError::Internal(format!(
            "unknown run status: {other}"
        ))),
    }
}

fn event_kind_text(kind: RunEventKind) -> &'static str {
    match kind {
        RunEventKind::UserTurn => "user_turn",
        RunEventKind::AssistantTurn => "assistant_turn",
        RunEventKind::ToolCall => "tool_call",
        RunEventKind::WorkflowStep => "workflow_step",
        RunEventKind::Interruption => "interruption",
        RunEventKind::Retry => "retry",
        RunEventKind::SystemEvent => "system_event",
        RunEventKind::Other => "other",
    }
}

fn parse_event_kind(value: &str) -> Result<RunEventKind, StorageError> {
    match value {
        "user_turn" => Ok(RunEventKind::UserTurn),
        "assistant_turn" => Ok(RunEventKind::AssistantTurn),
        "tool_call" => Ok(RunEventKind::ToolCall),
        "workflow_step" => Ok(RunEventKind::WorkflowStep),
        "interruption" => Ok(RunEventKind::Interruption),
        "retry" => Ok(RunEventKind::Retry),
        "system_event" => Ok(RunEventKind::SystemEvent),
        "other" => Ok(RunEventKind::Other),
        other => Err(StorageError::Internal(format!(
            "unknown run event kind: {other}"
        ))),
    }
}

fn event_summary(record: RunEventRecord) -> Result<RunEventSummary, StorageError> {
    Ok(RunEventSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        run_id: record.run_id.to_string(),
        sequence: record.sequence,
        kind: parse_event_kind(&record.kind)?,
        label: record.label,
        input_summary: record.input_summary,
        output_summary: record.output_summary,
        metadata: record.metadata,
        occurred_at: record.occurred_at.to_rfc3339(),
        created_at: record.created_at.to_rfc3339(),
    })
}

async fn latest_review_outcomes(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    rows: &[TraceReviewLookupRow],
) -> Result<std::collections::HashMap<Uuid, (HumanReviewOutcome, DateTime<Utc>)>, StorageError> {
    let trace_ids = rows.iter().map(|row| row.0).collect::<Vec<_>>();
    if trace_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let review_rows = human_review_events::table
        .filter(human_review_events::workspace_id.eq(workspace_id))
        .filter(human_review_events::trace_id.eq_any(trace_ids))
        .select((
            human_review_events::trace_id,
            human_review_events::outcome,
            human_review_events::created_at,
        ))
        .order(human_review_events::created_at.desc())
        .load::<(Uuid, String, DateTime<Utc>)>(conn)
        .await
        .map_err(|e| StorageError::Internal(format!("run trace latest review: {e}")))?;
    let mut latest = std::collections::HashMap::new();
    for (trace_id, outcome, created_at) in review_rows {
        latest
            .entry(trace_id)
            .or_insert((parse_review_outcome(&outcome)?, created_at));
    }
    Ok(latest)
}

fn parse_review_outcome(value: &str) -> Result<HumanReviewOutcome, StorageError> {
    match value {
        "accepted" => Ok(HumanReviewOutcome::Accepted),
        "corrected" => Ok(HumanReviewOutcome::Corrected),
        "rejected" => Ok(HumanReviewOutcome::Rejected),
        "false_positive" => Ok(HumanReviewOutcome::FalsePositive),
        "missed_issue" => Ok(HumanReviewOutcome::MissedIssue),
        "ignored" => Ok(HumanReviewOutcome::Ignored),
        other => Err(StorageError::Internal(format!(
            "unknown human review outcome: {other}"
        ))),
    }
}

impl std::fmt::Debug for RunRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunRepo").finish_non_exhaustive()
    }
}
