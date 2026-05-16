//! Workspace team + invite wire types.
//!
//! Shape backing `/v1/workspaces/:slug/{members,invites}` and the
//! signup-with-invite flow. Storage and HTTP handlers live in
//! `tl-storage` and `tl-server`; this module is the single source of
//! truth for the wire format consumed by the dashboard + SDKs.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Permission level a user holds inside a workspace. Mirrors the
/// `workspace_role` Postgres enum (migration 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Editor,
    Viewer,
}

impl WorkspaceRole {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkspaceRole::Owner => "owner",
            WorkspaceRole::Admin => "admin",
            WorkspaceRole::Editor => "editor",
            WorkspaceRole::Viewer => "viewer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(WorkspaceRole::Owner),
            "admin" => Some(WorkspaceRole::Admin),
            "editor" => Some(WorkspaceRole::Editor),
            "viewer" => Some(WorkspaceRole::Viewer),
            _ => None,
        }
    }
}

/// Lifecycle state of an invite row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum InviteStatus {
    Pending,
    Accepted,
    Revoked,
    Expired,
}

impl InviteStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InviteStatus::Pending => "pending",
            InviteStatus::Accepted => "accepted",
            InviteStatus::Revoked => "revoked",
            InviteStatus::Expired => "expired",
        }
    }
}

/// A user who currently has access to a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct WorkspaceMember {
    pub user_id: String,
    pub username: String,
    pub role: WorkspaceRole,
    /// RFC3339 timestamp.
    pub joined_at: String,
}

/// A pending or historical invite. The `id` doubles as the bearer
/// token: it's an opaque URL-safe random string, single-use, and
/// invalidated on accept/revoke/expire.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct WorkspaceInvite {
    pub id: String,
    pub workspace_id: String,
    pub email: String,
    pub role: WorkspaceRole,
    pub status: InviteStatus,
    pub invited_by_user_id: Option<String>,
    /// RFC3339 timestamps.
    pub created_at: String,
    pub expires_at: String,
}

/// POST `/v1/workspaces/:slug/invites` request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateInviteRequest {
    pub email: String,
    pub role: WorkspaceRole,
}

/// POST `/v1/workspaces/:slug/invites` response body. `accept_path` is
/// the dashboard URL path the caller should share with the invitee
/// (the dashboard fills in the absolute origin).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateInviteResponse {
    pub invite: WorkspaceInvite,
    pub accept_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct MemberListResponse {
    pub members: Vec<WorkspaceMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct InviteListResponse {
    pub invites: Vec<WorkspaceInvite>,
}

/// Read-only metadata about an invite, used by the unauthenticated
/// `/v1/invites/:id/lookup` endpoint so the accept page can show the
/// workspace name + invited email without exposing other workspaces.
/// A workspace the signed-in user belongs to. Drives the dashboard's
/// workspace switcher and the "no workspace yet" redirect.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct MyWorkspace {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub role: WorkspaceRole,
    pub organization_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct MyWorkspacesResponse {
    pub workspaces: Vec<MyWorkspace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct InviteLookupResponse {
    pub email: String,
    pub role: WorkspaceRole,
    pub workspace_name: String,
    pub workspace_slug: String,
    pub status: InviteStatus,
    pub expires_at: String,
    /// True when an account already exists for `email`. The dashboard
    /// uses this to redirect the user to sign in instead of signing up.
    pub user_exists: bool,
}
