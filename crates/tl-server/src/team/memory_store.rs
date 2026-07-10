use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::Rng;
use tl_core::{InviteStatus, MyWorkspace, WorkspaceInvite, WorkspaceMember, WorkspaceRole};
use uuid::Uuid;

use super::{AddMemberOutcome, TeamStore, TeamStoreError};

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

    /// Atomically consume a pending invite for `user_id`. Returns the
    /// workspace_id the user just joined.
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

    async fn add_member_or_invite(
        &self,
        workspace_id: &str,
        email: &str,
        role: WorkspaceRole,
        invited_by: Option<Uuid>,
    ) -> Result<AddMemberOutcome, TeamStoreError> {
        // Memory mode has no `users` table to consult, so we always
        // record a pending invite. The auto-bind path on signup picks
        // it up when the user eventually exists.
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
        Ok(AddMemberOutcome::Invited(invite))
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
                is_knowledge_base_enabled: false,
                is_attacks_enabled: false,
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
            is_knowledge_base_enabled: false,
            is_attacks_enabled: false,
        })
    }
}

fn generate_memory_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
