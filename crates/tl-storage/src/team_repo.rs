//! Workspace team + invite repository.
//!
//! Backs the `/v1/team/*` endpoints. Schema lives in migration 6
//! (`workspace_members`, `workspace_invites`, plus the
//! `workspace_role` and `invite_status` enums).
//!
//! Invite ids are opaque 32-byte URL-safe random strings generated here.
//! Pending invites are consumed by matching the invite email to the signed-in
//! user during workspace lookup, then invalidated on accept / revoke / expire.

macro_rules! pg_enum {
    ($value:expr, $pg_type:literal) => {
        diesel::dsl::sql::<diesel::sql_types::Text>("")
            .bind::<diesel::sql_types::Text, _>($value)
            .sql(concat!("::", $pg_type))
    };
}

mod helpers;
mod invites;
mod members;
mod rows;
mod starter_policies;
mod workspaces;

use tl_core::{WorkspaceInvite, WorkspaceMember};

use crate::postgres::{DbConnection, DbPool};
use crate::StorageError;
use rows::{InviteRow, MemberRow, UserNameRow};
use starter_policies::seed_starter_policies;

/// How long a freshly-minted invite stays valid.
pub const INVITE_TTL_DAYS: i64 = 7;

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

impl TeamRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}
