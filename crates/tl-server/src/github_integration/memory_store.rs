use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{
    GitHubConnectionStatus, GitHubConnectionSummary, GitHubInstallationStatus,
    GitHubInstallationSummary, GitHubIntegrationJobStatus, GitHubIntegrationJobSummary,
    GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1,
};
use uuid::Uuid;

use super::{
    ClaimedGitHubInstallationState, GitHubConnectionCreate, GitHubInstallationUpsert,
    GitHubIntegrationStore, GitHubIntegrationStoreError, GitHubJobCreate, GitHubJobUpdate,
    NewGitHubInstallationState,
};

#[derive(Default)]
pub struct MemoryGitHubIntegrationStore {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    states: HashMap<Vec<u8>, StoredState>,
    installations: HashMap<(String, i64), GitHubInstallationSummary>,
    connections: HashMap<(String, String), GitHubConnectionSummary>,
    jobs: HashMap<(String, String), GitHubIntegrationJobSummary>,
}

#[derive(Clone)]
struct StoredState {
    workspace_id: String,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
    consumed: bool,
}

impl MemoryGitHubIntegrationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl GitHubIntegrationStore for MemoryGitHubIntegrationStore {
    async fn create_state(
        &self,
        state: NewGitHubInstallationState,
    ) -> Result<(), GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        inner.states.insert(
            state.state_hash,
            StoredState {
                workspace_id: state.workspace_id,
                user_id: state.user_id,
                expires_at: state.expires_at,
                consumed: false,
            },
        );
        Ok(())
    }

    async fn claim_state(
        &self,
        state_hash: &[u8],
        user_id: Uuid,
    ) -> Result<ClaimedGitHubInstallationState, GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        let Some(state) = inner.states.get_mut(state_hash) else {
            return Err(GitHubIntegrationStoreError::NotFound);
        };
        if state.user_id != user_id || state.consumed || state.expires_at <= Utc::now() {
            return Err(GitHubIntegrationStoreError::NotFound);
        }
        state.consumed = true;
        Ok(ClaimedGitHubInstallationState {
            workspace_id: state.workspace_id.clone(),
            user_id: state.user_id,
        })
    }

    async fn upsert_installation(
        &self,
        workspace_id: &str,
        input: GitHubInstallationUpsert,
    ) -> Result<GitHubInstallationSummary, GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        let now = Utc::now().to_rfc3339();
        let key = (workspace_id.to_string(), input.installation_id);
        let summary = GitHubInstallationSummary {
            id: inner
                .installations
                .get(&key)
                .map(|existing| existing.id.clone())
                .unwrap_or_else(|| Uuid::now_v7().to_string()),
            workspace_id: workspace_id.to_string(),
            installation_id: input.installation_id.to_string(),
            account_login: input.account_login,
            account_type: input.account_type,
            repository_selection: input.repository_selection,
            status: GitHubInstallationStatus::Active,
            created_at: inner
                .installations
                .get(&key)
                .map(|existing| existing.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
        };
        inner.installations.insert(key, summary.clone());
        Ok(summary)
    }

    async fn active_installation(
        &self,
        workspace_id: &str,
    ) -> Result<GitHubInstallationSummary, GitHubIntegrationStoreError> {
        let inner = self.inner.lock().expect("github memory lock");
        inner
            .installations
            .values()
            .find(|row| {
                row.workspace_id == workspace_id && row.status == GitHubInstallationStatus::Active
            })
            .cloned()
            .ok_or(GitHubIntegrationStoreError::NotFound)
    }

    async fn installation_for_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<GitHubInstallationSummary, GitHubIntegrationStoreError> {
        let inner = self.inner.lock().expect("github memory lock");
        let connection = inner
            .connections
            .get(&(workspace_id.to_string(), connection_id.to_string()))
            .ok_or(GitHubIntegrationStoreError::NotFound)?;
        inner
            .installations
            .values()
            .find(|row| {
                row.workspace_id == workspace_id
                    && row.id == connection.installation_id
                    && row.status == GitHubInstallationStatus::Active
            })
            .cloned()
            .ok_or(GitHubIntegrationStoreError::NotFound)
    }

    async fn create_connection(
        &self,
        workspace_id: &str,
        input: GitHubConnectionCreate,
    ) -> Result<GitHubConnectionSummary, GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        let now = Utc::now().to_rfc3339();
        let id = Uuid::now_v7().to_string();
        if inner.connections.values().any(|row| {
            row.workspace_id == workspace_id
                && row.repository_id == input.repository_id.to_string()
                && row.root_path == input.root_path
                && row.agent_id == input.agent_id
                && row.environment_id == input.environment_id
                && row.status != GitHubConnectionStatus::Disconnected
        }) {
            return Err(GitHubIntegrationStoreError::Conflict);
        }
        let summary = GitHubConnectionSummary {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            installation_id: input.installation_id.to_string(),
            repository_id: input.repository_id.to_string(),
            owner: input.owner,
            name: input.name,
            default_branch: input.default_branch,
            root_path: input.root_path,
            agent_id: input.agent_id,
            environment_id: input.environment_id,
            status: GitHubConnectionStatus::Active,
            recipe_version: GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1.to_string(),
            created_at: now.clone(),
            updated_at: now,
        };
        inner
            .connections
            .insert((workspace_id.to_string(), id), summary.clone());
        Ok(summary)
    }

    async fn list_connections(
        &self,
        workspace_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<GitHubConnectionSummary>, GitHubIntegrationStoreError> {
        let inner = self.inner.lock().expect("github memory lock");
        let mut rows = inner
            .connections
            .values()
            .filter(|row| {
                row.workspace_id == workspace_id
                    && row.status != GitHubConnectionStatus::Disconnected
                    && agent_id.map_or(true, |agent_id| row.agent_id == agent_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(rows)
    }

    async fn get_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<GitHubConnectionSummary, GitHubIntegrationStoreError> {
        let inner = self.inner.lock().expect("github memory lock");
        inner
            .connections
            .get(&(workspace_id.to_string(), connection_id.to_string()))
            .cloned()
            .ok_or(GitHubIntegrationStoreError::NotFound)
    }

    async fn disconnect_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<(), GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        let Some(row) = inner
            .connections
            .get_mut(&(workspace_id.to_string(), connection_id.to_string()))
        else {
            return Err(GitHubIntegrationStoreError::NotFound);
        };
        row.status = GitHubConnectionStatus::Disconnected;
        row.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    async fn mark_installation_removed(
        &self,
        installation_id: i64,
    ) -> Result<(), GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        let mut removed_installation_ids = Vec::new();
        for installation in inner.installations.values_mut() {
            if installation.installation_id == installation_id.to_string() {
                installation.status = GitHubInstallationStatus::Removed;
                installation.updated_at = Utc::now().to_rfc3339();
                removed_installation_ids.push(installation.id.clone());
            }
        }
        for connection in inner.connections.values_mut() {
            if removed_installation_ids.contains(&connection.installation_id) {
                connection.status = GitHubConnectionStatus::AccessRemoved;
                connection.updated_at = Utc::now().to_rfc3339();
            }
        }
        Ok(())
    }

    async fn mark_pull_request_closed(
        &self,
        repository_id: i64,
        pull_request_number: i64,
        branch_name: &str,
        merged: bool,
        closed_at: DateTime<Utc>,
    ) -> Result<(), GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        let connection_ids = inner
            .connections
            .values()
            .filter(|connection| connection.repository_id == repository_id.to_string())
            .map(|connection| connection.id.clone())
            .collect::<Vec<_>>();
        for job in inner.jobs.values_mut() {
            if job.status == GitHubIntegrationJobStatus::DraftPrOpen
                && connection_ids.contains(&job.connection_id)
                && job.pull_request_number == Some(pull_request_number)
                && job.branch_name.as_deref() == Some(branch_name)
            {
                job.status = if merged {
                    GitHubIntegrationJobStatus::AwaitingVerification
                } else {
                    GitHubIntegrationJobStatus::ClosedUnmerged
                };
                job.proposed_changes.clear();
                if merged {
                    job.pr_merged_at = Some(closed_at.to_rfc3339());
                }
                job.updated_at = Utc::now().to_rfc3339();
            }
        }
        Ok(())
    }

    async fn create_job(
        &self,
        workspace_id: &str,
        input: GitHubJobCreate,
    ) -> Result<GitHubIntegrationJobSummary, GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        if inner.jobs.values().any(|job| {
            job.workspace_id == workspace_id
                && job.connection_id == input.connection_id.to_string()
                && !matches!(
                    job.status,
                    GitHubIntegrationJobStatus::Verified
                        | GitHubIntegrationJobStatus::ClosedUnmerged
                        | GitHubIntegrationJobStatus::Error
                        | GitHubIntegrationJobStatus::Cancelled
                )
        }) {
            return Err(GitHubIntegrationStoreError::Conflict);
        }
        let now = Utc::now().to_rfc3339();
        let id = Uuid::now_v7().to_string();
        let job = GitHubIntegrationJobSummary {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            connection_id: input.connection_id.to_string(),
            status: GitHubIntegrationJobStatus::Queued,
            risk_statement: input.risk_statement,
            base_branch: input.base_branch,
            base_sha: input.base_sha,
            recipe_version: GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1.to_string(),
            analysis_summary: None,
            proposed_changes: vec![],
            manual_steps: vec![],
            branch_name: None,
            commit_sha: None,
            pull_request_number: None,
            pull_request_url: None,
            error_code: None,
            error_message: None,
            pr_opened_at: None,
            pr_merged_at: None,
            first_verified_trace_at: None,
            created_at: now.clone(),
            updated_at: now,
        };
        inner
            .jobs
            .insert((workspace_id.to_string(), id), job.clone());
        Ok(job)
    }

    async fn get_job(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<GitHubIntegrationJobSummary, GitHubIntegrationStoreError> {
        let inner = self.inner.lock().expect("github memory lock");
        inner
            .jobs
            .get(&(workspace_id.to_string(), job_id.to_string()))
            .cloned()
            .ok_or(GitHubIntegrationStoreError::NotFound)
    }

    async fn transition_job(
        &self,
        workspace_id: &str,
        job_id: &str,
        expected: &[GitHubIntegrationJobStatus],
        next: GitHubIntegrationJobStatus,
        fields: GitHubJobUpdate,
    ) -> Result<GitHubIntegrationJobSummary, GitHubIntegrationStoreError> {
        let mut inner = self.inner.lock().expect("github memory lock");
        let Some(job) = inner
            .jobs
            .get_mut(&(workspace_id.to_string(), job_id.to_string()))
        else {
            return Err(GitHubIntegrationStoreError::NotFound);
        };
        if !expected.contains(&job.status) {
            return Err(GitHubIntegrationStoreError::Conflict);
        }
        job.status = next;
        if let Some(value) = fields.analysis_summary {
            job.analysis_summary = Some(value);
        }
        if let Some(value) = fields.proposed_changes {
            job.proposed_changes = value;
        }
        if fields.clear_proposed_changes {
            job.proposed_changes.clear();
        }
        if let Some(value) = fields.manual_steps {
            job.manual_steps = value;
        }
        if fields.base_sha.is_some() {
            job.base_sha = fields.base_sha;
        }
        if fields.branch_name.is_some() {
            job.branch_name = fields.branch_name;
        }
        if fields.commit_sha.is_some() {
            job.commit_sha = fields.commit_sha;
        }
        if fields.pull_request_number.is_some() {
            job.pull_request_number = fields.pull_request_number;
        }
        if fields.pull_request_url.is_some() {
            job.pull_request_url = fields.pull_request_url;
        }
        job.error_code = fields.error_code.or_else(|| job.error_code.clone());
        job.error_message = fields.error_message.or_else(|| job.error_message.clone());
        if let Some(value) = fields.pr_opened_at {
            job.pr_opened_at = Some(value.to_rfc3339());
        }
        if let Some(value) = fields.pr_merged_at {
            job.pr_merged_at = Some(value.to_rfc3339());
        }
        if let Some(value) = fields.first_verified_trace_at {
            job.first_verified_trace_at = Some(value.to_rfc3339());
        }
        job.updated_at = Utc::now().to_rfc3339();
        Ok(job.clone())
    }

    async fn list_recoverable_jobs(
        &self,
    ) -> Result<Vec<(String, String, GitHubIntegrationJobStatus)>, GitHubIntegrationStoreError>
    {
        let inner = self.inner.lock().expect("github memory lock");
        Ok(inner
            .jobs
            .values()
            .filter(|job| {
                matches!(
                    job.status,
                    GitHubIntegrationJobStatus::Queued
                        | GitHubIntegrationJobStatus::Analyzing
                        | GitHubIntegrationJobStatus::Applying
                )
            })
            .map(|job| (job.workspace_id.clone(), job.id.clone(), job.status))
            .collect())
    }
}
