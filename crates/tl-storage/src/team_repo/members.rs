use std::collections::HashMap;

use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{WorkspaceMember, WorkspaceRole};
use uuid::Uuid;

use super::{MemberRow, TeamRepo, UserNameRow};
use crate::{
    postgres::DbConnection,
    schema::{organization_members, users, workspace_members},
    StorageError,
};

impl TeamRepo {
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
            .map_err(|error| StorageError::Internal(format!("list members: {error}")))?;
        if member_rows.is_empty() {
            return Ok(vec![]);
        }

        let usernames = load_usernames(&mut conn, &member_rows).await?;
        Ok(member_rows
            .into_iter()
            .map(|row| member_row_to_wire(row, &usernames))
            .collect())
    }
}

pub(super) async fn insert_existing_workspace_member(
    conn: &mut DbConnection<'_>,
    organization_id: String,
    workspace_id: String,
    user_id: Uuid,
    username: String,
    role: WorkspaceRole,
) -> Result<WorkspaceMember, StorageError> {
    conn.transaction::<WorkspaceMember, StorageError, _>(async |conn| {
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
                workspace_members::workspace_id.eq(&workspace_id),
                workspace_members::user_id.eq(user_id),
                workspace_members::role.eq(pg_enum!(role.as_str(), "workspace_role")),
            ))
            .on_conflict((workspace_members::workspace_id, workspace_members::user_id))
            .do_update()
            .set(workspace_members::role.eq(pg_enum!(role.as_str(), "workspace_role")))
            .execute(conn)
            .await?;

        Ok(WorkspaceMember {
            user_id: user_id.to_string(),
            username,
            role,
            joined_at: Utc::now().to_rfc3339(),
        })
    })
    .await
}

async fn load_usernames(
    conn: &mut DbConnection<'_>,
    member_rows: &[MemberRow],
) -> Result<HashMap<Uuid, String>, StorageError> {
    let ids: Vec<Uuid> = member_rows.iter().map(|row| row.user_id).collect();
    let users_rows: Vec<UserNameRow> = users::table
        .filter(users::id.eq_any(&ids))
        .select(UserNameRow::as_select())
        .load::<UserNameRow>(conn)
        .await
        .map_err(|error| StorageError::Internal(format!("list members.users: {error}")))?;

    Ok(users_rows
        .into_iter()
        .map(|user| (user.id, user.username))
        .collect())
}

fn member_row_to_wire(row: MemberRow, usernames: &HashMap<Uuid, String>) -> WorkspaceMember {
    WorkspaceMember {
        user_id: row.user_id.to_string(),
        username: usernames
            .get(&row.user_id)
            .cloned()
            .unwrap_or_else(|| row.user_id.to_string()),
        role: WorkspaceRole::parse(&row.role).unwrap_or(WorkspaceRole::Viewer),
        joined_at: row.created_at.to_rfc3339(),
    }
}
