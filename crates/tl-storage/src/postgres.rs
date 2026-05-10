//! Postgres-backed `DecisionStore` implementation.
//!
//! Schema is defined in `migrations/0001_init.sql` and embedded into the
//! binary via `sqlx::migrate!`. Callers run `PostgresStore::migrate(&pool)`
//! once at server boot.
//!
//! The PR 12 batched writer + AgentRepo extends this module with async
//! batched inserts and per-agent CRUD; this PR ships only the trait impl
//! that satisfies `DecisionStore::put` / `get` synchronously per call.

use sqlx::postgres::PgPool;
use sqlx::types::Json;
use tl_core::Decision;

use crate::{DecisionStore, StorageError};

/// Run migrations against `pool`. Idempotent — sqlx records applied
/// migrations in `_sqlx_migrations`, so repeat invocations no-op.
pub async fn migrate(pool: &PgPool) -> Result<(), StorageError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| StorageError::Internal(format!("migrate: {e}")))?;
    Ok(())
}

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl std::fmt::Debug for PostgresStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresStore").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl DecisionStore for PostgresStore {
    async fn put(&self, decision: &Decision) -> Result<(), StorageError> {
        // The trace_id is a UUID string from the engine. Parse here so
        // we get a real `uuid::Uuid` for the partitioned column type.
        let trace_uuid = uuid::Uuid::parse_str(&decision.trace_id)
            .map_err(|e| StorageError::Internal(format!("trace_id parse: {e}")))?;

        // domain isn't on Decision today; default to customer_support to
        // match the engine's default. PR 15 will plumb a real domain
        // through once the request envelope carries it consistently.
        let domain = "customer_support";
        let verdict = verdict_text(&decision.verdict);
        let payload = Json(
            serde_json::to_value(decision)
                .map_err(|e| StorageError::Internal(format!("decision serialize: {e}")))?,
        );

        sqlx::query(
            r#"
            INSERT INTO "Traces" (trace_id, domain, decision, elapsed_ms, payload)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (trace_id, created_at) DO NOTHING
            "#,
        )
        .bind(trace_uuid)
        .bind(domain)
        .bind(verdict)
        .bind(decision.latency_ms as i32)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("insert trace: {e}")))?;

        Ok(())
    }

    async fn get(&self, trace_id: &str) -> Result<Decision, StorageError> {
        let trace_uuid = uuid::Uuid::parse_str(trace_id)
            .map_err(|e| StorageError::Internal(format!("trace_id parse: {e}")))?;

        let row: Option<(Json<Decision>,)> = sqlx::query_as(
            r#"SELECT payload FROM "Traces" WHERE trace_id = $1 ORDER BY created_at DESC LIMIT 1"#,
        )
        .bind(trace_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("select trace: {e}")))?;

        match row {
            Some((Json(d),)) => Ok(d),
            None => Err(StorageError::NotFound),
        }
    }
}

fn verdict_text(v: &tl_core::Verdict) -> &'static str {
    match v {
        tl_core::Verdict::Allow => "allow",
        tl_core::Verdict::Block => "block",
        tl_core::Verdict::Rewrite => "rewrite",
        tl_core::Verdict::Escalate => "escalate",
    }
}
