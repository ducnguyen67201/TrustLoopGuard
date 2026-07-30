use async_trait::async_trait;
use tl_core::{MyWorkspace, WorkspaceInvite, WorkspaceMember, WorkspaceRole};
use tl_storage::{StorageError, TeamRepo, WorkspaceDeletionOutcome};
use uuid::Uuid;

use super::{AddMemberOutcome, TeamStore, TeamStoreError};

pub struct TeamRepoAdapter {
    repo: TeamRepo,
}

impl TeamRepoAdapter {
    pub fn new(repo: TeamRepo) -> Self {
        Self { repo }
    }
}

fn map_err(error: StorageError) -> TeamStoreError {
    match error {
        StorageError::NotFound => TeamStoreError::NotFound,
        StorageError::Conflict => TeamStoreError::Conflict,
        StorageError::Internal(message) => TeamStoreError::Internal(message),
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

    async fn add_member_or_invite(
        &self,
        workspace_id: &str,
        email: &str,
        role: WorkspaceRole,
        invited_by: Option<Uuid>,
    ) -> Result<AddMemberOutcome, TeamStoreError> {
        let outcome = self
            .repo
            .add_member_or_invite(workspace_id, email, role, invited_by)
            .await
            .map_err(map_err)?;
        Ok(match outcome {
            tl_storage::AddMemberOutcome::Added(member) => AddMemberOutcome::Added(member),
            tl_storage::AddMemberOutcome::Invited(invite) => AddMemberOutcome::Invited(invite),
        })
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

    async fn is_platform_admin(&self, user_id: Uuid) -> Result<bool, TeamStoreError> {
        self.repo.is_platform_admin(user_id).await.map_err(map_err)
    }

    async fn list_all_workspaces(&self) -> Result<Vec<MyWorkspace>, TeamStoreError> {
        self.repo.list_all_workspaces().await.map_err(map_err)
    }

    async fn get_workspace(&self, workspace_id: &str) -> Result<MyWorkspace, TeamStoreError> {
        self.repo.get_workspace(workspace_id).await.map_err(map_err)
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

    async fn delete_workspace(
        &self,
        user_id: Uuid,
        workspace_id: &str,
    ) -> Result<(), TeamStoreError> {
        match self
            .repo
            .delete_workspace(user_id, workspace_id)
            .await
            .map_err(map_err)?
        {
            WorkspaceDeletionOutcome::Deleted => Ok(()),
            WorkspaceDeletionOutcome::Forbidden => Err(TeamStoreError::Forbidden),
        }
    }
}
