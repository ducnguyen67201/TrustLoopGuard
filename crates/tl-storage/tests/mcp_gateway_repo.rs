#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::McpGatewayAuthKind;
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{agents, organizations, users, workspace_members, workspaces},
    CatalogToolInput, McpGatewayRepo, NewMcpConnection, StorageError,
};
use uuid::Uuid;

#[tokio::test]
async fn catalog_and_assignments_are_workspace_scoped() {
    let container = PostgresImage::default().start().await.unwrap();
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.unwrap();
    let pool = connect_postgres(&url, 8).await.unwrap();
    let user = Uuid::new_v4();
    let mut conn = pool.get().await.unwrap();
    diesel::insert_into(organizations::table)
        .values((
            organizations::id.eq("org"),
            organizations::name.eq("Org"),
            organizations::slug.eq("org"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    for workspace in ["one", "two"] {
        diesel::insert_into(workspaces::table)
            .values((
                workspaces::id.eq(workspace),
                workspaces::organization_id.eq("org"),
                workspaces::name.eq(workspace),
                workspaces::slug.eq(workspace),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }
    diesel::insert_into(users::table)
        .values((
            users::id.eq(user),
            users::username.eq("member@example.com"),
            users::password_hash.eq("hash"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    diesel::insert_into(workspace_members::table)
        .values((
            workspace_members::workspace_id.eq("one"),
            workspace_members::user_id.eq(user),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    diesel::insert_into(agents::table)
        .values((
            agents::workspace_id.eq("one"),
            agents::id.eq("customer-agent"),
            agents::profile_yaml.eq("id: customer-agent"),
            agents::parsed_profile.eq(serde_json::json!({"agent_id":"customer-agent"})),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    drop(conn);
    let repo = McpGatewayRepo::new(pool);
    let connection_id = Uuid::new_v4();
    repo.create_connection(NewMcpConnection {
        workspace_id: "one".into(),
        id: connection_id,
        display_name: "Example".into(),
        server_slug: "example".into(),
        endpoint_url: "https://mcp.example/mcp".into(),
        auth_kind: McpGatewayAuthKind::None,
        encrypted_credential: None,
        enabled: true,
    })
    .await
    .unwrap();
    repo.replace_catalog_snapshot(
        "one",
        connection_id,
        vec![CatalogToolInput {
            upstream_name: "search".into(),
            public_name: "example__search".into(),
            title: None,
            description: None,
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: None,
            annotations: serde_json::json!({}),
            schema_hash: "sha256:v1:test".into(),
        }],
    )
    .await
    .unwrap();
    let tool = repo.list_tools("one").await.unwrap().remove(0);
    let tool_id = Uuid::parse_str(&tool.id).unwrap();
    repo.replace_agent_assignments("one", tool_id, "customer-agent", vec![user, user], None)
        .await
        .unwrap();
    assert_eq!(
        repo.list_entitled_tools("one", user, "customer-agent", None, 101)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(repo
        .list_entitled_tools("one", user, "other-agent", None, 101)
        .await
        .unwrap()
        .is_empty());
    let listed = repo.list_tools("one").await.unwrap().remove(0);
    assert_eq!(listed.agent_assignments.len(), 1);
    assert!(listed.unbound_user_ids.is_empty());
    assert!(matches!(
        repo.resolve_entitled_tool("two", user, "customer-agent", "example__search")
            .await,
        Err(StorageError::NotFound)
    ));

    repo.replace_agent_assignments("one", tool_id, "customer-agent", vec![], None)
        .await
        .unwrap();
    let revoked = repo.list_tools("one").await.unwrap().remove(0);
    assert!(revoked.assigned_user_ids.is_empty());
}
