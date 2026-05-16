//! Workspace team + invite endpoints.
//!
//! Wire shape lives in `tl_core::team`; durable storage lives in
//! `tl-storage::team_repo`. This module exposes:
//!
//! - `GET    /v1/team/members`         — list workspace members
//! - `GET    /v1/team/invites`         — list pending invites
//! - `POST   /v1/team/invites`         — create an invite (returns token + accept path)
//! - `DELETE /v1/team/invites/:id`     — revoke a pending invite
//! - `GET    /v1/invites/:id/lookup`   — **public** invite metadata (for accept page)
//!
//! The first four are bearer-protected via the existing shared-key
//! middleware. The lookup endpoint is intentionally public so the
//! dashboard's `/invite/accept?token=…` page can render before the
//! visitor has signed in. It only returns the workspace name, the
//! invited email, role, status, and a `user_exists` flag — nothing
//! that isn't already known to the invite recipient.
//!
//! Actually *consuming* an invite happens via
//! `POST /v1/auth/signup` with `invite_token` set, or via the
//! follow-up bind-invite call once Phase B per-user auth lands.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{
    ApiError, ApiErrorCode, CreateInviteRequest, CreateInviteResponse, CreateWorkspaceRequest,
    InviteListResponse, InviteLookupResponse, InviteStatus, MemberListResponse, MyWorkspace,
    MyWorkspacesResponse, WorkspaceInvite, WorkspaceMember, WorkspaceRole,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum TeamStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct InviteLookupRecord {
    pub invite: WorkspaceInvite,
    pub workspace_name: String,
    pub workspace_slug: String,
    pub user_exists: bool,
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

    async fn create_invite(
        &self,
        workspace_id: &str,
        email: &str,
        role: WorkspaceRole,
        invited_by: Option<Uuid>,
    ) -> Result<WorkspaceInvite, TeamStoreError>;

    async fn revoke_invite(
        &self,
        workspace_id: &str,
        invite_id: &str,
    ) -> Result<(), TeamStoreError>;

    async fn lookup_invite(&self, invite_id: &str) -> Result<InviteLookupRecord, TeamStoreError>;

    /// Atomically consume a pending invite for `user_id`. Returns the
    /// workspace_id the user just joined.
    async fn accept_invite(&self, invite_id: &str, user_id: Uuid)
        -> Result<String, TeamStoreError>;

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

/// In-memory implementation. Useful for the no-DB boot path and unit
/// tests. Not durable.
#[derive(Debug, Default)]
pub struct MemoryTeamStore {
    inner: tokio::sync::RwLock<MemoryTeamState>,
}

#[derive(Debug, Default)]
struct MemoryTeamState {
    members: Vec<(String, WorkspaceMember)>,
    invites: Vec<WorkspaceInvite>,
}

impl MemoryTeamStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TeamStore for MemoryTeamStore {
    async fn list_members(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceMember>, TeamStoreError> {
        let guard = self.inner.read().await;
        Ok(guard
            .members
            .iter()
            .filter(|(ws, _)| ws == workspace_id)
            .map(|(_, m)| m.clone())
            .collect())
    }

    async fn list_pending_invites(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceInvite>, TeamStoreError> {
        let guard = self.inner.read().await;
        Ok(guard
            .invites
            .iter()
            .filter(|i| i.workspace_id == workspace_id && i.status == InviteStatus::Pending)
            .cloned()
            .collect())
    }

    async fn create_invite(
        &self,
        workspace_id: &str,
        email: &str,
        role: WorkspaceRole,
        invited_by: Option<Uuid>,
    ) -> Result<WorkspaceInvite, TeamStoreError> {
        let mut guard = self.inner.write().await;
        if guard.invites.iter().any(|i| {
            i.workspace_id == workspace_id && i.email == email && i.status == InviteStatus::Pending
        }) {
            return Err(TeamStoreError::Conflict);
        }
        let id = generate_memory_token();
        let now = chrono::Utc::now();
        let invite = WorkspaceInvite {
            id,
            workspace_id: workspace_id.to_string(),
            email: email.to_string(),
            role,
            status: InviteStatus::Pending,
            invited_by_user_id: invited_by.map(|u| u.to_string()),
            created_at: now.to_rfc3339(),
            expires_at: (now + chrono::Duration::days(7)).to_rfc3339(),
        };
        guard.invites.push(invite.clone());
        Ok(invite)
    }

    async fn revoke_invite(
        &self,
        workspace_id: &str,
        invite_id: &str,
    ) -> Result<(), TeamStoreError> {
        let mut guard = self.inner.write().await;
        let pos = guard
            .invites
            .iter()
            .position(|i| {
                i.id == invite_id
                    && i.workspace_id == workspace_id
                    && i.status == InviteStatus::Pending
            })
            .ok_or(TeamStoreError::NotFound)?;
        guard.invites[pos].status = InviteStatus::Revoked;
        Ok(())
    }

    async fn lookup_invite(&self, invite_id: &str) -> Result<InviteLookupRecord, TeamStoreError> {
        let guard = self.inner.read().await;
        let invite = guard
            .invites
            .iter()
            .find(|i| i.id == invite_id)
            .cloned()
            .ok_or(TeamStoreError::NotFound)?;
        Ok(InviteLookupRecord {
            invite,
            workspace_name: "Workspace".to_string(),
            workspace_slug: "workspace".to_string(),
            user_exists: false,
        })
    }

    async fn accept_invite(
        &self,
        invite_id: &str,
        user_id: Uuid,
    ) -> Result<String, TeamStoreError> {
        let mut guard = self.inner.write().await;
        let pos = guard
            .invites
            .iter()
            .position(|i| i.id == invite_id)
            .ok_or(TeamStoreError::NotFound)?;
        if guard.invites[pos].status != InviteStatus::Pending {
            return Err(TeamStoreError::Conflict);
        }
        let workspace_id = guard.invites[pos].workspace_id.clone();
        let role = guard.invites[pos].role;
        let email = guard.invites[pos].email.clone();
        guard.invites[pos].status = InviteStatus::Accepted;
        guard.members.push((
            workspace_id.clone(),
            WorkspaceMember {
                user_id: user_id.to_string(),
                username: email,
                role,
                joined_at: chrono::Utc::now().to_rfc3339(),
            },
        ));
        Ok(workspace_id)
    }

    async fn accept_pending_invites_for_email(
        &self,
        email: &str,
        user_id: Uuid,
    ) -> Result<usize, TeamStoreError> {
        let ids: Vec<String> = {
            let guard = self.inner.read().await;
            guard
                .invites
                .iter()
                .filter(|i| i.email == email && i.status == InviteStatus::Pending)
                .map(|i| i.id.clone())
                .collect()
        };
        let mut accepted = 0usize;
        for id in ids {
            if self.accept_invite(&id, user_id).await.is_ok() {
                accepted += 1;
            }
        }
        Ok(accepted)
    }

    async fn list_workspaces_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MyWorkspace>, TeamStoreError> {
        let guard = self.inner.read().await;
        let user_str = user_id.to_string();
        Ok(guard
            .members
            .iter()
            .filter(|(_, m)| m.user_id == user_str)
            .map(|(ws_id, m)| MyWorkspace {
                id: ws_id.clone(),
                slug: ws_id.clone(),
                name: ws_id.clone(),
                organization_id: format!("org_{}", ws_id),
                role: m.role,
            })
            .collect())
    }

    async fn create_workspace(
        &self,
        user_id: Uuid,
        name: &str,
    ) -> Result<MyWorkspace, TeamStoreError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TeamStoreError::Internal(
                "workspace name is required".into(),
            ));
        }
        let slug = trimmed.to_ascii_lowercase().replace(' ', "-");
        let id = format!("ws_{}", slug.replace('-', "_"));
        let mut guard = self.inner.write().await;
        guard.members.push((
            id.clone(),
            WorkspaceMember {
                user_id: user_id.to_string(),
                username: trimmed.to_string(),
                role: WorkspaceRole::Owner,
                joined_at: chrono::Utc::now().to_rfc3339(),
            },
        ));
        Ok(MyWorkspace {
            id: id.clone(),
            slug,
            name: trimmed.to_string(),
            organization_id: format!("org_{}", id),
            role: WorkspaceRole::Owner,
        })
    }
}

fn generate_memory_token() -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Clone)]
pub struct TeamState {
    pub store: Arc<dyn TeamStore>,
}

const X_USER_HEADER: &str = "x-tlg-user-id";
const X_USER_EMAIL_HEADER: &str = "x-tlg-user-email";

/// GET /v1/team/members
pub async fn list_members(State(state): State<TeamState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list_members(&workspace_id).await {
        Ok(members) => Json(MemberListResponse { members }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// GET /v1/team/invites
pub async fn list_invites(State(state): State<TeamState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list_pending_invites(&workspace_id).await {
        Ok(invites) => Json(InviteListResponse { invites }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// POST /v1/team/invites
pub async fn create_invite(
    State(state): State<TeamState>,
    headers: HeaderMap,
    Json(req): Json<CreateInviteRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let email = req.email.trim();
    if email.is_empty() || !email.contains('@') {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "email is required".into(),
        );
    }
    let invited_by = headers
        .get(X_USER_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s.trim()).ok());

    match state
        .store
        .create_invite(&workspace_id, email, req.role, invited_by)
        .await
    {
        Ok(invite) => {
            let accept_path = format!("/invite/accept?token={}", invite.id);
            (
                StatusCode::CREATED,
                Json(CreateInviteResponse {
                    invite,
                    accept_path,
                }),
            )
                .into_response()
        }
        Err(TeamStoreError::Conflict) => api_error(
            StatusCode::CONFLICT,
            ApiErrorCode::Unprocessable,
            "a pending invite already exists for this email".into(),
        ),
        Err(e) => internal_error(e),
    }
}

/// DELETE /v1/team/invites/:id
pub async fn revoke_invite(
    State(state): State<TeamState>,
    headers: HeaderMap,
    Path(invite_id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.revoke_invite(&workspace_id, &invite_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(TeamStoreError::NotFound) => api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "invite not found".into(),
        ),
        Err(e) => internal_error(e),
    }
}

/// GET /v1/team/my-workspaces — list workspaces for the signed-in user.
///
/// Reads `X-TLG-User-Id` (required, UUID) and `X-TLG-User-Email`
/// (optional). When the email is present we first bulk-accept any
/// pending invites addressed to it; the membership query then sees
/// the new rows in the same response. This is the dashboard's
/// "auto-bind on next request" mechanism.
pub async fn list_my_workspaces(State(state): State<TeamState>, headers: HeaderMap) -> Response {
    let user_id = match headers
        .get(X_USER_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
    {
        Some(id) => id,
        None => {
            return api_error(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "X-TLG-User-Id header is required and must be a UUID".into(),
            )
        }
    };

    if let Some(email) = headers
        .get(X_USER_EMAIL_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Err(e) = state
            .store
            .accept_pending_invites_for_email(email, user_id)
            .await
        {
            tracing::warn!(
                user_id = %user_id,
                error = %e,
                "auto-bind pending invites failed; continuing with existing memberships"
            );
        }
    }

    match state.store.list_workspaces_for_user(user_id).await {
        Ok(workspaces) => Json(MyWorkspacesResponse { workspaces }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// POST /v1/team/my-workspaces — create a new workspace owned by
/// the caller. Bootstraps a fresh organization too, so a user who
/// signed up without an invite can self-serve.
pub async fn create_my_workspace(
    State(state): State<TeamState>,
    headers: HeaderMap,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Response {
    let user_id = match headers
        .get(X_USER_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
    {
        Some(id) => id,
        None => {
            return api_error(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "X-TLG-User-Id header is required and must be a UUID".into(),
            )
        }
    };
    let name = req.name.trim();
    if name.is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "workspace name is required".into(),
        );
    }
    match state.store.create_workspace(user_id, name).await {
        Ok(ws) => (StatusCode::CREATED, Json(ws)).into_response(),
        Err(e) => internal_error(e),
    }
}

/// GET /v1/invites/:id/lookup — public.
pub async fn lookup_invite(
    State(state): State<TeamState>,
    Path(invite_id): Path<String>,
) -> Response {
    match state.store.lookup_invite(&invite_id).await {
        Ok(record) => Json(InviteLookupResponse {
            email: record.invite.email,
            role: record.invite.role,
            workspace_name: record.workspace_name,
            workspace_slug: record.workspace_slug,
            status: record.invite.status,
            expires_at: record.invite.expires_at,
            user_exists: record.user_exists,
        })
        .into_response(),
        Err(TeamStoreError::NotFound) => api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "invite not found".into(),
        ),
        Err(e) => internal_error(e),
    }
}

fn internal_error(e: TeamStoreError) -> Response {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        e.to_string(),
    )
}

fn api_error(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}

#[cfg(feature = "postgres")]
mod postgres_adapter {
    use super::*;
    use tl_storage::{StorageError, TeamRepo};

    pub struct TeamRepoAdapter {
        repo: TeamRepo,
    }

    impl TeamRepoAdapter {
        pub fn new(repo: TeamRepo) -> Self {
            Self { repo }
        }
    }

    fn map_err(e: StorageError) -> TeamStoreError {
        match e {
            StorageError::NotFound => TeamStoreError::NotFound,
            StorageError::Conflict => TeamStoreError::Conflict,
            StorageError::Internal(msg) => TeamStoreError::Internal(msg),
        }
    }

    #[async_trait]
    impl TeamStore for TeamRepoAdapter {
        async fn list_members(
            &self,
            workspace_id: &str,
        ) -> Result<Vec<WorkspaceMember>, TeamStoreError> {
            self.repo.list_members(workspace_id).await.map_err(map_err)
        }

        async fn list_pending_invites(
            &self,
            workspace_id: &str,
        ) -> Result<Vec<WorkspaceInvite>, TeamStoreError> {
            self.repo
                .list_pending_invites(workspace_id)
                .await
                .map_err(map_err)
        }

        async fn create_invite(
            &self,
            workspace_id: &str,
            email: &str,
            role: WorkspaceRole,
            invited_by: Option<Uuid>,
        ) -> Result<WorkspaceInvite, TeamStoreError> {
            self.repo
                .create_invite(workspace_id, email, role, invited_by)
                .await
                .map_err(map_err)
        }

        async fn revoke_invite(
            &self,
            workspace_id: &str,
            invite_id: &str,
        ) -> Result<(), TeamStoreError> {
            self.repo
                .revoke_invite(workspace_id, invite_id)
                .await
                .map_err(map_err)
        }

        async fn lookup_invite(
            &self,
            invite_id: &str,
        ) -> Result<InviteLookupRecord, TeamStoreError> {
            let lookup = self.repo.lookup_invite(invite_id).await.map_err(map_err)?;
            Ok(InviteLookupRecord {
                invite: lookup.invite,
                workspace_name: lookup.workspace_name,
                workspace_slug: lookup.workspace_slug,
                user_exists: lookup.user_exists,
            })
        }

        async fn accept_invite(
            &self,
            invite_id: &str,
            user_id: Uuid,
        ) -> Result<String, TeamStoreError> {
            self.repo
                .accept_invite(invite_id, user_id)
                .await
                .map_err(map_err)
        }

        async fn accept_pending_invites_for_email(
            &self,
            email: &str,
            user_id: Uuid,
        ) -> Result<usize, TeamStoreError> {
            self.repo
                .accept_pending_invites_for_email(email, user_id)
                .await
                .map_err(map_err)
        }

        async fn list_workspaces_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<MyWorkspace>, TeamStoreError> {
            self.repo
                .list_workspaces_for_user(user_id)
                .await
                .map_err(map_err)
        }

        async fn create_workspace(
            &self,
            user_id: Uuid,
            name: &str,
        ) -> Result<MyWorkspace, TeamStoreError> {
            self.repo
                .create_workspace(user_id, name)
                .await
                .map_err(map_err)
        }
    }
}

#[cfg(feature = "postgres")]
pub use postgres_adapter::TeamRepoAdapter;
