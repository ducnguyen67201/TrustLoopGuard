use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::TraceSummary;
use uuid::Uuid;

use crate::schema::traces;
use crate::StorageError;

use super::reviews::latest_review_outcomes;
use super::summary::{p95, RunStats};
use super::validation::parse_run_id;
use super::RunRepo;

impl RunRepo {
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

    pub(super) async fn stats(
        &self,
        workspace_id: &str,
        run_id: Uuid,
    ) -> Result<RunStats, StorageError> {
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
}
