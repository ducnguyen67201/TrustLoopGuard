//! Persistent CRUD for the `"Escalations"` table.
//!
//! One row per `Decision::Escalate` once persistence is enabled. Lifecycle:
//!
//! ```text
//!   insert_pending  →  pending
//!                       ├─ POST 2xx → mark_sent
//!                       └─ retries exhausted → mark_failed
//! ```
//!
//! Boot drain reads `pending` rows whose `created_at` is older than a
//! threshold so the server picks up escalations that were in flight
//! when a previous process crashed.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::StorageError;

/// Wide row tuple matching the SELECT order in `list_stale_pending`.
/// Aliased to keep clippy's complex-type lint happy.
type EscalationRowTuple = (
    Uuid,
    Uuid,
    String,
    String,
    i32,
    Json<serde_json::Value>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);

/// Materialised view of the `"Escalations"` table — what the worker
/// receives when draining the pending queue at boot, and what audit
/// queries return.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRow {
    pub id: Uuid,
    pub trace_id: Uuid,
    pub webhook_url: String,
    pub status: String,
    pub attempts: i32,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct EscalationRepo {
    pool: PgPool,
}

impl EscalationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Insert a new escalation row in `pending` state. Caller chooses
    /// the `id` (UUIDv7 typically — keeps audit queries time-ordered).
    pub async fn insert_pending(
        &self,
        id: Uuid,
        trace_id: Uuid,
        webhook_url: &str,
        payload: &serde_json::Value,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO "Escalations" (id, trace_id, webhook_url, status, attempts, payload)
            VALUES ($1, $2, $3, 'pending', 0, $4)
            "#,
        )
        .bind(id)
        .bind(trace_id)
        .bind(webhook_url)
        .bind(Json(payload))
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("insert escalation: {e}")))?;
        Ok(())
    }

    pub async fn record_attempt(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query(r#"UPDATE "Escalations" SET attempts = attempts + 1 WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Internal(format!("record_attempt: {e}")))?;
        Ok(())
    }

    pub async fn mark_sent(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query(r#"UPDATE "Escalations" SET status = 'sent', sent_at = NOW() WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Internal(format!("mark_sent: {e}")))?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query(r#"UPDATE "Escalations" SET status = 'failed', sent_at = NOW() WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Internal(format!("mark_failed: {e}")))?;
        Ok(())
    }

    /// Read pending escalations older than `older_than`. The server
    /// boot path uses this to redeliver in-flight escalations that
    /// were interrupted by a crash.
    pub async fn list_stale_pending(
        &self,
        older_than: Duration,
    ) -> Result<Vec<EscalationRow>, StorageError> {
        let cutoff_seconds = older_than.as_secs() as i64;
        let rows: Vec<EscalationRowTuple> = sqlx::query_as(
            r#"
            SELECT id, trace_id, webhook_url, status, attempts, payload, created_at, sent_at
              FROM "Escalations"
             WHERE status = 'pending'
               AND created_at < NOW() - make_interval(secs => $1::DOUBLE PRECISION)
             ORDER BY created_at
            "#,
        )
        .bind(cutoff_seconds as f64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("list_stale_pending: {e}")))?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    trace_id,
                    webhook_url,
                    status,
                    attempts,
                    Json(payload),
                    created_at,
                    sent_at,
                )| {
                    EscalationRow {
                        id,
                        trace_id,
                        webhook_url,
                        status,
                        attempts,
                        payload,
                        created_at,
                        sent_at,
                    }
                },
            )
            .collect())
    }
}

impl std::fmt::Debug for EscalationRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EscalationRepo").finish_non_exhaustive()
    }
}
