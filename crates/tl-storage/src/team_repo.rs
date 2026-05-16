//! Workspace team + invite repository.
//!
//! Backs the `/v1/team/*` endpoints. Schema lives in migration 6
//! (`workspace_members`, `workspace_invites`, plus the
//! `workspace_role` and `invite_status` enums).
//!
//! The invite `id` doubles as the bearer token: it's an opaque
//! 32-byte URL-safe random string, generated here, single-use, and
//! invalidated on accept / revoke / expire.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use rand::RngCore;
use tl_core::{InviteStatus, MyWorkspace, WorkspaceInvite, WorkspaceMember, WorkspaceRole};
use uuid::Uuid;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    organization_members, organizations, users, workspace_invites, workspace_members, workspaces,
};
use crate::StorageError;

/// How long a freshly-minted invite stays valid.
pub const INVITE_TTL_DAYS: i64 = 7;

#[derive(Clone)]
pub struct TeamRepo {
    pool: DbPool,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct MemberRow {
    user_id: Uuid,
    role: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct UserNameRow {
    id: Uuid,
    username: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_invites)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InviteRow {
    id: String,
    workspace_id: String,
    email: String,
    role: String,
    status: String,
    invited_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

/// Read-only view returned by [`TeamRepo::lookup_invite`] — joins the
/// invite with its workspace so the public accept page can show the
/// workspace name without a second round trip.
#[derive(Debug, Clone)]
pub struct InviteLookup {
    pub invite: WorkspaceInvite,
    pub workspace_name: String,
    pub workspace_slug: String,
    pub user_exists: bool,
}

impl TeamRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_members(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceMember>, StorageError> {
        let mut conn = self.connection().await?;
        let member_rows = workspace_members::table
            .filter(workspace_members::workspace_id.eq(workspace_id))
            .order(workspace_members::created_at.asc())
            .select(MemberRow::as_select())
            .load::<MemberRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list members: {e}")))?;
        if member_rows.is_empty() {
            return Ok(vec![]);
        }
        let ids: Vec<Uuid> = member_rows.iter().map(|r| r.user_id).collect();
        let users_rows: Vec<UserNameRow> = users::table
            .filter(users::id.eq_any(&ids))
            .select(UserNameRow::as_select())
            .load::<UserNameRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list members.users: {e}")))?;
        let usernames: std::collections::HashMap<Uuid, String> =
            users_rows.into_iter().map(|u| (u.id, u.username)).collect();

        Ok(member_rows
            .into_iter()
            .map(|row| WorkspaceMember {
                user_id: row.user_id.to_string(),
                username: usernames
                    .get(&row.user_id)
                    .cloned()
                    .unwrap_or_else(|| row.user_id.to_string()),
                role: WorkspaceRole::parse(&row.role).unwrap_or(WorkspaceRole::Viewer),
                joined_at: row.created_at.to_rfc3339(),
            })
            .collect())
    }

    pub async fn list_pending_invites(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceInvite>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = workspace_invites::table
            .filter(workspace_invites::workspace_id.eq(workspace_id))
            .filter(workspace_invites::status.eq(InviteStatus::Pending.as_str()))
            .filter(workspace_invites::expires_at.gt(Utc::now()))
            .order(workspace_invites::created_at.desc())
            .select(InviteRow::as_select())
            .load::<InviteRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list invites: {e}")))?;
        Ok(rows.into_iter().map(invite_row_to_wire).collect())
    }

    /// Creates a fresh pending invite. Generates an opaque
    /// 32-byte URL-safe token to use as the row id / accept token.
    /// Returns Conflict if a pending invite already exists for the
    /// same `(workspace_id, email)` pair.
    pub async fn create_invite(
        &self,
        workspace_id: &str,
        email: &str,
        role: WorkspaceRole,
        invited_by_user_id: Option<Uuid>,
    ) -> Result<WorkspaceInvite, StorageError> {
        let mut conn = self.connection().await?;
        let already: Option<InviteRow> = workspace_invites::table
            .filter(workspace_invites::workspace_id.eq(workspace_id))
            .filter(workspace_invites::email.eq(email))
            .filter(workspace_invites::status.eq(InviteStatus::Pending.as_str()))
            .filter(workspace_invites::expires_at.gt(Utc::now()))
            .select(InviteRow::as_select())
            .first::<InviteRow>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("invite lookup: {e}")))?;
        if already.is_some() {
            return Err(StorageError::Conflict);
        }
        let id = generate_token();
        let expires_at = Utc::now() + Duration::days(INVITE_TTL_DAYS);
        let inserted: InviteRow = diesel::insert_into(workspace_invites::table)
            .values((
                workspace_invites::id.eq(&id),
                workspace_invites::workspace_id.eq(workspace_id),
                workspace_invites::email.eq(email),
                workspace_invites::role.eq(role.as_str()),
                workspace_invites::status.eq(InviteStatus::Pending.as_str()),
                workspace_invites::invited_by_user_id.eq(invited_by_user_id),
                workspace_invites::expires_at.eq(expires_at),
            ))
            .returning(InviteRow::as_returning())
            .get_result(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("insert invite: {e}")))?;
        Ok(invite_row_to_wire(inserted))
    }

    pub async fn revoke_invite(
        &self,
        workspace_id: &str,
        invite_id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows = diesel::update(
            workspace_invites::table
                .filter(workspace_invites::id.eq(invite_id))
                .filter(workspace_invites::workspace_id.eq(workspace_id))
                .filter(workspace_invites::status.eq(InviteStatus::Pending.as_str())),
        )
        .set(workspace_invites::status.eq(InviteStatus::Revoked.as_str()))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("revoke invite: {e}")))?;
        if rows == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Public accept-page metadata. Returns the invite, the workspace
    /// it points at, and whether the invitee email already has an
    /// account. Does **not** consume the invite — that's [`Self::accept_invite`].
    pub async fn lookup_invite(&self, invite_id: &str) -> Result<InviteLookup, StorageError> {
        let mut conn = self.connection().await?;
        let invite: InviteRow = workspace_invites::table
            .filter(workspace_invites::id.eq(invite_id))
            .select(InviteRow::as_select())
            .first::<InviteRow>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("invite lookup: {e}")))?
            .ok_or(StorageError::NotFound)?;

        let (workspace_name, workspace_slug): (String, String) = workspaces::table
            .filter(workspaces::id.eq(&invite.workspace_id))
            .select((workspaces::name, workspaces::slug))
            .first::<(String, String)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("invite workspace: {e}")))?;

        let lowered = invite.email.to_ascii_lowercase();
        let user_exists: bool = users::table
            .filter(
                diesel::dsl::sql::<diesel::sql_types::Bool>("LOWER(username) = ")
                    .bind::<diesel::sql_types::Text, _>(lowered),
            )
            .select(users::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("invite user_exists: {e}")))?
            .is_some();

        let mut wire = invite_row_to_wire(invite);
        if wire.status == InviteStatus::Pending
            && DateTime::parse_from_rfc3339(&wire.expires_at)
                .map(|d| d.with_timezone(&Utc) < Utc::now())
                .unwrap_or(false)
        {
            wire.status = InviteStatus::Expired;
        }
        Ok(InviteLookup {
            invite: wire,
            workspace_name,
            workspace_slug,
            user_exists,
        })
    }

    /// Atomically consume a pending invite for `user_id`:
    /// - mark invite accepted
    /// - upsert `workspace_members` with the invited role
    /// - upsert `organization_members` with the default role
    ///
    /// Returns the workspace_id the user just joined.
    pub async fn accept_invite(
        &self,
        invite_id: &str,
        user_id: Uuid,
    ) -> Result<String, StorageError> {
        let mut conn = self.connection().await?;
        let invite_id_owned = invite_id.to_string();
        conn.transaction::<String, StorageError, _>(async |conn| {
            let invite_id = invite_id_owned;
            let invite: InviteRow = workspace_invites::table
                .filter(workspace_invites::id.eq(&invite_id))
                .select(InviteRow::as_select())
                .first::<InviteRow>(conn)
                .await?;
            if invite.status != InviteStatus::Pending.as_str() {
                return Err(StorageError::Conflict);
            }
            if invite.expires_at < Utc::now() {
                diesel::update(
                    workspace_invites::table.filter(workspace_invites::id.eq(&invite_id)),
                )
                .set(workspace_invites::status.eq(InviteStatus::Expired.as_str()))
                .execute(conn)
                .await?;
                return Err(StorageError::Conflict);
            }

            let organization_id: String = workspaces::table
                .filter(workspaces::id.eq(&invite.workspace_id))
                .select(workspaces::organization_id)
                .first::<String>(conn)
                .await?;

            diesel::insert_into(organization_members::table)
                .values((
                    organization_members::organization_id.eq(&organization_id),
                    organization_members::user_id.eq(user_id),
                    organization_members::role.eq("member"),
                ))
                .on_conflict((
                    organization_members::organization_id,
                    organization_members::user_id,
                ))
                .do_nothing()
                .execute(conn)
                .await?;

            diesel::insert_into(workspace_members::table)
                .values((
                    workspace_members::workspace_id.eq(&invite.workspace_id),
                    workspace_members::user_id.eq(user_id),
                    workspace_members::role.eq(invite.role.as_str()),
                ))
                .on_conflict((workspace_members::workspace_id, workspace_members::user_id))
                .do_update()
                .set(workspace_members::role.eq(invite.role.as_str()))
                .execute(conn)
                .await?;

            diesel::update(workspace_invites::table.filter(workspace_invites::id.eq(&invite_id)))
                .set(workspace_invites::status.eq(InviteStatus::Accepted.as_str()))
                .execute(conn)
                .await?;

            let _ = organizations::table;
            Ok(invite.workspace_id)
        })
        .await
    }

    /// Bulk-accept every pending invite addressed to `email`. Run as a
    /// prelude to membership lookups so an admin who invites a user
    /// after that user has already signed up doesn't leave the
    /// invitee stuck on `/welcome`.
    ///
    /// Each accept reuses the same transaction shape as a single
    /// [`Self::accept_invite`]. Returns the number of invites consumed.
    pub async fn accept_pending_invites_for_email(
        &self,
        email: &str,
        user_id: Uuid,
    ) -> Result<usize, StorageError> {
        let mut conn = self.connection().await?;
        let rows = workspace_invites::table
            .filter(workspace_invites::email.eq(email))
            .filter(workspace_invites::status.eq(InviteStatus::Pending.as_str()))
            .filter(workspace_invites::expires_at.gt(Utc::now()))
            .select(workspace_invites::id)
            .load::<String>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("auto-bind list: {e}")))?;
        let mut accepted = 0usize;
        for id in rows {
            match self.accept_invite(&id, user_id).await {
                Ok(_) => accepted += 1,
                // A race condition (concurrent accept/revoke) is benign
                // here — we're best-effort. Don't surface as an error.
                Err(StorageError::Conflict) | Err(StorageError::NotFound) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(accepted)
    }

    /// Create a fresh organization + workspace pair, with `user_id`
    /// as `owner` on both. Used by the `/welcome` "create your own
    /// workspace" path so a self-serve signup can bootstrap without
    /// an admin invite.
    ///
    /// The slug is derived from `name`; if it collides with an
    /// existing workspace (rare but possible), a short random
    /// suffix is appended. Org and workspace ids are stable
    /// `org_<slug>` / `ws_<slug>` strings so they line up with the
    /// dashboard's `workspaceIdFromSlug` convention.
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

            diesel::insert_into(organization_members::table)
                .values((
                    organization_members::organization_id.eq(&organization_id),
                    organization_members::user_id.eq(user_id),
                    organization_members::role.eq("owner"),
                ))
                .execute(conn)
                .await?;

            diesel::insert_into(workspace_members::table)
                .values((
                    workspace_members::workspace_id.eq(&workspace_id),
                    workspace_members::user_id.eq(user_id),
                    workspace_members::role.eq(WorkspaceRole::Owner.as_str()),
                ))
                .execute(conn)
                .await?;

            Ok(MyWorkspace {
                id: workspace_id,
                slug,
                name: name_owned,
                organization_id,
                role: WorkspaceRole::Owner,
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
        let rows: Vec<(String, String, String, String, String)> = workspace_members::table
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
            ))
            .load::<(String, String, String, String, String)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list user workspaces: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|(id, slug, name, org_id, role)| MyWorkspace {
                id,
                slug,
                name,
                organization_id: org_id,
                role: WorkspaceRole::parse(&role).unwrap_or(WorkspaceRole::Viewer),
            })
            .collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

fn invite_row_to_wire(row: InviteRow) -> WorkspaceInvite {
    let status = match row.status.as_str() {
        "pending" => InviteStatus::Pending,
        "accepted" => InviteStatus::Accepted,
        "revoked" => InviteStatus::Revoked,
        _ => InviteStatus::Expired,
    };
    let role = WorkspaceRole::parse(&row.role).unwrap_or(WorkspaceRole::Viewer);
    WorkspaceInvite {
        id: row.id,
        workspace_id: row.workspace_id,
        email: row.email,
        role,
        status,
        invited_by_user_id: row.invited_by_user_id.map(|u| u.to_string()),
        created_at: row.created_at.to_rfc3339(),
        expires_at: row.expires_at.to_rfc3339(),
    }
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Best-effort slug: lowercase, ASCII alphanumerics + hyphens, no
/// leading/trailing hyphens, capped at 48 chars. Empty input or
/// purely non-ASCII input falls back to a short random slug.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_was_dash = true;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            for lower in c.to_lowercase() {
                out.push(lower);
            }
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        let mut bytes = [0u8; 6];
        rand::thread_rng().fill_bytes(&mut bytes);
        out = format!("workspace-{}", URL_SAFE_NO_PAD.encode(bytes));
    }
    if out.len() > 48 {
        out.truncate(48);
        while out.ends_with('-') {
            out.pop();
        }
    }
    out
}

/// Ensures the candidate slug isn't already taken at the workspaces
/// table level. Adds a short random suffix on collision; tries a
/// handful of times before giving up.
async fn unique_workspace_slug(
    conn: &mut crate::postgres::DbConnection<'_>,
    base: &str,
) -> Result<String, StorageError> {
    for attempt in 0..8 {
        let candidate = if attempt == 0 {
            base.to_string()
        } else {
            let mut bytes = [0u8; 3];
            rand::thread_rng().fill_bytes(&mut bytes);
            format!("{}-{}", base, URL_SAFE_NO_PAD.encode(bytes))
        };
        let exists: Option<String> = workspaces::table
            .filter(workspaces::slug.eq(&candidate))
            .select(workspaces::slug)
            .first::<String>(conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("slug check: {e}")))?;
        if exists.is_none() {
            return Ok(candidate);
        }
    }
    Err(StorageError::Internal(
        "could not allocate unique workspace slug".to_string(),
    ))
}
