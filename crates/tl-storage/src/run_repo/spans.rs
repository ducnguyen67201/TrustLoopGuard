use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::RunSpanSummary;

use crate::models::RunSpanRecord;
use crate::schema::run_spans;
use crate::StorageError;

use super::validation::parse_run_id;
use super::RunRepo;

impl RunRepo {
    pub async fn spans(
        &self,
        workspace_id: &str,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<RunSpanSummary>, StorageError> {
        let id = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let rows = run_spans::table
            .filter(run_spans::workspace_id.eq(workspace_id))
            .filter(run_spans::run_id.eq(id))
            .select(RunSpanRecord::as_select())
            .order((run_spans::started_at.asc(), run_spans::otel_span_id.asc()))
            .limit(limit.clamp(1, 500))
            .load::<RunSpanRecord>(&mut conn)
            .await
            .map_err(|error| StorageError::Internal(format!("run spans: {error}")))?;

        Ok(rows.into_iter().map(run_span_summary).collect())
    }
}

fn run_span_summary(record: RunSpanRecord) -> RunSpanSummary {
    RunSpanSummary {
        trace_id: record.otel_trace_id,
        span_id: record.otel_span_id,
        parent_span_id: record.parent_span_id,
        agent_id: record.agent_id,
        run_event_id: record.run_event_id.map(|id| id.to_string()),
        name: record.name,
        span_kind: record.span_kind,
        operation_name: record.operation_name,
        conversation_id: record.conversation_id,
        external_agent_id: record.external_agent_id,
        started_at: record.started_at.to_rfc3339(),
        ended_at: record.ended_at.to_rfc3339(),
        status_code: record.status_code,
        status_message: record.status_message,
        resource: record.resource.as_object().cloned().unwrap_or_default(),
        attributes: record.attributes.as_object().cloned().unwrap_or_default(),
        events: record.events.as_array().cloned().unwrap_or_default(),
        links: record.links.as_array().cloned().unwrap_or_default(),
        content_capture_status: record.content_capture_status,
        dropped_attribute_count: record.dropped_attribute_count,
        late_evidence: record.late_evidence,
        ingested_at: record.ingested_at.to_rfc3339(),
    }
}
