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

use crate::models::NewTrace;
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
    async fn put(&self, decision: &Decision) -> Result<(), StorageError> {
        // The trace_id is a UUID string from the engine. Parse here so
        // we get a real `uuid::Uuid` for the partitioned column type.
        let trace_uuid = Uuid::parse_str(&decision.trace_id)
            .map_err(|e| StorageError::Internal(format!("trace_id parse: {e}")))?;

        // domain isn't on Decision today; default to customer_support to
        // match the engine's default. PR 15 will plumb a real domain
        // through once the request envelope carries it consistently.
        let domain = "customer_support";
        let verdict = verdict_text(&decision.verdict);
        let payload = serde_json::to_value(decision)
            .map_err(|e| StorageError::Internal(format!("decision serialize: {e}")))?;
        let new_trace = NewTrace {
            workspace_id: tl_core::DEFAULT_WORKSPACE_ID.to_string(),
            trace_id: trace_uuid,
            run_id: None,
            run_event_id: None,
            domain: domain.to_string(),
            decision: verdict.to_string(),
            elapsed_ms: decision.latency_ms as i32,
            payload,
        };
        let mut conn = self.connection().await?;

        diesel::insert_into(traces::table)
            .values(&new_trace)
            .on_conflict((traces::trace_id, traces::created_at))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("insert trace: {e}")))?;

        Ok(())
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

fn verdict_text(v: &tl_core::Verdict) -> &'static str {
    match v {
        tl_core::Verdict::Allow => "allow",
        tl_core::Verdict::Block => "block",
        tl_core::Verdict::Rewrite => "rewrite",
        tl_core::Verdict::Escalate => "escalate",
    }
}

#[cfg(all(test, feature = "postgres-it"))]
mod tests {
    use super::*;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres as PostgresImage;

    async fn fresh_database_url() -> (String, testcontainers::ContainerAsync<PostgresImage>) {
        let container = PostgresImage::default()
            .start()
            .await
            .expect("postgres container");
        let host = container.get_host().await.expect("host");
        let port = container.get_host_port_ipv4(5432).await.expect("port");
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        (url, container)
    }

    fn establish(database_url: &str) -> PgConnection {
        PgConnection::establish(database_url).expect("connect postgres")
    }

    fn assert_relation_state(conn: &mut PgConnection, relation: &str, should_exist: bool) {
        let condition = if should_exist {
            "IS NULL"
        } else {
            "IS NOT NULL"
        };
        conn.batch_execute(&format!(
            "DO $$
            BEGIN
                IF to_regclass('{relation}') {condition} THEN
                    RAISE EXCEPTION 'unexpected relation state: {relation}';
                END IF;
            END
            $$;"
        ))
        .expect("check relation");
    }

    fn assert_migration_was_recorded(conn: &mut PgConnection) {
        conn.batch_execute(
            "DO $$
            BEGIN
                PERFORM 1
                FROM __diesel_schema_migrations
                WHERE version = '00000000000010';
                IF NOT FOUND THEN
                    RAISE EXCEPTION 'migration was not recorded';
                END IF;
            END
            $$;",
        )
        .expect("check migration");
    }

    fn assert_human_review_schema_exists(conn: &mut PgConnection) {
        for relation in [
            "public.human_review_events",
            "public.human_review_events_workspace_trace_created_idx",
            "public.human_review_events_workspace_created_idx",
            "public.human_review_events_workspace_outcome_created_idx",
        ] {
            assert_relation_state(conn, relation, true);
        }
    }

    fn assert_human_review_schema_missing(conn: &mut PgConnection) {
        for relation in [
            "public.human_review_events",
            "public.human_review_events_workspace_trace_created_idx",
            "public.human_review_events_workspace_created_idx",
            "public.human_review_events_workspace_outcome_created_idx",
        ] {
            assert_relation_state(conn, relation, false);
        }
    }

    fn drop_human_review_schema(conn: &mut PgConnection) {
        conn.batch_execute(&format!("DROP {} IF EXISTS human_review_events", "TABLE"))
            .expect("drop human_review_events");
    }

    #[tokio::test]
    async fn migrate_repairs_recorded_human_review_schema_drift_and_is_idempotent() {
        let (database_url, _container) = fresh_database_url().await;
        migrate(&database_url).await.expect("initial migrate");

        let mut conn = establish(&database_url);
        assert_migration_was_recorded(&mut conn);
        assert_human_review_schema_exists(&mut conn);

        drop_human_review_schema(&mut conn);
        assert_migration_was_recorded(&mut conn);
        assert_human_review_schema_missing(&mut conn);

        migrate(&database_url)
            .await
            .expect("startup migrate repairs drift");
        let mut conn = establish(&database_url);
        assert_migration_was_recorded(&mut conn);
        assert_human_review_schema_exists(&mut conn);

        drop_human_review_schema(&mut conn);
        assert_migration_was_recorded(&mut conn);
        repair_known_schema_drift(&mut conn).expect("direct repair");
        assert_human_review_schema_exists(&mut conn);

        migrate(&database_url)
            .await
            .expect("startup migrate remains idempotent");
        let mut conn = establish(&database_url);
        assert_human_review_schema_exists(&mut conn);
    }
}
