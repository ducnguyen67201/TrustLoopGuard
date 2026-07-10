use super::*;
use diesel::migration::MigrationSource;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;

diesel::table! {
    #[sql_name = "enforcement_profiles"]
    legacy_enforcement_profiles (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        display_name -> Text,
        input_action -> Text,
        output_action -> Text,
        fail_mode -> Text,
        retention_mode -> Text,
        response_mode -> Text,
        fallback_message -> Text,
        max_regenerations -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    #[sql_name = "gateway_routes"]
    legacy_gateway_routes (workspace_id, id) {
        workspace_id -> Text,
        id -> Text,
        display_name -> Text,
        provider_connection_id -> Text,
        agent_id -> Text,
        enforcement_profile_id -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

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

#[tokio::test]
async fn gateway_profile_removal_preserves_existing_routes() {
    use crate::schema::{gateway_provider_connections, gateway_routes, organizations, workspaces};
    use diesel::RunQueryDsl as SyncRunQueryDsl;

    let (database_url, _container) = fresh_database_url().await;
    let mut conn = establish(&database_url);
    run_migrations_before(&mut conn, "00000000000044");
    SyncRunQueryDsl::execute(
        diesel::insert_into(organizations::table).values((
            organizations::id.eq("org_gateway_migration"),
            organizations::name.eq("Gateway migration"),
            organizations::slug.eq("gateway-migration"),
        )),
        &mut conn,
    )
    .expect("insert migration organization");
    SyncRunQueryDsl::execute(
        diesel::insert_into(workspaces::table).values((
            workspaces::id.eq("ws_gateway_migration"),
            workspaces::organization_id.eq("org_gateway_migration"),
            workspaces::name.eq("Gateway migration"),
            workspaces::slug.eq("gateway-migration"),
        )),
        &mut conn,
    )
    .expect("insert migration workspace");
    SyncRunQueryDsl::execute(
        diesel::insert_into(gateway_provider_connections::table).values((
            gateway_provider_connections::workspace_id.eq("ws_gateway_migration"),
            gateway_provider_connections::id.eq("provider"),
            gateway_provider_connections::display_name.eq("Provider"),
            gateway_provider_connections::kind.eq("openai_compatible"),
            gateway_provider_connections::default_model.eq("model"),
            gateway_provider_connections::encrypted_api_key.eq("sealed"),
        )),
        &mut conn,
    )
    .expect("insert migration provider");
    SyncRunQueryDsl::execute(
        diesel::insert_into(legacy_enforcement_profiles::table).values((
            legacy_enforcement_profiles::workspace_id.eq("ws_gateway_migration"),
            legacy_enforcement_profiles::id.eq("profile"),
            legacy_enforcement_profiles::display_name.eq("Profile"),
            legacy_enforcement_profiles::input_action.eq("block"),
            legacy_enforcement_profiles::output_action.eq("block"),
            legacy_enforcement_profiles::fail_mode.eq("closed"),
            legacy_enforcement_profiles::retention_mode.eq("metadata_only"),
            legacy_enforcement_profiles::response_mode.eq("regular"),
            legacy_enforcement_profiles::fallback_message.eq("Blocked"),
            legacy_enforcement_profiles::max_regenerations.eq(0),
        )),
        &mut conn,
    )
    .expect("insert migration profile");
    SyncRunQueryDsl::execute(
        diesel::insert_into(legacy_gateway_routes::table).values((
            legacy_gateway_routes::workspace_id.eq("ws_gateway_migration"),
            legacy_gateway_routes::id.eq("route"),
            legacy_gateway_routes::display_name.eq("Route"),
            legacy_gateway_routes::provider_connection_id.eq("provider"),
            legacy_gateway_routes::agent_id.eq("agent"),
            legacy_gateway_routes::enforcement_profile_id.eq("profile"),
        )),
        &mut conn,
    )
    .expect("insert migration route");

    conn.run_pending_migrations(MIGRATIONS)
        .expect("run profile-removal migration");
    let route = SyncRunQueryDsl::first::<(String, String)>(
        gateway_routes::table
            .filter(gateway_routes::workspace_id.eq("ws_gateway_migration"))
            .filter(gateway_routes::id.eq("route"))
            .select((
                gateway_routes::provider_connection_id,
                gateway_routes::agent_id,
            )),
        &mut conn,
    )
    .expect("load preserved route through the profile-free schema");
    assert_eq!(route, ("provider".to_string(), "agent".to_string()));
}
