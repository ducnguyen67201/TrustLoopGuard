#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::GatewayReliabilityMode;
use tl_storage::{
    connect_postgres, migrate_postgres,
    models::{NewGatewayProviderConnection, NewGatewayRoute},
    schema::{agents, organizations, workspace_environments, workspaces},
    GatewayRepo, GatewayRoutePatch,
};

#[tokio::test]
async fn fallback_and_secret_persist() {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.expect("migrate");
    let pool = connect_postgres(&url, 8).await.expect("connect");
    {
        let mut conn = pool.get().await.expect("connection");
        diesel::insert_into(organizations::table)
            .values((
                organizations::id.eq("org_gateway"),
                organizations::name.eq("Gateway Org"),
                organizations::slug.eq("gateway-org"),
            ))
            .execute(&mut conn)
            .await
            .expect("organization");
        diesel::insert_into(workspaces::table)
            .values((
                workspaces::id.eq("ws_gateway"),
                workspaces::organization_id.eq("org_gateway"),
                workspaces::name.eq("Gateway Workspace"),
                workspaces::slug.eq("gateway"),
            ))
            .execute(&mut conn)
            .await
            .expect("workspace");
        diesel::insert_into(workspace_environments::table)
            .values((
                workspace_environments::workspace_id.eq("ws_gateway"),
                workspace_environments::id.eq("production"),
                workspace_environments::slug.eq("production"),
                workspace_environments::name.eq("Production"),
                workspace_environments::is_default.eq(true),
            ))
            .execute(&mut conn)
            .await
            .expect("environment");
        diesel::insert_into(agents::table)
            .values((
                agents::workspace_id.eq("ws_gateway"),
                agents::id.eq("agent-a"),
                agents::profile_yaml.eq("agent_id: agent-a"),
                agents::parsed_profile.eq(serde_json::json!({"agent_id": "agent-a"})),
            ))
            .execute(&mut conn)
            .await
            .expect("agent");
    }

    let repo = GatewayRepo::new(pool);
    for (id, key) in [
        ("primary", "sealed-primary"),
        ("fallback-a", "sealed-a"),
        ("fallback-b", "sealed-b"),
    ] {
        repo.create_provider_connection(NewGatewayProviderConnection {
            workspace_id: "ws_gateway".into(),
            id: id.into(),
            display_name: id.into(),
            kind: "openai_compatible".into(),
            base_url: Some(format!("https://{id}.example.com")),
            default_model: format!("{id}-model"),
            encrypted_api_key: key.into(),
        })
        .await
        .expect("provider");
    }
    let route = repo
        .create_gateway_route(NewGatewayRoute {
            workspace_id: "ws_gateway".into(),
            id: "route-a".into(),
            display_name: "Route A".into(),
            provider_connection_id: "primary".into(),
            agent_id: "agent-a".into(),
            reliability_mode: "standard".into(),
            fallback_provider_connection_id: Some("fallback-a".into()),
        })
        .await
        .expect("route");
    assert_eq!(route.reliability_mode, GatewayReliabilityMode::Standard);
    assert_eq!(
        route.fallback_provider_connection_id.as_deref(),
        Some("fallback-a")
    );

    let resolved = repo
        .resolve_gateway_route("ws_gateway", "route-a")
        .await
        .expect("resolve route");
    assert_eq!(resolved.encrypted_api_key, "sealed-primary");
    let fallback = resolved
        .fallback_provider_connection
        .expect("resolved fallback");
    assert_eq!(fallback.connection.id, "fallback-a");
    assert_eq!(fallback.encrypted_api_key, "sealed-a");

    let updated = repo
        .update_gateway_route(
            "ws_gateway",
            "route-a",
            GatewayRoutePatch {
                fallback_provider_connection_id: Some(Some("fallback-b".into())),
                ..GatewayRoutePatch::default()
            },
        )
        .await
        .expect("replace fallback");
    assert_eq!(
        updated.fallback_provider_connection_id.as_deref(),
        Some("fallback-b")
    );

    let cleared = repo
        .update_gateway_route(
            "ws_gateway",
            "route-a",
            GatewayRoutePatch {
                fallback_provider_connection_id: Some(None),
                ..GatewayRoutePatch::default()
            },
        )
        .await
        .expect("clear fallback");
    assert!(cleared.fallback_provider_connection_id.is_none());
}
