//! TeamRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test team_repo

#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{policies, policy_environment_deployments, users},
    PolicyRepo, TeamRepo,
};
use uuid::Uuid;

async fn fresh_repos() -> (
    TeamRepo,
    PolicyRepo,
    testcontainers::ContainerAsync<PostgresImage>,
) {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.expect("migrate");
    let pool = connect_postgres(&url, 4).await.expect("connect");
    (
        TeamRepo::new(pool.clone()),
        PolicyRepo::new(pool),
        container,
    )
}

#[tokio::test]
async fn create_workspace_seeds_enabled_starter_policies() {
    let (team_repo, policy_repo, _container) = fresh_repos().await;
    let user_id = Uuid::new_v4();
    {
        let mut conn = policy_repo.pool().get().await.expect("connection");
        diesel::insert_into(users::table)
            .values((
                users::id.eq(user_id),
                users::username.eq("owner@example.com"),
                users::password_hash.eq("hash"),
            ))
            .execute(&mut conn)
            .await
            .expect("insert user");
    }

    let workspace = team_repo
        .create_workspace(user_id, "Starter Policies")
        .await
        .expect("create workspace");
    assert!(!workspace.is_knowledge_base_enabled);
    assert!(!workspace.is_attacks_enabled);
    assert!(!workspace.is_mcp_gateway_enabled);

    let listed_workspace = team_repo
        .list_workspaces_for_user(user_id)
        .await
        .expect("list workspaces")
        .into_iter()
        .next()
        .expect("created workspace membership");
    assert!(!listed_workspace.is_knowledge_base_enabled);
    assert!(!listed_workspace.is_attacks_enabled);
    assert!(!listed_workspace.is_mcp_gateway_enabled);

    let rows = policy_repo
        .list_records_in_environment(&workspace.id, tl_core::DEFAULT_ENVIRONMENT_ID)
        .await
        .expect("list starter policies");
    let ids: Vec<_> = rows.iter().map(|row| row.policy.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "starter-pii-credit-card",
            "starter-pii-email",
            "starter-pii-ipv4",
            "starter-pii-phone",
            "starter-pii-ssn",
            "starter-prompt-injection",
        ]
    );
    assert!(rows.iter().all(|row| row.enabled));

    let enabled = policy_repo
        .list_enabled_in_environment(&workspace.id, tl_core::DEFAULT_ENVIRONMENT_ID)
        .await
        .expect("list enabled starter policies");
    let enabled_ids: Vec<_> = enabled.iter().map(|policy| policy.id.as_str()).collect();
    assert_eq!(enabled_ids, ids);

    let mut conn = policy_repo.pool().get().await.expect("connection");
    let deployment_enabled: Vec<bool> = policy_environment_deployments::table
        .filter(policy_environment_deployments::workspace_id.eq(&workspace.id))
        .select(policy_environment_deployments::enabled)
        .load(&mut conn)
        .await
        .expect("deployment rows");
    assert_eq!(deployment_enabled.len(), 6);
    assert!(deployment_enabled.iter().all(|enabled| *enabled));

    let workspace_policy_enabled: Vec<bool> = policies::table
        .filter(policies::workspace_id.eq(&workspace.id))
        .select(policies::enabled)
        .load(&mut conn)
        .await
        .expect("policy rows");
    assert_eq!(workspace_policy_enabled.len(), 6);
    assert!(workspace_policy_enabled.iter().all(|enabled| *enabled));
}
