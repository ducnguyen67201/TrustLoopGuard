use std::sync::Arc;

use async_trait::async_trait;
use tl_storage::{
    GitHubCreateConnection, GitHubCreateJob, GitHubIntegrationRepo, GitHubJobTransition,
    GitHubUpsertInstallation, NewGitHubInstallationState,
};

use crate::github_integration::{
    ClaimedGitHubInstallationState, GitHubConnectionCreate, GitHubInstallationUpsert,
    GitHubIntegrationStore, GitHubIntegrationStoreError, GitHubJobCreate, GitHubJobUpdate,
    NewGitHubInstallationState as ServerNewGitHubInstallationState,
};

pub struct PostgresGitHubIntegrationAdapter {
    repo: Arc<GitHubIntegrationRepo>,
}

impl PostgresGitHubIntegrationAdapter {
    pub fn new(repo: Arc<GitHubIntegrationRepo>) -> Arc<Self> {
        Arc::new(Self { repo })
    }
}

#[async_trait]
impl GitHubIntegrationStore for PostgresGitHubIntegrationAdapter {
    async fn create_state(
        &self,
        state: ServerNewGitHubInstallationState,
    ) -> Result<(), GitHubIntegrationStoreError> {
        self.repo
            .create_state(NewGitHubInstallationState {
                state_hash: state.state_hash,
                workspace_id: state.workspace_id,
                user_id: state.user_id,
                expires_at: state.expires_at,
            })
            .await
            .map_err(map_storage)
    }

    async fn claim_state(
        &self,
        state_hash: &[u8],
        user_id: uuid::Uuid,
    ) -> Result<ClaimedGitHubInstallationState, GitHubIntegrationStoreError> {
        self.repo
            .claim_state(state_hash, user_id)
            .await
            .map(|claimed| ClaimedGitHubInstallationState {
                workspace_id: claimed.workspace_id,
                user_id: claimed.user_id,
            })
            .map_err(map_storage)
    }

    async fn upsert_installation(
        &self,
        workspace_id: &str,
        input: GitHubInstallationUpsert,
    ) -> Result<tl_core::GitHubInstallationSummary, GitHubIntegrationStoreError> {
        self.repo
            .upsert_installation(
                workspace_id,
                GitHubUpsertInstallation {
                    installation_id: input.installation_id,
                    account_login: input.account_login,
                    account_type: input.account_type,
                    repository_selection: input.repository_selection,
                    installed_by_user_id: input.installed_by_user_id,
                },
            )
            .await
            .map_err(map_storage)
    }

    async fn active_installation(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::GitHubInstallationSummary, GitHubIntegrationStoreError> {
        self.repo
            .active_installation(workspace_id)
            .await
            .map_err(map_storage)
    }

    async fn installation_for_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<tl_core::GitHubInstallationSummary, GitHubIntegrationStoreError> {
        self.repo
            .installation_for_connection(workspace_id, connection_id)
            .await
            .map_err(map_storage)
    }

    async fn create_connection(
        &self,
        workspace_id: &str,
        input: GitHubConnectionCreate,
    ) -> Result<tl_core::GitHubConnectionSummary, GitHubIntegrationStoreError> {
        self.repo
            .create_connection(
                workspace_id,
                GitHubCreateConnection {
                    installation_id: input.installation_id,
                    repository_id: input.repository_id,
                    owner: input.owner,
                    name: input.name,
                    default_branch: input.default_branch,
                    root_path: input.root_path,
                    agent_id: input.agent_id,
                    environment_id: input.environment_id,
                },
            )
            .await
            .map_err(map_storage)
    }

    async fn list_connections(
        &self,
        workspace_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<tl_core::GitHubConnectionSummary>, GitHubIntegrationStoreError> {
        self.repo
            .list_connections(workspace_id, agent_id)
            .await
            .map_err(map_storage)
    }

    async fn get_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<tl_core::GitHubConnectionSummary, GitHubIntegrationStoreError> {
        self.repo
            .get_connection(workspace_id, connection_id)
            .await
            .map_err(map_storage)
    }

    async fn disconnect_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<(), GitHubIntegrationStoreError> {
        self.repo
            .disconnect_connection(workspace_id, connection_id)
            .await
            .map_err(map_storage)
    }

    async fn mark_installation_removed(
        &self,
        installation_id: i64,
    ) -> Result<(), GitHubIntegrationStoreError> {
        self.repo
            .mark_installation_removed(installation_id)
            .await
            .map_err(map_storage)
    }

    async fn mark_pull_request_closed(
        &self,
        repository_id: i64,
        pull_request_number: i64,
        branch_name: &str,
        merged: bool,
        closed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), GitHubIntegrationStoreError> {
        self.repo
            .mark_pull_request_closed(
                repository_id,
                pull_request_number,
                branch_name,
                merged,
                closed_at,
            )
            .await
            .map_err(map_storage)
    }

    async fn create_job(
        &self,
        workspace_id: &str,
        input: GitHubJobCreate,
    ) -> Result<tl_core::GitHubIntegrationJobSummary, GitHubIntegrationStoreError> {
        self.repo
            .create_job(
                workspace_id,
                GitHubCreateJob {
                    connection_id: input.connection_id,
                    risk_statement: input.risk_statement,
                    base_branch: input.base_branch,
                    base_sha: input.base_sha,
                    installation_connected_at: input.installation_connected_at,
                    repository_connected_at: input.repository_connected_at,
                },
            )
            .await
            .map_err(map_storage)
    }

    async fn get_job(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<tl_core::GitHubIntegrationJobSummary, GitHubIntegrationStoreError> {
        self.repo
            .get_job(workspace_id, job_id)
            .await
            .map_err(map_storage)
    }

    async fn transition_job(
        &self,
        workspace_id: &str,
        job_id: &str,
        expected: &[tl_core::GitHubIntegrationJobStatus],
        next: tl_core::GitHubIntegrationJobStatus,
        fields: GitHubJobUpdate,
    ) -> Result<tl_core::GitHubIntegrationJobSummary, GitHubIntegrationStoreError> {
        self.repo
            .transition_job(
                workspace_id,
                job_id,
                expected,
                next,
                GitHubJobTransition {
                    analysis_summary: fields.analysis_summary,
                    proposed_changes: fields.proposed_changes,
                    manual_steps: fields.manual_steps,
                    base_sha: fields.base_sha,
                    branch_name: fields.branch_name,
                    commit_sha: fields.commit_sha,
                    pull_request_number: fields.pull_request_number,
                    pull_request_url: fields.pull_request_url,
                    error_code: fields.error_code,
                    error_message: fields.error_message,
                    clear_proposed_changes: fields.clear_proposed_changes,
                    analysis_completed_at: fields.analysis_completed_at,
                    pr_opened_at: fields.pr_opened_at,
                    pr_merged_at: fields.pr_merged_at,
                    first_verified_trace_at: fields.first_verified_trace_at,
                },
            )
            .await
            .map_err(map_storage)
    }

    async fn list_recoverable_jobs(
        &self,
    ) -> Result<
        Vec<(String, String, tl_core::GitHubIntegrationJobStatus)>,
        GitHubIntegrationStoreError,
    > {
        self.repo.list_recoverable_jobs().await.map_err(map_storage)
    }
}

fn map_storage(error: tl_storage::StorageError) -> GitHubIntegrationStoreError {
    match error {
        tl_storage::StorageError::NotFound => GitHubIntegrationStoreError::NotFound,
        tl_storage::StorageError::Conflict => GitHubIntegrationStoreError::Conflict,
        tl_storage::StorageError::Internal(message) => {
            GitHubIntegrationStoreError::Internal(message)
        }
    }
}
