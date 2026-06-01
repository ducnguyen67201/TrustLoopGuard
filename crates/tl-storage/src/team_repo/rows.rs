use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::{users, workspace_invites, workspace_members};

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct MemberRow {
    pub(super) user_id: Uuid,
    pub(super) role: String,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct UserNameRow {
    pub(super) id: Uuid,
    pub(super) username: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_invites)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct InviteRow {
    pub(super) id: String,
    pub(super) workspace_id: String,
    pub(super) email: String,
    pub(super) role: String,
    pub(super) status: String,
    pub(super) invited_by_user_id: Option<Uuid>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) expires_at: DateTime<Utc>,
}
