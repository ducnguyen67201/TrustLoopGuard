//! Persistent CRUD for the `escalations` table.
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
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{EscalationRecord, NewEscalation};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::escalations;
use crate::StorageError;

/// Materialised view of the `escalations` table — what the worker
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
    pool: DbPool,
}

impl EscalationRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &DbPool {
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
        let new_escalation = NewEscalation {
            id,
            trace_id,
            webhook_url: webhook_url.to_string(),
            status: "pending".to_string(),
            attempts: 0,
            payload: payload.clone(),
        };
        let mut conn = self.connection().await?;

        diesel::insert_into(escalations::table)
            .values(&new_escalation)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("insert escalation: {e}")))?;
        Ok(())
    }

    pub async fn record_attempt(&self, id: Uuid) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::update(escalations::table.filter(escalations::id.eq(id)))
            .set(escalations::attempts.eq(escalations::attempts + 1))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("record_attempt: {e}")))?;
        Ok(())
    }

    pub async fn mark_sent(&self, id: Uuid) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::update(escalations::table.filter(escalations::id.eq(id)))
            .set((
                escalations::status.eq("sent"),
                escalations::sent_at.eq(Some(Utc::now())),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("mark_sent: {e}")))?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: Uuid) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::update(escalations::table.filter(escalations::id.eq(id)))
            .set((
                escalations::status.eq("failed"),
                escalations::sent_at.eq(Some(Utc::now())),
            ))
            .execute(&mut conn)
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
        let cutoff = Utc::now()
            - chrono::Duration::from_std(older_than)
                .map_err(|e| StorageError::Internal(format!("stale cutoff: {e}")))?;
        let mut conn = self.connection().await?;
        let rows = escalations::table
            .filter(escalations::status.eq("pending"))
            .filter(escalations::created_at.lt(cutoff))
            .select(EscalationRecord::as_select())
            .order(escalations::created_at.asc())
            .load::<EscalationRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list_stale_pending: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|row| EscalationRow {
                id: row.id,
                trace_id: row.trace_id,
                webhook_url: row.webhook_url,
                status: row.status,
                attempts: row.attempts,
                payload: row.payload,
                created_at: row.created_at,
                sent_at: row.sent_at,
            })
            .collect())
    }
}

impl EscalationRepo {
    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for EscalationRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EscalationRepo").finish_non_exhaustive()
    }
}
