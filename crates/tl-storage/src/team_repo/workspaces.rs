use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{InviteStatus, MyWorkspace, WorkspaceRole, DEFAULT_ENVIRONMENT_ID};
use uuid::Uuid;

use super::{
    helpers::{ensure_user_exists, slugify, unique_workspace_slug},
    seed_starter_policies, TeamRepo, WorkspaceDeletionOutcome,
};
use crate::{
    schema::{
        organization_members, organizations, users, workspace_api_keys, workspace_environments,
        workspace_invites, workspace_members, workspaces,
    },
    StorageError,
};

type WorkspaceMembershipRow = (String, String, String, String, String, bool, bool, bool);
type WorkspaceRow = (String, String, String, String, bool, bool, bool);

impl TeamRepo {
    /// Create a fresh organization + workspace pair, with `user_id`
    /// as `owner` on both. Used by the `/welcome` "create your own
    /// workspace" path so a self-serve signup can bootstrap without
    /// an admin invite.
    ///
    /// The slug is derived from `name`; if it collides with an
    /// existing workspace (rare but possible), a short random
    /// suffix is appended. Org and workspace ids are stable
    /// `org_<slug>` / `ws_<slug>` strings so they line up with
    /// existing dashboard workspace ids.
    pub async fn create_workspace(
        &self,
        user_id: Uuid,
        name: &str,
    ) -> Result<MyWorkspace, StorageError> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(StorageError::Internal(
                "workspace name is required".to_string(),
            ));
        }
        let mut conn = self.connection().await?;
        let base_slug = slugify(trimmed_name);
        let slug = unique_workspace_slug(&mut conn, &base_slug).await?;
        let workspace_id = format!("ws_{}", slug.replace('-', "_"));
        let organization_id = format!("org_{}", slug.replace('-', "_"));
        let name_owned = trimmed_name.to_string();

        conn.transaction::<MyWorkspace, StorageError, _>(async |conn| {
            ensure_user_exists(conn, user_id).await?;

            diesel::insert_into(organizations::table)
                .values((
                    organizations::id.eq(&organization_id),
                    organizations::name.eq(&name_owned),
                    organizations::slug.eq(&slug),
                ))
                .execute(conn)
                .await?;

            diesel::insert_into(workspaces::table)
                .values((
                    workspaces::id.eq(&workspace_id),
                    workspaces::organization_id.eq(&organization_id),
                    workspaces::name.eq(&name_owned),
                    workspaces::slug.eq(&slug),
                ))
                .execute(conn)
                .await?;

            diesel::insert_into(workspace_environments::table)
                .values((
                    workspace_environments::workspace_id.eq(&workspace_id),
                    workspace_environments::id.eq(DEFAULT_ENVIRONMENT_ID),
                    workspace_environments::slug.eq(DEFAULT_ENVIRONMENT_ID),
                    workspace_environments::name.eq("Production"),
                    workspace_environments::is_default.eq(true),
                ))
                .execute(conn)
                .await?;

            seed_starter_policies(conn, &workspace_id, DEFAULT_ENVIRONMENT_ID).await?;

            diesel::insert_into(organization_members::table)
                .values((
                    organization_members::organization_id.eq(&organization_id),
                    organization_members::user_id.eq(user_id),
                    organization_members::role.eq(pg_enum!("owner", "organization_role")),
                ))
                .execute(conn)
                .await?;

            diesel::insert_into(workspace_members::table)
                .values((
                    workspace_members::workspace_id.eq(&workspace_id),
                    workspace_members::user_id.eq(user_id),
                    workspace_members::role
                        .eq(pg_enum!(WorkspaceRole::Owner.as_str(), "workspace_role")),
                ))
                .execute(conn)
                .await?;

            Ok(MyWorkspace {
                id: workspace_id,
                slug,
                name: name_owned,
                organization_id,
                role: WorkspaceRole::Owner,
                is_knowledge_base_enabled: false,
                is_attacks_enabled: false,
                is_mcp_gateway_enabled: false,
            })
        })
        .await
    }

    /// Workspaces the user holds membership in. Joins
    /// `workspace_members` to `workspaces` so the dashboard's shell
    /// can render the workspace switcher without a second round trip.
    pub async fn list_workspaces_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MyWorkspace>, StorageError> {
        let mut conn = self.connection().await?;
        let rows: Vec<WorkspaceMembershipRow> = workspace_members::table
            .inner_join(workspaces::table.on(workspaces::id.eq(workspace_members::workspace_id)))
            .filter(workspace_members::user_id.eq(user_id))
            .filter(workspaces::deleted_at.is_null())
            .order(workspaces::name.asc())
            .select((
                workspaces::id,
                workspaces::slug,
                workspaces::name,
                workspaces::organization_id,
                workspace_members::role,
                workspaces::is_knowledge_base_enabled,
                workspaces::is_attacks_enabled,
                workspaces::is_mcp_gateway_enabled,
            ))
            .load::<WorkspaceMembershipRow>(&mut conn)
            .await
            .map_err(|error| StorageError::Internal(format!("list user workspaces: {error}")))?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    slug,
                    name,
                    org_id,
                    role,
                    is_knowledge_base_enabled,
                    is_attacks_enabled,
                    is_mcp_gateway_enabled,
                )| {
                    MyWorkspace {
                        id,
                        slug,
                        name,
                        organization_id: org_id,
                        role: WorkspaceRole::parse(&role).unwrap_or(WorkspaceRole::Viewer),
                        is_knowledge_base_enabled,
                        is_attacks_enabled,
                        is_mcp_gateway_enabled,
                    }
                },
            )
            .collect())
    }

    pub async fn is_platform_admin(&self, user_id: Uuid) -> Result<bool, StorageError> {
        let mut conn = self.connection().await?;
        users::table
            .filter(users::id.eq(user_id))
            .select(users::is_platform_admin)
            .first(&mut conn)
            .await
            .optional()
            .map_err(|error| {
                StorageError::Internal(format!("platform admin lookup failed: {error}"))
            })?
            .ok_or(StorageError::NotFound)
    }

    pub async fn list_all_workspaces(&self) -> Result<Vec<MyWorkspace>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = workspaces::table
            .filter(workspaces::deleted_at.is_null())
            .order(workspaces::name.asc())
            .select((
                workspaces::id,
                workspaces::slug,
                workspaces::name,
                workspaces::organization_id,
                workspaces::is_knowledge_base_enabled,
                workspaces::is_attacks_enabled,
                workspaces::is_mcp_gateway_enabled,
            ))
            .load::<WorkspaceRow>(&mut conn)
            .await
            .map_err(|error| StorageError::Internal(format!("list all workspaces: {error}")))?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    slug,
                    name,
                    organization_id,
                    is_knowledge_base_enabled,
                    is_attacks_enabled,
                    is_mcp_gateway_enabled,
                )| MyWorkspace {
                    id,
                    slug,
                    name,
                    role: WorkspaceRole::Admin,
                    organization_id,
                    is_knowledge_base_enabled,
                    is_attacks_enabled,
                    is_mcp_gateway_enabled,
                },
            )
            .collect())
    }

    /// Soft-delete an active workspace owned by `user_id` and revoke
    /// pending invite and runtime-key access in the same transaction.
    pub async fn delete_workspace(
        &self,
        user_id: Uuid,
        workspace_id: &str,
    ) -> Result<WorkspaceDeletionOutcome, StorageError> {
        let mut conn = self.connection().await?;
        let now = Utc::now();

        conn.transaction::<WorkspaceDeletionOutcome, StorageError, _>(async |conn| {
            workspaces::table
                .filter(workspaces::id.eq(workspace_id))
                .filter(workspaces::deleted_at.is_null())
                .select(workspaces::id)
                .for_update()
                .first::<String>(conn)
                .await
                .optional()?
                .ok_or(StorageError::NotFound)?;

            let role = workspace_members::table
                .filter(workspace_members::workspace_id.eq(workspace_id))
                .filter(workspace_members::user_id.eq(user_id))
                .select(workspace_members::role)
                .for_update()
                .first::<String>(conn)
                .await
                .optional()?;
            if role.as_deref().and_then(WorkspaceRole::parse) != Some(WorkspaceRole::Owner) {
                return Ok(WorkspaceDeletionOutcome::Forbidden);
            }

            diesel::update(
                workspace_invites::table
                    .filter(workspace_invites::workspace_id.eq(workspace_id))
                    .filter(
                        workspace_invites::status
                            .eq(pg_enum!(InviteStatus::Pending.as_str(), "invite_status")),
                    ),
            )
            .set(
                workspace_invites::status
                    .eq(pg_enum!(InviteStatus::Revoked.as_str(), "invite_status")),
            )
            .execute(conn)
            .await?;

            diesel::update(
                workspace_api_keys::table
                    .filter(workspace_api_keys::workspace_id.eq(workspace_id))
                    .filter(workspace_api_keys::status.eq(pg_enum!("active", "api_key_status"))),
            )
            .set((
                workspace_api_keys::status.eq(pg_enum!("revoked", "api_key_status")),
                workspace_api_keys::revoked_at.eq(Some(now)),
            ))
            .execute(conn)
            .await?;

            let deleted_rows = diesel::update(
                workspaces::table
                    .filter(workspaces::id.eq(workspace_id))
                    .filter(workspaces::deleted_at.is_null()),
            )
            .set((
                workspaces::deleted_at.eq(Some(now)),
                workspaces::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;
            if deleted_rows != 1 {
                return Err(StorageError::NotFound);
            }

            Ok(WorkspaceDeletionOutcome::Deleted)
        })
        .await
    }
}
