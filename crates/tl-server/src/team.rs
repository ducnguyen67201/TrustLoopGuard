//! Workspace team + invite endpoints.
//!
//! Wire shape lives in `tl_core::team`; durable storage lives in
//! `tl-storage::team_repo`. This module exposes:
//!
//! - `GET    /v1/team/members`         — list workspace members
//! - `GET    /v1/team/invites`         — list pending invites
//! - `POST   /v1/team/invites`         — add an existing user or create a pending invite
//! - `DELETE /v1/team/invites/:id`     — revoke a pending invite
//!
//! These routes are bearer-protected via the existing shared-key
//! middleware. Pending invites are consumed by `GET /v1/team/my-workspaces`
//! with `X-TLG-User-Email` set: any pending invite for that email is
//! bulk-accepted before the membership list is returned.

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{MyWorkspace, WorkspaceInvite, WorkspaceMember, WorkspaceRole};
use uuid::Uuid;

mod handlers;
mod memory_store;
#[cfg(feature = "postgres")]
mod postgres_adapter;
mod request_context;
mod response;

pub use handlers::{
    __path_create_my_workspace, __path_list_my_workspaces, create_invite, create_my_workspace,
    list_invites, list_members, list_my_workspaces, revoke_invite,
};
pub use memory_store::MemoryTeamStore;
#[cfg(feature = "postgres")]
pub use postgres_adapter::TeamRepoAdapter;

#[derive(Debug, thiserror::Error)]
pub enum TeamStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("internal: {0}")]
    Internal(String),
}

/// Outcome of the smart admin-add path. `Added` when the email
/// matched an existing user (they're now a member); `Invited` when
/// the email has no account yet (pending membership intent that
/// auto-binds on signup).
#[derive(Debug, Clone)]
pub enum AddMemberOutcome {
    Added(WorkspaceMember),
    Invited(WorkspaceInvite),
}

#[async_trait]
pub trait TeamStore: Send + Sync {
    async fn list_members(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceMember>, TeamStoreError>;

    async fn list_pending_invites(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceInvite>, TeamStoreError>;

    /// Admin-driven add. Smart path: existing user → workspace_member
    /// immediately; otherwise pending invite (auto-binds on signup).
    async fn add_member_or_invite(
        &self,
        workspace_id: &str,
        email: &str,
        role: WorkspaceRole,
        invited_by: Option<Uuid>,
    ) -> Result<AddMemberOutcome, TeamStoreError>;

    async fn revoke_invite(
        &self,
        workspace_id: &str,
        invite_id: &str,
    ) -> Result<(), TeamStoreError>;

    /// Bulk-accept every pending invite addressed to `email`. Used as a
    /// prelude to membership lookups so a user who's invited *after*
    /// signing up auto-binds on their next session refresh.
    async fn accept_pending_invites_for_email(
        &self,
        email: &str,
        user_id: Uuid,
    ) -> Result<usize, TeamStoreError>;

    /// Workspaces the signed-in user belongs to. Drives the
    /// dashboard's "no workspace yet" enforcement.
    async fn list_workspaces_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MyWorkspace>, TeamStoreError>;

    /// Create a fresh org+workspace pair owned by `user_id`. Used by
    /// the `/welcome` page so a self-serve signup can bootstrap
    /// without an admin invite.
    async fn create_workspace(
        &self,
        user_id: Uuid,
        name: &str,
    ) -> Result<MyWorkspace, TeamStoreError>;
}

#[derive(Clone)]
pub struct TeamState {
    pub store: Arc<dyn TeamStore>,
    pub workspace_self_service_enabled: bool,
}
