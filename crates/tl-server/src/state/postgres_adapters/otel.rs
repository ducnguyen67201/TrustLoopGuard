use std::sync::Arc;

use async_trait::async_trait;
use tl_storage::{OtelIngestBatch, OtelRepo};

use crate::otel::{IngestSpanBatch, IngestSpanResult, OtelStore, OtelStoreError};

pub struct PostgresOtelAdapter(Arc<OtelRepo>);

impl PostgresOtelAdapter {
    pub fn new(repo: Arc<OtelRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl OtelStore for PostgresOtelAdapter {
    async fn ingest(&self, batch: IngestSpanBatch) -> Result<IngestSpanResult, OtelStoreError> {
        let run_id = uuid::Uuid::parse_str(&batch.run_id)
            .map_err(|error| OtelStoreError::Internal(format!("run_id parse: {error}")))?;
        let spans = batch
            .spans
            .into_iter()
            .map(|span| {
                Ok(tl_storage::models::NewRunSpan {
                    workspace_id: batch.workspace_id.clone(),
                    environment_id: batch.environment_id.clone(),
                    run_id,
                    agent_id: span.agent_id,
                    run_event_id: span
                        .run_event_id
                        .as_deref()
                        .map(uuid::Uuid::parse_str)
                        .transpose()
                        .map_err(|error| {
                            OtelStoreError::Internal(format!("run_event_id parse: {error}"))
                        })?,
                    otel_trace_id: span.otel_trace_id,
                    otel_span_id: span.otel_span_id,
                    parent_span_id: span.parent_span_id,
                    name: span.name,
                    span_kind: span.span_kind,
                    operation_name: span.operation_name,
                    conversation_id: span.conversation_id,
                    external_agent_id: span.external_agent_id,
                    started_at: span.started_at,
                    ended_at: span.ended_at,
                    status_code: span.status_code,
                    status_message: span.status_message,
                    resource: span.resource,
                    attributes: span.attributes,
                    events: span.events,
                    links: span.links,
                    content_capture_status: span.content_capture_status,
                    dropped_attribute_count: span.dropped_attribute_count,
                    late_evidence: false,
                })
            })
            .collect::<Result<Vec<_>, OtelStoreError>>()?;
        self.0
            .ingest(OtelIngestBatch {
                workspace_id: batch.workspace_id,
                environment_id: batch.environment_id,
                run_id: batch.run_id,
                flush_receipts: batch.flush_receipts,
                spans,
            })
            .await
            .map(|result| IngestSpanResult {
                accepted_span_count: result.accepted_span_count,
                late_span_count: result.late_span_count,
            })
            .map_err(|error| match error {
                tl_storage::StorageError::NotFound => OtelStoreError::NotFound,
                tl_storage::StorageError::Conflict => OtelStoreError::Conflict,
                tl_storage::StorageError::Internal(message) => OtelStoreError::Internal(message),
            })
    }
}
