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

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        trace_id: &str,
    ) -> Result<Option<tl_core::TraceSummary>, TraceStoreError> {
        self.repo
            .get_by_id(workspace_id, environment_id, trace_id)
            .await
            .map_err(|error| TraceStoreError::Internal(error.to_string()))
            .map(|row| row.map(trace_summary_from_row))
    }

    async fn find_github_integration_marker(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        integration_id: &str,
        min_created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<tl_core::TraceSummary>, TraceStoreError> {
        self.repo
            .find_github_integration_marker(
                workspace_id,
                environment_id,
                agent_id,
                integration_id,
                min_created_at,
            )
            .await
            .map_err(|error| TraceStoreError::Internal(error.to_string()))
            .map(|row| row.map(trace_summary_from_row))
    }

    async fn record(&self, write: TraceWriteRequest) -> Result<(), TraceStoreError> {
        // Best-effort, matching the old inline try_send: a full or closed
        // writer channel drops the trace with a warning, never fails the
        // decision path.
        let trace = trace_write(write);
        let run_id = trace.run_id.clone();
        let workspace_id = trace.workspace_id.clone();
        let environment_id = trace.environment_id.clone();
        if let Err(e) = self.writer_tx.try_send(trace) {
            tracing::warn!(run_id, error = %e, "trace channel full or closed; dropped");
            if let Some(run_id) = run_id {
                if let Err(error) = self
                    .repo
                    .increment_dropped_trace(&workspace_id, &environment_id, &run_id)
                    .await
                {
                    tracing::warn!(run_id, error = %error, "dropped trace counter update failed");
                }
            }
        }
        Ok(())
    }

    async fn record_durable(&self, write: TraceWriteRequest) -> Result<(), TraceStoreError> {
        self.repo
            .insert_durable(trace_write(write))
            .await
            .map_err(|error| TraceStoreError::Internal(error.to_string()))
    }
}

fn trace_write(write: TraceWriteRequest) -> TraceWrite {
    TraceWrite {
        decision: write.decision,
        event: write.event,
        workspace_id: write.workspace_id,
        environment_id: write.environment_id,
        agent_id: write.agent_id,
        run_id: write.run_id,
        run_event_id: write.run_event_id,
        session_id: write.session_id,
        domain: write.domain,
    }
}

fn trace_summary_from_row(row: tl_storage::TraceRow) -> tl_core::TraceSummary {
    tl_core::TraceSummary {
        trace_id: row.trace_id.to_string(),
        agent_id: row.agent_id,
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
