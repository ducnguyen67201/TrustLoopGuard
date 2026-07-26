//! Workspace team + invite wire types.
//!
//! Shape backing `/v1/team/*` workspace membership and invite flows.
//! Storage and HTTP handlers live in `tl-storage` and `tl-server`;
//! this module is the single source of truth for the wire format
//! consumed by the dashboard + SDKs.

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

/// POST `/v1/team/invites` request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateInviteRequest {
    pub email: String,
    pub role: WorkspaceRole,
}

/// POST `/v1/team/invites` outcome. Discriminated by `kind`:
/// - `added` — the email matched an existing user; they're now a
///   workspace member. No accept step needed.
/// - `invited` — no account exists for that email yet; we recorded
///   a pending membership intent. When the user signs up with this
///   email (any time, anywhere), they're auto-joined on their next
///   page load via the `accept_pending_invites_for_email` path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum CreateInviteResponse {
    Added { member: WorkspaceMember },
    Invited { invite: WorkspaceInvite },
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

/// A workspace the signed-in user can access. Normally this comes from
/// membership; platform administrators receive every active workspace.
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
    pub is_knowledge_base_enabled: bool,
    pub is_attacks_enabled: bool,
    pub is_mcp_gateway_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct MyWorkspacesResponse {
    /// Whether the signed-in user has cross-workspace platform access.
    pub is_platform_admin: bool,
    pub workspaces: Vec<MyWorkspace>,
}

/// POST `/v1/team/my-workspaces` body. Creates a fresh organization +
/// workspace pair and grants the calling user `owner` on both.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateWorkspaceRequest {
    pub name: String,
}
