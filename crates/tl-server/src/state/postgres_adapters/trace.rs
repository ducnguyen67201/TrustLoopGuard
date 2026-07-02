use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_storage::{TraceRepo, TraceWrite};
use tokio::sync::mpsc;

use crate::traces::{TraceStore, TraceStoreError, TraceWriteRequest};

pub struct PostgresTraceAdapter {
    repo: Arc<TraceRepo>,
    /// Channel into the background batched trace writer.
    writer_tx: mpsc::Sender<TraceWrite>,
}

impl PostgresTraceAdapter {
    pub fn new(repo: Arc<TraceRepo>, writer_tx: mpsc::Sender<TraceWrite>) -> Arc<Self> {
        Arc::new(Self { repo, writer_tx })
    }
}

#[async_trait]
impl TraceStore for PostgresTraceAdapter {
    async fn list_recent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<tl_core::TraceSummary>, TraceStoreError> {
        self.repo
            .list_recent(workspace_id, environment_id, session_id, limit as i64)
            .await
            .map_err(|error| TraceStoreError::Internal(error.to_string()))
            .map(|rows| rows.into_iter().map(trace_summary_from_row).collect())
    }

    async fn sum_payment_minor_since(
        &self,
        workspace_id: &str,
        owner: &str,
        operations: &[String],
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, TraceStoreError> {
        self.repo
            .sum_payment_minor_since(workspace_id, owner, operations, since)
            .await
            .map_err(|error| TraceStoreError::Internal(error.to_string()))
    }

    async fn record(&self, write: TraceWriteRequest) -> Result<(), TraceStoreError> {
        // Best-effort, matching the old inline try_send: a full or closed
        // writer channel drops the trace with a warning, never fails the
        // decision path.
        let trace = TraceWrite {
            decision: write.decision,
            event: write.event,
            workspace_id: write.workspace_id,
            environment_id: write.environment_id,
            run_id: write.run_id,
            run_event_id: write.run_event_id,
            session_id: write.session_id,
            domain: write.domain,
        };
        if let Err(e) = self.writer_tx.try_send(trace) {
            tracing::warn!(error = %e, "trace channel full or closed; dropped");
        }
        Ok(())
    }
}

fn trace_summary_from_row(row: tl_storage::TraceRow) -> tl_core::TraceSummary {
    tl_core::TraceSummary {
        trace_id: row.trace_id.to_string(),
        run_id: row.run_id.map(|id| id.to_string()),
        run_event_id: row.run_event_id.map(|id| id.to_string()),
        session_id: row.session_id,
        environment_id: row.environment_id.clone(),
        environment: row.environment_id,
        domain: row.domain,
        decision: row.decision,
        elapsed_ms: row.elapsed_ms,
        latest_review_outcome: row.latest_review_outcome,
        latest_reviewed_at: row.latest_reviewed_at.map(|value| value.to_rfc3339()),
        payload: row.payload,
        created_at: row.created_at.to_rfc3339(),
    }
}
