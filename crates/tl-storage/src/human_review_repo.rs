use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{CreateHumanReviewEventRequest, HumanReviewEvent};
use uuid::Uuid;

use crate::models::{HumanReviewEventRecord, NewHumanReviewEvent};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{human_review_events, traces};
use crate::StorageError;

mod analytics;
mod analytics_query;
mod summary;
mod text;
mod validation;

pub use analytics_query::HumanReviewAnalyticsFilter;
use summary::event_summary;
use text::outcome_text;
use validation::{
    clean_reason_codes, non_empty_string, normalize_metadata, parse_uuid, validate_create_event,
};

#[derive(Clone)]
pub struct HumanReviewRepo {
    pool: DbPool,
}

impl HumanReviewRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<HumanReviewEvent, StorageError> {
        validate_create_event(&input)?;
        let trace_uuid = parse_uuid("trace_id", trace_id)?;
        let mut conn = self.connection().await?;
        let trace = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .filter(traces::trace_id.eq(trace_uuid))
            .select((traces::run_id, traces::run_event_id))
            .order(traces::created_at.desc())
            .first::<(Option<Uuid>, Option<Uuid>)>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("review trace lookup: {e}")))?
            .ok_or(StorageError::NotFound)?;

        let id = Uuid::now_v7();
        let event = NewHumanReviewEvent {
            workspace_id: workspace_id.to_string(),
            id,
            trace_id: trace_uuid,
            run_id: trace.0,
            run_event_id: trace.1,
            outcome: outcome_text(input.outcome).to_string(),
            reviewer_id: reviewer_id.and_then(|value| non_empty_string(value.trim())),
            reason_codes: serde_json::json!(clean_reason_codes(input.reason_codes)),
            note: input.note.and_then(|value| non_empty_string(value.trim())),
            metadata: normalize_metadata(input.metadata),
        };
        diesel::insert_into(human_review_events::table)
            .values(&event)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review event insert: {e}")))?;

        drop(conn);
        self.event(workspace_id, id).await
    }

    pub async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: i64,
    ) -> Result<Vec<HumanReviewEvent>, StorageError> {
        let trace_uuid = parse_uuid("trace_id", trace_id)?;
        let mut conn = self.connection().await?;
        let records = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .filter(human_review_events::trace_id.eq(trace_uuid))
            .select(HumanReviewEventRecord::as_select())
            .order(human_review_events::created_at.asc())
            .limit(limit.clamp(1, 100))
            .load::<HumanReviewEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review event list: {e}")))?;
        records.into_iter().map(event_summary).collect()
    }

    pub async fn latest_by_trace_ids(
        &self,
        workspace_id: &str,
        trace_ids: &[String],
    ) -> Result<HashMap<String, HumanReviewEvent>, StorageError> {
        if trace_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let trace_uuids = trace_ids
            .iter()
            .map(|id| parse_uuid("trace_id", id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut conn = self.connection().await?;
        let records = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .filter(human_review_events::trace_id.eq_any(trace_uuids))
            .select(HumanReviewEventRecord::as_select())
            .order(human_review_events::created_at.desc())
            .load::<HumanReviewEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review latest: {e}")))?;

        let mut latest = HashMap::new();
        for record in records {
            let key = record.trace_id.to_string();
            latest.entry(key).or_insert(event_summary(record)?);
        }
        Ok(latest)
    }

    async fn event(&self, workspace_id: &str, id: Uuid) -> Result<HumanReviewEvent, StorageError> {
        let mut conn = self.connection().await?;
        let record = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .filter(human_review_events::id.eq(id))
            .select(HumanReviewEventRecord::as_select())
            .first::<HumanReviewEventRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("review event get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        event_summary(record)
    }

    pub(super) async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for HumanReviewRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HumanReviewRepo").finish_non_exhaustive()
    }
}
