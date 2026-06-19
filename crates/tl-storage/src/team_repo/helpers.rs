use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand::Rng;
use tl_core::{InviteStatus, WorkspaceInvite, WorkspaceRole};
use uuid::Uuid;

use super::InviteRow;
use crate::schema::{users, workspaces};
use crate::StorageError;

pub(super) fn invite_row_to_wire(row: InviteRow) -> WorkspaceInvite {
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

pub(super) fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Best-effort slug: lowercase, ASCII alphanumerics + hyphens, no
/// leading/trailing hyphens, capped at 48 chars. Empty input or
/// purely non-ASCII input falls back to a short random slug.
pub(super) fn slugify(name: &str) -> String {
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
        rand::rng().fill_bytes(&mut bytes);
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
pub(super) async fn unique_workspace_slug(
    conn: &mut crate::postgres::DbConnection<'_>,
    base: &str,
) -> Result<String, StorageError> {
    for attempt in 0..8 {
        let candidate = if attempt == 0 {
            base.to_string()
        } else {
            let mut bytes = [0u8; 3];
            rand::rng().fill_bytes(&mut bytes);
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

pub(super) async fn ensure_user_exists(
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

pub(super) async fn ensure_oauth_user_exists(
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
