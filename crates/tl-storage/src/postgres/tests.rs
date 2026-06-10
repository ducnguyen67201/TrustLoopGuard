use super::*;
use diesel::migration::MigrationSource;
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

fn run_migrations_before(conn: &mut PgConnection, before_version: &str) {
    conn.applied_migrations()
        .expect("setup migration bookkeeping");
    let mut migrations = MIGRATIONS.migrations().expect("load migrations");
    migrations.sort_by(|left, right| left.name().version().cmp(&right.name().version()));
    let migrations = migrations
        .into_iter()
        .filter(|migration| migration.name().version().to_string().as_str() < before_version)
        .collect::<Vec<_>>();
    conn.run_migrations(&migrations)
        .expect("run prior migrations");
}

fn insert_legacy_orphan_trace(conn: &mut PgConnection) {
    diesel::RunQueryDsl::execute(
        diesel::insert_into(traces::table).values((
            traces::workspace_id.eq("default"),
            traces::trace_id.eq(Uuid::nil()),
            traces::domain.eq("customer_support"),
            traces::decision.eq("allow"),
            traces::elapsed_ms.eq(1),
            traces::payload.eq(serde_json::json!({})),
        )),
        conn,
    )
    .expect("insert legacy orphan trace");
}

fn assert_legacy_orphan_trace_preserved(conn: &mut PgConnection) {
    conn.batch_execute(
        "DO $$
        BEGIN
            PERFORM 1
            FROM workspaces
            WHERE id = 'default'
              AND organization_id = 'org_legacy_runtime';
            IF NOT FOUND THEN
                RAISE EXCEPTION 'legacy default workspace was not backfilled';
            END IF;

            PERFORM 1
            FROM workspace_environments
            WHERE workspace_id = 'default'
              AND id = 'production';
            IF NOT FOUND THEN
                RAISE EXCEPTION 'legacy default environment was not backfilled';
            END IF;

            PERFORM 1
            FROM traces
            WHERE workspace_id = 'default';
            IF NOT FOUND THEN
                RAISE EXCEPTION 'legacy trace was deleted instead of preserved';
            END IF;

            PERFORM 1
            FROM pg_constraint
            WHERE conname = 'traces_environment_fk';
            IF NOT FOUND THEN
                RAISE EXCEPTION 'traces environment foreign key was not added';
            END IF;
        END
        $$;",
    )
    .expect("assert legacy orphan trace preserved");
}

/// Pre-workspace deployments stamped every runtime row with workspace
/// `default` and no `workspaces` row. The environment migration must
/// backfill those workspaces (under `org_legacy_runtime`) instead of
/// deleting customer trace data, then add the environment foreign key.
#[tokio::test]
async fn migrate_backfills_legacy_workspaces_before_environment_fk() {
    let (database_url, _container) = fresh_database_url().await;
    let mut conn = establish(&database_url);
    run_migrations_before(&mut conn, "00000000000018");
    insert_legacy_orphan_trace(&mut conn);
    drop(conn);

    migrate(&database_url)
        .await
        .expect("legacy orphan trace migrates");

    let mut conn = establish(&database_url);
    assert_legacy_orphan_trace_preserved(&mut conn);
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
