use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::HumanReviewOutcome;
use uuid::Uuid;

use crate::{postgres::DbConnection, schema::human_review_events, StorageError};

use super::text::parse_review_outcome;

pub(super) type TraceReviewLookupRow = (
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

pub(super) async fn latest_review_outcomes(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    rows: &[TraceReviewLookupRow],
) -> Result<HashMap<Uuid, (HumanReviewOutcome, DateTime<Utc>)>, StorageError> {
    let trace_ids = rows.iter().map(|row| row.0).collect::<Vec<_>>();
    if trace_ids.is_empty() {
        return Ok(HashMap::new());
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
        .map_err(|error| StorageError::Internal(format!("run trace latest review: {error}")))?;
    let mut latest = HashMap::new();
    for (trace_id, outcome, created_at) in review_rows {
        latest
            .entry(trace_id)
            .or_insert((parse_review_outcome(&outcome)?, created_at));
    }
    Ok(latest)
}
