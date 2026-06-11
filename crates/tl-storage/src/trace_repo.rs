use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::HumanReviewOutcome;
use uuid::Uuid;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{human_review_events, traces};
use crate::StorageError;

/// Row shape shared by every traces-table select that feeds
/// `latest_review_outcomes` (here and in `run_repo`): trace_id, run_id,
/// run_event_id, session_id, environment_id, domain, decision,
/// elapsed_ms, payload, created_at. Single source of truth — extend it
/// here when the traces select grows a column.
pub(crate) type TraceReviewLookupRow = (
    Uuid,
    Option<Uuid>,
    Option<Uuid>,
    Option<String>,
    String,
    String,
    String,
    i32,
    serde_json::Value,
    DateTime<Utc>,
);

#[derive(Debug, Clone)]
pub struct TraceRow {
    pub trace_id: Uuid,
    pub run_id: Option<Uuid>,
    pub run_event_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub environment_id: String,
    pub domain: String,
    pub decision: String,
    pub elapsed_ms: i32,
    pub latest_review_outcome: Option<HumanReviewOutcome>,
    pub latest_reviewed_at: Option<DateTime<Utc>>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct TraceRepo {
    pool: DbPool,
}

impl TraceRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_recent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        session_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<TraceRow>, StorageError> {
        let limit = limit.clamp(1, 100);
        let mut conn = self.connection().await?;
        let mut query = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .filter(traces::environment_id.eq(environment_id))
            .select((
                traces::trace_id,
                traces::run_id,
                traces::run_event_id,
                traces::session_id,
                traces::environment_id,
                traces::domain,
                traces::decision,
                traces::elapsed_ms,
                traces::payload,
                traces::created_at,
            ))
            .order(traces::created_at.desc())
            .limit(limit)
            .into_boxed();
        if let Some(session_id) = session_id {
            query = query.filter(traces::session_id.eq(session_id));
        }
        let rows = query
            .load::<TraceReviewLookupRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("trace list: {e}")))?;

        let latest_reviews = latest_review_outcomes(&mut conn, workspace_id, &rows).await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    trace_id,
                    run_id,
                    run_event_id,
                    session_id,
                    environment_id,
                    domain,
                    decision,
                    elapsed_ms,
                    payload,
                    created_at,
                )| {
                    let latest_review = latest_reviews.get(&trace_id);
                    TraceRow {
                        trace_id,
                        run_id,
                        run_event_id,
                        session_id,
                        environment_id,
                        domain,
                        decision,
                        elapsed_ms,
                        latest_review_outcome: latest_review.map(|row| row.0),
                        latest_reviewed_at: latest_review.map(|row| row.1),
                        payload,
                        created_at,
                    }
                },
            )
            .collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
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
        .map_err(|e| StorageError::Internal(format!("trace latest review: {e}")))?;
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

impl std::fmt::Debug for TraceRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceRepo").finish_non_exhaustive()
    }
}
