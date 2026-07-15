//! Postgres-backed `DecisionStore` implementation.
//!
//! Schema is defined in Diesel migrations and embedded into the binary.
//! Callers run [`migrate`] once at server boot, then share the async
//! Diesel pool with repositories and the background writer.

use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::bb8::{Pool, PooledConnection};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tl_core::Decision;
use uuid::Uuid;

use crate::schema::traces;
use crate::{DecisionStore, StorageError};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
const HUMAN_REVIEW_EVENTS_DDL: &str =
    include_str!("../migrations/00000000000010_human_review_events/up.sql");

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection<'a> = PooledConnection<'a, AsyncPgConnection>;

pub async fn connect(database_url: &str, max_connections: u32) -> Result<DbPool, StorageError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    Pool::builder()
        .max_size(max_connections)
        .build(manager)
        .await
        .map_err(|e| StorageError::Internal(format!("connect postgres: {e}")))
}

/// Run embedded Diesel migrations. Idempotent via Diesel's
/// `__diesel_schema_migrations` bookkeeping table.
pub async fn migrate(database_url: &str) -> Result<(), StorageError> {
    let database_url = database_url.to_string();
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url)
            .map_err(|e| StorageError::Internal(format!("connect migrations: {e}")))?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| StorageError::Internal(format!("migrate: {e}")))?;
        repair_known_schema_drift(&mut conn)?;
        Ok(())
    })
    .await
    .map_err(|e| StorageError::Internal(format!("migrate task: {e}")))?
}

fn repair_known_schema_drift(conn: &mut PgConnection) -> Result<(), StorageError> {
    // This DDL is intentionally idempotent. It repairs local/dev databases
    // where Diesel recorded migration 10 as applied but the table was later
    // dropped, so a normal run_pending_migrations call will not recreate it.
    conn.batch_execute(HUMAN_REVIEW_EVENTS_DDL)
        .map_err(|e| StorageError::Internal(format!("repair human_review_events: {e}")))?;
    Ok(())
}

#[derive(Clone)]
pub struct PostgresStore {
    pool: DbPool,
}

impl PostgresStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &DbPool {
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
    async fn put(&self, _decision: &Decision) -> Result<(), StorageError> {
        Err(StorageError::Internal(
            "PostgresStore::put cannot infer workspace context; use workspace-scoped trace writer"
                .into(),
        ))
    }

    async fn get(&self, trace_id: &str) -> Result<Decision, StorageError> {
        let trace_uuid = Uuid::parse_str(trace_id)
            .map_err(|e| StorageError::Internal(format!("trace_id parse: {e}")))?;
        let mut conn = self.connection().await?;

        let payload = traces::table
            .filter(traces::trace_id.eq(trace_uuid))
            .select(traces::payload)
            .order(traces::created_at.desc())
            .first::<serde_json::Value>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("select trace: {e}")))?;

        match payload {
            Some(value) => serde_json::from_value(value)
                .map_err(|e| StorageError::Internal(format!("decision deserialize: {e}"))),
            None => Err(StorageError::NotFound),
        }
    }
}

impl PostgresStore {
    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

#[cfg(all(test, feature = "postgres-it"))]
mod tests;
