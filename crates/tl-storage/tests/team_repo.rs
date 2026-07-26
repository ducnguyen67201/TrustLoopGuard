//! TeamRepo integration tests against testcontainers Postgres.
//!
//!   cargo test -p tl-storage --features postgres-it --test team_repo

#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{WorkspaceRole, DEFAULT_ENVIRONMENT_ID};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{
        organization_members, organizations, policies, policy_environment_deployments, users,
        workspace_api_keys, workspace_environments, workspace_invites, workspace_members,
        workspaces,
    },
    AddMemberOutcome, DashboardAdminRepo, PolicyRepo, StorageError, TeamRepo,
    WorkspaceDeletionOutcome,
};
use uuid::Uuid;

async fn fresh_repos() -> (
    TeamRepo,
    PolicyRepo,
    DashboardAdminRepo,
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
        PolicyRepo::new(pool.clone()),
        DashboardAdminRepo::new(pool),
        container,
    )
}

#[tokio::test]
async fn create_workspace_seeds_enabled_starter_policies() {
    let (team_repo, policy_repo, _dashboard_admin_repo, _container) = fresh_repos().await;
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

#[tokio::test]
async fn platform_admin_lookup_and_all_workspace_listing_are_database_authoritative() {
    let (team_repo, policy_repo, _dashboard_admin_repo, _container) = fresh_repos().await;
    let operator_id = Uuid::new_v4();
    let first_owner_id = Uuid::new_v4();
    let second_owner_id = Uuid::new_v4();
    {
        let mut conn = policy_repo.pool().get().await.expect("connection");
        diesel::insert_into(users::table)
            .values(vec![
                (
                    users::id.eq(operator_id),
                    users::username.eq("operator@example.com"),
                    users::password_hash.eq("hash"),
                ),
                (
                    users::id.eq(first_owner_id),
                    users::username.eq("first-owner@example.com"),
                    users::password_hash.eq("hash"),
                ),
                (
                    users::id.eq(second_owner_id),
                    users::username.eq("second-owner@example.com"),
                    users::password_hash.eq("hash"),
                ),
            ])
            .execute(&mut conn)
            .await
            .expect("insert users");
    }

    let first = team_repo
        .create_workspace(first_owner_id, "First Customer")
        .await
        .expect("first workspace");
    let second = team_repo
        .create_workspace(second_owner_id, "Second Customer")
        .await
        .expect("second workspace");

    assert!(!team_repo
        .is_platform_admin(operator_id)
        .await
        .expect("default platform admin state"));
    assert!(team_repo
        .list_workspaces_for_user(operator_id)
        .await
        .expect("operator memberships")
        .is_empty());

    {
        let mut conn = policy_repo.pool().get().await.expect("connection");
        diesel::update(users::table.filter(users::id.eq(operator_id)))
            .set(users::is_platform_admin.eq(true))
            .execute(&mut conn)
            .await
            .expect("grant platform admin");
    }
    assert!(team_repo
        .is_platform_admin(operator_id)
        .await
        .expect("granted platform admin state"));
    let all = team_repo
        .list_all_workspaces()
        .await
        .expect("all active workspaces");
    assert_eq!(
        all.iter()
            .map(|workspace| workspace.id.as_str())
            .collect::<Vec<_>>(),
        vec![first.id.as_str(), second.id.as_str()]
    );
    assert!(all
        .iter()
        .all(|workspace| workspace.role == WorkspaceRole::Admin));
}

#[tokio::test]
async fn delete_workspace_revokes_access_and_retains_history() {
    let (team_repo, policy_repo, dashboard_admin_repo, _container) = fresh_repos().await;
    let owner_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    let outsider_id = Uuid::new_v4();
    {
        let mut conn = policy_repo.pool().get().await.expect("connection");
        diesel::insert_into(users::table)
            .values(vec![
                (
                    users::id.eq(owner_id),
                    users::username.eq("owner@example.com"),
                    users::password_hash.eq("hash"),
                ),
                (
                    users::id.eq(member_id),
                    users::username.eq("admin@example.com"),
                    users::password_hash.eq("hash"),
                ),
                (
                    users::id.eq(outsider_id),
                    users::username.eq("outsider@example.com"),
                    users::password_hash.eq("hash"),
                ),
            ])
            .execute(&mut conn)
            .await
            .expect("insert users");
    }

    let workspace = team_repo
        .create_workspace(owner_id, "Delete Me")
        .await
        .expect("create workspace");
    let add_member = team_repo
        .add_member_or_invite(
            &workspace.id,
            "admin@example.com",
            WorkspaceRole::Admin,
            Some(owner_id),
        )
        .await
        .expect("add member");
    assert!(matches!(add_member, AddMemberOutcome::Added(_)));

    let invite = team_repo
        .create_invite(
            &workspace.id,
            "pending@example.com",
            WorkspaceRole::Viewer,
            Some(owner_id),
        )
        .await
        .expect("create pending invite");
    dashboard_admin_repo
        .create_api_key(
            "key_delete_workspace",
            &workspace.id,
            DEFAULT_ENVIRONMENT_ID,
            "Delete workspace key",
            "tlg_delete",
            "delete-workspace-key-hash",
            Some(owner_id),
            None,
        )
        .await
        .expect("create api key");

    assert_eq!(
        team_repo
            .delete_workspace(member_id, &workspace.id)
            .await
            .expect("admin delete outcome"),
        WorkspaceDeletionOutcome::Forbidden
    );
    assert_eq!(
        team_repo
            .delete_workspace(outsider_id, &workspace.id)
            .await
            .expect("outsider delete outcome"),
        WorkspaceDeletionOutcome::Forbidden
    );
    assert_eq!(
        team_repo
            .list_pending_invites(&workspace.id)
            .await
            .expect("pending invites before owner delete")
            .len(),
        1
    );
    assert!(dashboard_admin_repo
        .verify_api_key_hash("delete-workspace-key-hash")
        .await
        .expect("verify key before owner delete")
        .is_some());
    assert_eq!(
        team_repo
            .list_workspaces_for_user(owner_id)
            .await
            .expect("owner workspaces before delete")
            .len(),
        1
    );

    assert_eq!(
        team_repo
            .delete_workspace(owner_id, &workspace.id)
            .await
            .expect("owner delete outcome"),
        WorkspaceDeletionOutcome::Deleted
    );

    assert!(team_repo
        .list_workspaces_for_user(owner_id)
        .await
        .expect("owner workspaces after delete")
        .is_empty());
    assert!(team_repo
        .list_workspaces_for_user(member_id)
        .await
        .expect("member workspaces after delete")
        .is_empty());
    assert!(team_repo
        .list_members(&workspace.id)
        .await
        .expect("members after delete")
        .is_empty());
    assert!(team_repo
        .list_pending_invites(&workspace.id)
        .await
        .expect("pending invites after delete")
        .is_empty());
    assert!(dashboard_admin_repo
        .verify_api_key_hash("delete-workspace-key-hash")
        .await
        .expect("verify key after delete")
        .is_none());

    let retained_policies = policy_repo
        .list_records_in_environment(&workspace.id, DEFAULT_ENVIRONMENT_ID)
        .await
        .expect("retained starter policies");
    assert_eq!(retained_policies.len(), 6);

    let mut conn = policy_repo.pool().get().await.expect("connection");
    let deleted_at = workspaces::table
        .filter(workspaces::id.eq(&workspace.id))
        .select(workspaces::deleted_at)
        .first::<Option<chrono::DateTime<chrono::Utc>>>(&mut conn)
        .await
        .expect("workspace retained");
    assert!(deleted_at.is_some());

    let invite_status = workspace_invites::table
        .filter(workspace_invites::id.eq(&invite.id))
        .select(workspace_invites::status)
        .first::<String>(&mut conn)
        .await
        .expect("invite retained");
    assert_eq!(invite_status, "revoked");

    let (key_status, revoked_at) = workspace_api_keys::table
        .filter(workspace_api_keys::id.eq("key_delete_workspace"))
        .select((workspace_api_keys::status, workspace_api_keys::revoked_at))
        .first::<(String, Option<chrono::DateTime<chrono::Utc>>)>(&mut conn)
        .await
        .expect("api key retained");
    assert_eq!(key_status, "revoked");
    assert!(revoked_at.is_some());

    let organization_count = organizations::table
        .filter(organizations::id.eq(&workspace.organization_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .expect("organization count");
    let organization_member_count = organization_members::table
        .filter(organization_members::organization_id.eq(&workspace.organization_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .expect("organization member count");
    let workspace_member_count = workspace_members::table
        .filter(workspace_members::workspace_id.eq(&workspace.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .expect("workspace member count");
    let environment_count = workspace_environments::table
        .filter(workspace_environments::workspace_id.eq(&workspace.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .expect("environment count");
    let policy_count = policies::table
        .filter(policies::workspace_id.eq(&workspace.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .expect("policy count");
    let deployment_count = policy_environment_deployments::table
        .filter(policy_environment_deployments::workspace_id.eq(&workspace.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .expect("deployment count");

    assert_eq!(organization_count, 1);
    assert_eq!(organization_member_count, 2);
    assert_eq!(workspace_member_count, 2);
    assert_eq!(environment_count, 1);
    assert_eq!(policy_count, 6);
    assert_eq!(deployment_count, 6);

    assert!(matches!(
        team_repo.delete_workspace(owner_id, &workspace.id).await,
        Err(StorageError::NotFound)
    ));
}
