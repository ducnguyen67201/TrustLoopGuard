//! Workspace team + invite repository.
//!
//! Backs the `/v1/team/*` endpoints. Schema lives in migration 6
//! (`workspace_members`, `workspace_invites`, plus the
//! `workspace_role` and `invite_status` enums).
//!
//! Invite ids are opaque 32-byte URL-safe random strings generated here.
//! Pending invites are consumed by matching the invite email to the signed-in
//! user during workspace lookup, then invalidated on accept / revoke / expire.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use rand::RngCore;
use tl_core::{
    InviteStatus, MyWorkspace, WorkspaceInvite, WorkspaceMember, WorkspaceRole,
    DEFAULT_ENVIRONMENT_ID,
};
use uuid::Uuid;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    entity_versions, organization_members, organizations, policies, policy_environment_deployments,
    users, workspace_environments, workspace_invites, workspace_members, workspaces,
};
use crate::StorageError;

macro_rules! pg_enum {
    ($value:expr, $pg_type:literal) => {
        diesel::dsl::sql::<diesel::sql_types::Text>("")
            .bind::<diesel::sql_types::Text, _>($value)
            .sql(concat!("::", $pg_type))
    };
}

/// How long a freshly-minted invite stays valid.
pub const INVITE_TTL_DAYS: i64 = 7;

const STARTER_POLICY_YAMLS: &[&str] = &[
    r#"
id: starter-pii-email
description: Blocks drafts that expose an email address.
match:
  regex: "(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}"
action: block
severity: high
"#,
    r#"
id: starter-pii-phone
description: Blocks drafts that expose a US-style phone number.
match:
  regex: "\\b(?:\\+?1[-.\\s]?)?(?:\\(?\\d{3}\\)?[-.\\s]?)\\d{3}[-.\\s]?\\d{4}\\b"
action: block
severity: high
"#,
    r#"
id: starter-pii-ssn
description: Blocks drafts that expose a US Social Security number.
match:
  regex: "\\b\\d{3}-\\d{2}-\\d{4}\\b"
action: block
severity: critical
"#,
    r#"
id: starter-pii-credit-card
description: Blocks drafts that expose a likely payment card number.
match:
  regex: "\\b(?:\\d[ -]*?){13,19}\\b"
action: block
severity: critical
"#,
    r#"
id: starter-pii-ipv4
description: Blocks drafts that expose an IPv4 address.
match:
  regex: "\\b(?:(?:25[0-5]|2[0-4]\\d|1?\\d?\\d)\\.){3}(?:25[0-5]|2[0-4]\\d|1?\\d?\\d)\\b"
action: block
severity: medium
"#,
    r#"
id: starter-prompt-injection
description: Escalates drafts that contain common prompt-injection phrases.
match:
  regex: "(?i)(ignore previous instructions|ignore all previous instructions|ignore the above|disregard the above|reveal (the )?(system )?prompt|developer message)"
action: escalate
severity: high
"#,
];

/// Outcome of `add_member_or_invite`. The caller dispatches on the
/// variant to tell the admin whether the person was joined now or
/// will join on their next signup.
#[derive(Debug, Clone)]
pub enum AddMemberOutcome {
    /// The email matched an existing user; they're now a member.
    Added(WorkspaceMember),
    /// No account yet for that email — we recorded a pending intent
    /// that auto-binds on signup.
    Invited(WorkspaceInvite),
}

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
            .filter(
                workspace_invites::status
                    .eq(pg_enum!(InviteStatus::Pending.as_str(), "invite_status")),
            )
            .filter(workspace_invites::expires_at.gt(Utc::now()))
            .order(workspace_invites::created_at.desc())
            .select(InviteRow::as_select())
            .load::<InviteRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("list invites: {e}")))?;
        Ok(rows.into_iter().map(invite_row_to_wire).collect())
    }

    /// Creates a fresh pending invite. Generates an opaque
    /// 32-byte URL-safe string to use as the row id.
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
            .filter(
                workspace_invites::status
                    .eq(pg_enum!(InviteStatus::Pending.as_str(), "invite_status")),
            )
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
                workspace_invites::role.eq(pg_enum!(role.as_str(), "workspace_role")),
                workspace_invites::status
                    .eq(pg_enum!(InviteStatus::Pending.as_str(), "invite_status")),
                workspace_invites::invited_by_user_id.eq(invited_by_user_id),
                workspace_invites::expires_at.eq(expires_at),
            ))
            .returning(InviteRow::as_returning())
            .get_result(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("insert invite: {e}")))?;
        Ok(invite_row_to_wire(inserted))
    }

    /// Smart admin path. If `email` matches an existing user (case-
    /// insensitive on `users.username`), we add them to the workspace
    /// immediately and skip the invite row. Otherwise we create a
    /// pending invite — same row shape `accept_pending_invites_for_email`
    /// reads on the user's first dashboard request after signup.
    ///
    /// The single transaction either inserts both
    /// `organization_members` (as `member`) + `workspace_members` (at
    /// the invited role), OR it inserts the pending invite row.
    pub async fn add_member_or_invite(
        &self,
        workspace_id: &str,
        email: &str,
        role: WorkspaceRole,
        invited_by_user_id: Option<Uuid>,
    ) -> Result<AddMemberOutcome, StorageError> {
        let lowered_email = email.to_ascii_lowercase();
        let mut conn = self.connection().await?;

        let existing_user: Option<(Uuid, String)> = users::table
            .filter(
                diesel::dsl::sql::<diesel::sql_types::Bool>("LOWER(username) = ")
                    .bind::<diesel::sql_types::Text, _>(&lowered_email),
            )
            .select((users::id, users::username))
            .first::<(Uuid, String)>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("user lookup: {e}")))?;

        if let Some((user_id, username)) = existing_user {
            let organization_id: String = workspaces::table
                .filter(workspaces::id.eq(workspace_id))
                .select(workspaces::organization_id)
                .first::<String>(&mut conn)
                .await
                .map_err(|e| StorageError::Internal(format!("workspace org lookup: {e}")))?;

            let role_owned = role;
            let workspace_id_owned = workspace_id.to_string();
            let username_owned = username.clone();

            let member: WorkspaceMember = conn
                .transaction::<WorkspaceMember, StorageError, _>(async |conn| {
                    diesel::insert_into(organization_members::table)
                        .values((
                            organization_members::organization_id.eq(&organization_id),
                            organization_members::user_id.eq(user_id),
                            organization_members::role.eq(pg_enum!("member", "organization_role")),
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
                            workspace_members::workspace_id.eq(&workspace_id_owned),
                            workspace_members::user_id.eq(user_id),
                            workspace_members::role
                                .eq(pg_enum!(role_owned.as_str(), "workspace_role")),
                        ))
                        .on_conflict((workspace_members::workspace_id, workspace_members::user_id))
                        .do_update()
                        .set(
                            workspace_members::role
                                .eq(pg_enum!(role_owned.as_str(), "workspace_role")),
                        )
                        .execute(conn)
                        .await?;

                    Ok(WorkspaceMember {
                        user_id: user_id.to_string(),
                        username: username_owned,
                        role: role_owned,
                        joined_at: Utc::now().to_rfc3339(),
                    })
                })
                .await?;

            return Ok(AddMemberOutcome::Added(member));
        }

        let invite = self
            .create_invite(workspace_id, email, role, invited_by_user_id)
            .await?;
        Ok(AddMemberOutcome::Invited(invite))
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
                .filter(
                    workspace_invites::status
                        .eq(pg_enum!(InviteStatus::Pending.as_str(), "invite_status")),
                ),
        )
        .set(
            workspace_invites::status.eq(pg_enum!(InviteStatus::Revoked.as_str(), "invite_status")),
        )
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("revoke invite: {e}")))?;
        if rows == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
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
            ensure_user_exists(conn, user_id).await?;

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
                .set(
                    workspace_invites::status
                        .eq(pg_enum!(InviteStatus::Expired.as_str(), "invite_status")),
                )
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
                    organization_members::role.eq(pg_enum!("member", "organization_role")),
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
                    workspace_members::role.eq(pg_enum!(invite.role.as_str(), "workspace_role")),
                ))
                .on_conflict((workspace_members::workspace_id, workspace_members::user_id))
                .do_update()
                .set(workspace_members::role.eq(pg_enum!(invite.role.as_str(), "workspace_role")))
                .execute(conn)
                .await?;

            diesel::update(workspace_invites::table.filter(workspace_invites::id.eq(&invite_id)))
                .set(
                    workspace_invites::status
                        .eq(pg_enum!(InviteStatus::Accepted.as_str(), "invite_status")),
                )
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
        ensure_oauth_user_exists(&mut conn, user_id, email).await?;
        let rows = workspace_invites::table
            .filter(workspace_invites::email.eq(email))
            .filter(
                workspace_invites::status
                    .eq(pg_enum!(InviteStatus::Pending.as_str(), "invite_status")),
            )
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

async fn ensure_user_exists(
    conn: &mut crate::postgres::DbConnection<'_>,
    user_id: Uuid,
) -> Result<(), StorageError> {
    let exists = users::table
        .filter(users::id.eq(user_id))
        .select(users::id)
        .first::<Uuid>(conn)
        .await
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(StorageError::NotFound)
    }
}

async fn ensure_oauth_user_exists(
    conn: &mut crate::postgres::DbConnection<'_>,
    user_id: Uuid,
    email: &str,
) -> Result<(), StorageError> {
    let exists = users::table
        .filter(users::id.eq(user_id))
        .select(users::id)
        .first::<Uuid>(conn)
        .await
        .optional()?
        .is_some();
    if exists {
        return Ok(());
    }

    diesel::insert_into(users::table)
        .values((
            users::id.eq(user_id),
            users::username.eq(email),
            users::password_hash.eq("oauth:external-provider"),
        ))
        .on_conflict(users::id)
        .do_nothing()
        .execute(conn)
        .await
        .map_err(|e| StorageError::Internal(format!("oauth user upsert: {e}")))?;
    Ok(())
}

async fn seed_starter_policies(
    conn: &mut crate::postgres::DbConnection<'_>,
    workspace_id: &str,
    environment_id: &str,
) -> Result<(), StorageError> {
    for source_yaml in STARTER_POLICY_YAMLS {
        let policy = tl_policy::load_str(source_yaml)
            .map_err(|e| StorageError::Internal(format!("starter policy parse: {e}")))?;
        let parsed_policy = serde_json::to_value(&policy)
            .map_err(|e| StorageError::Internal(format!("starter policy serialize: {e}")))?;

        diesel::insert_into(policies::table)
            .values((
                policies::workspace_id.eq(workspace_id),
                policies::id.eq(&policy.id),
                policies::policy_yaml.eq(source_yaml),
                policies::parsed_policy.eq(parsed_policy),
                policies::enabled.eq(false),
                policies::owner_agent_id.eq(None::<String>),
            ))
            .on_conflict((policies::workspace_id, policies::id))
            .do_nothing()
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("starter policy insert: {e}")))?;

        diesel::insert_into(policy_environment_deployments::table)
            .values((
                policy_environment_deployments::workspace_id.eq(workspace_id),
                policy_environment_deployments::environment_id.eq(environment_id),
                policy_environment_deployments::policy_id.eq(&policy.id),
                policy_environment_deployments::enabled.eq(false),
                policy_environment_deployments::deployed_version.eq(None::<i32>),
            ))
            .on_conflict((
                policy_environment_deployments::workspace_id,
                policy_environment_deployments::environment_id,
                policy_environment_deployments::policy_id,
            ))
            .do_nothing()
            .execute(conn)
            .await
            .map_err(|e| {
                StorageError::Internal(format!("starter policy deployment insert: {e}"))
            })?;

        diesel::insert_into(entity_versions::table)
            .values((
                entity_versions::workspace_id.eq(workspace_id),
                entity_versions::entity_type.eq("policy"),
                entity_versions::entity_id.eq(&policy.id),
                entity_versions::version.eq(1),
                entity_versions::content.eq(source_yaml),
            ))
            .on_conflict((
                entity_versions::workspace_id,
                entity_versions::entity_type,
                entity_versions::entity_id,
                entity_versions::version,
            ))
            .do_nothing()
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("starter policy version insert: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn starter_policies_are_valid_yaml() {
        let ids: Vec<_> = super::STARTER_POLICY_YAMLS
            .iter()
            .map(|source| tl_policy::load_str(source).expect("starter policy").id)
            .collect();

        assert_eq!(
            ids,
            vec![
                "starter-pii-email",
                "starter-pii-phone",
                "starter-pii-ssn",
                "starter-pii-credit-card",
                "starter-pii-ipv4",
                "starter-prompt-injection",
            ]
        );
    }
}
