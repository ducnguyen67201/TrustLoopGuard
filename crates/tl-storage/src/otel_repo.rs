//! Durable OTLP span ingestion. Protocol decoding and privacy normalization
//! live in `tl-server`; this repository owns tenant/run checks and commit.

use chrono::Utc;
use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::models::NewRunSpan;
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{otel_flush_receipts, run_events, run_spans, runs};
use crate::StorageError;

#[derive(Debug)]
pub struct OtelIngestBatch {
    pub workspace_id: String,
    pub environment_id: String,
    pub run_id: String,
    pub flush_id: Option<String>,
    pub rejected_span_count: i32,
    pub spans: Vec<NewRunSpan>,
}

#[derive(Debug, Clone, Copy)]
pub struct OtelIngestResult {
    pub accepted_span_count: i32,
    pub late_span_count: i32,
}

#[derive(Clone)]
pub struct OtelRepo {
    pool: DbPool,
}

impl OtelRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn ingest(
        &self,
        mut batch: OtelIngestBatch,
    ) -> Result<OtelIngestResult, StorageError> {
        let run_id = Uuid::parse_str(&batch.run_id)
            .map_err(|error| StorageError::Internal(format!("run_id parse: {error}")))?;
        let mut conn = self.connection().await?;
        conn.transaction::<OtelIngestResult, StorageError, _>(async |conn| {
            let run = diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(&batch.workspace_id))
                    .filter(runs::environment_id.eq(&batch.environment_id))
                    .filter(runs::id.eq(run_id)),
            )
            .set(runs::updated_at.eq(runs::updated_at))
            .returning((
                runs::capture_status,
                runs::capture_deadline,
                runs::finalized_at,
            ))
            .get_result::<(
                String,
                Option<chrono::DateTime<Utc>>,
                Option<chrono::DateTime<Utc>>,
            )>(conn)
            .await
            .optional()?
            .ok_or(StorageError::NotFound)?;

            let received_at = Utc::now();
            let late = matches!(run.0.as_str(), "complete" | "incomplete")
                || run.1.is_some_and(|deadline| received_at > deadline);
            for span in &mut batch.spans {
                if span.workspace_id != batch.workspace_id
                    || span.environment_id != batch.environment_id
                    || span.run_id != run_id
                {
                    return Err(StorageError::Internal(
                        "normalized span tenant/run mismatch".into(),
                    ));
                }
                span.late_evidence = late;
            }
            let event_ids = batch
                .spans
                .iter()
                .filter_map(|span| span.run_event_id)
                .collect::<std::collections::HashSet<_>>();
            if !event_ids.is_empty() {
                let event_id_values = event_ids.iter().copied().collect::<Vec<_>>();
                let matched = run_events::table
                    .filter(run_events::workspace_id.eq(&batch.workspace_id))
                    .filter(run_events::run_id.eq(run_id))
                    .filter(run_events::id.eq_any(&event_id_values))
                    .select(run_events::id)
                    .load::<Uuid>(conn)
                    .await?
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>();
                if matched != event_ids {
                    return Err(StorageError::Internal(
                        "run_event_id does not belong to correlated run".into(),
                    ));
                }
            }
            let accepted = if batch.spans.is_empty() {
                0
            } else {
                diesel::insert_into(run_spans::table)
                    .values(&batch.spans)
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?
            };
            let accepted = i32::try_from(accepted).unwrap_or(i32::MAX);

            if let Some(flush_id) = batch.flush_id.as_deref() {
                diesel::insert_into(otel_flush_receipts::table)
                    .values((
                        otel_flush_receipts::workspace_id.eq(&batch.workspace_id),
                        otel_flush_receipts::environment_id.eq(&batch.environment_id),
                        otel_flush_receipts::run_id.eq(run_id),
                        otel_flush_receipts::flush_id.eq(flush_id),
                        otel_flush_receipts::accepted_span_count.eq(accepted),
                        otel_flush_receipts::rejected_span_count.eq(batch.rejected_span_count),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;
            }
            diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(&batch.workspace_id))
                    .filter(runs::id.eq(run_id)),
            )
            .set((
                runs::last_evidence_at.eq(received_at),
                runs::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;
            Ok(OtelIngestResult {
                accepted_span_count: accepted,
                late_span_count: if late { accepted } else { 0 },
            })
        })
        .await
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|error| StorageError::Internal(format!("db pool: {error}")))
    }
}

impl std::fmt::Debug for OtelRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OtelRepo").finish_non_exhaustive()
    }
}
