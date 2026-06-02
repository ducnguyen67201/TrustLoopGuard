use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{InviteStatus, WorkspaceInvite, WorkspaceRole};
use uuid::Uuid;

use super::{
    helpers::{ensure_oauth_user_exists, ensure_user_exists, generate_token, invite_row_to_wire},
    members::insert_existing_workspace_member,
    AddMemberOutcome, InviteRow, TeamRepo, INVITE_TTL_DAYS,
};
use crate::{
    schema::{organization_members, users, workspace_invites, workspace_members, workspaces},
    StorageError,
};

impl TeamRepo {
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
            .map_err(|error| StorageError::Internal(format!("list invites: {error}")))?;
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
            .map_err(|error| StorageError::Internal(format!("invite lookup: {error}")))?;
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
            .map_err(|error| StorageError::Internal(format!("insert invite: {error}")))?;
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
            .map_err(|error| StorageError::Internal(format!("user lookup: {error}")))?;

        if let Some((user_id, username)) = existing_user {
            let organization_id: String = workspaces::table
                .filter(workspaces::id.eq(workspace_id))
                .select(workspaces::organization_id)
                .first::<String>(&mut conn)
                .await
                .map_err(|error| {
                    StorageError::Internal(format!("workspace org lookup: {error}"))
                })?;

            let member = insert_existing_workspace_member(
                &mut conn,
                organization_id,
                workspace_id.to_string(),
                user_id,
                username,
                role,
            )
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
        .map_err(|error| StorageError::Internal(format!("revoke invite: {error}")))?;
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
            .map_err(|error| StorageError::Internal(format!("auto-bind list: {error}")))?;
        let mut accepted = 0usize;
        for id in rows {
            match self.accept_invite(&id, user_id).await {
                Ok(_) => accepted += 1,
                // A race condition (concurrent accept/revoke) is benign
                // here — we're best-effort. Don't surface as an error.
                Err(StorageError::Conflict) | Err(StorageError::NotFound) => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(accepted)
    }
}
