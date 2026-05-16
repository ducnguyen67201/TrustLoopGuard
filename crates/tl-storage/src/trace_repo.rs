use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::traces;
use crate::StorageError;

#[derive(Debug, Clone)]
pub struct TraceRow {
    pub trace_id: Uuid,
    pub domain: String,
    pub decision: String,
    pub elapsed_ms: i32,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct TraceRepo {
    pool: DbPool,
}

impl TraceRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_recent(
        &self,
        workspace_id: &str,
        limit: i64,
    ) -> Result<Vec<TraceRow>, StorageError> {
        let limit = limit.clamp(1, 100);
        let mut conn = self.connection().await?;
        let rows = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .select((
                traces::trace_id,
                traces::domain,
                traces::decision,
                traces::elapsed_ms,
                traces::payload,
                traces::created_at,
            ))
            .order(traces::created_at.desc())
            .limit(limit)
            .load::<(Uuid, String, String, i32, serde_json::Value, DateTime<Utc>)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("trace list: {e}")))?;

        Ok(rows
            .into_iter()
            .map(
                |(trace_id, domain, decision, elapsed_ms, payload, created_at)| TraceRow {
                    trace_id,
                    domain,
                    decision,
                    elapsed_ms,
                    payload,
                    created_at,
                },
            )
            .collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for TraceRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceRepo").finish_non_exhaustive()
    }
}
