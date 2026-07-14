//! GitHub-assisted agent installation endpoints and worker.

mod config;
mod github_client;
pub(crate) mod handlers;
mod memory_store;
mod orchestrator;
mod recipe;
mod validation;
pub(crate) mod webhooks;

use std::sync::Arc;

use async_trait::async_trait;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
pub use config::GitHubAppConfig;
pub use github_client::{
    GitHubClient, GitHubClientError, GitHubFile, GitHubPullRequest, GitHubRepository,
    GitHubTreeEntry, ReqwestGitHubClient,
};
pub use memory_store::MemoryGitHubIntegrationStore;
pub use orchestrator::{spawn_github_integration_worker, GitHubIntegrationMessage};
use tl_core::{
    GitHubConnectionSummary, GitHubInstallationSummary, GitHubIntegrationAnalysisSummary,
    GitHubIntegrationJobStatus, GitHubIntegrationJobSummary, GitHubIntegrationManualStep,
    GitHubProposedFileChange,
};

use crate::agents::AgentStore;
use crate::environments::EnvironmentStore;
use crate::team::TeamStore;
use crate::traces::TraceStore;

#[derive(Debug, thiserror::Error)]
pub enum GitHubIntegrationStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("validation: {0}")]
    Validation(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct NewGitHubInstallationState {
    pub state_hash: Vec<u8>,
    pub workspace_id: String,
    pub user_id: uuid::Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ClaimedGitHubInstallationState {
    pub workspace_id: String,
    pub user_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct GitHubInstallationUpsert {
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub repository_selection: tl_core::GitHubRepositorySelection,
    pub installed_by_user_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct GitHubConnectionCreate {
    pub installation_id: uuid::Uuid,
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    pub root_path: String,
    pub agent_id: String,
    pub environment_id: String,
}

#[derive(Debug, Clone)]
pub struct GitHubJobCreate {
    pub connection_id: uuid::Uuid,
    pub risk_statement: String,
    pub base_branch: String,
    pub base_sha: Option<String>,
    pub installation_connected_at: Option<DateTime<Utc>>,
    pub repository_connected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct GitHubJobUpdate {
    pub analysis_summary: Option<GitHubIntegrationAnalysisSummary>,
    pub proposed_changes: Option<Vec<GitHubProposedFileChange>>,
    pub manual_steps: Option<Vec<GitHubIntegrationManualStep>>,
    pub base_sha: Option<String>,
    pub branch_name: Option<String>,
    pub commit_sha: Option<String>,
    pub pull_request_number: Option<i64>,
    pub pull_request_url: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub clear_proposed_changes: bool,
    pub analysis_completed_at: Option<DateTime<Utc>>,
    pub pr_opened_at: Option<DateTime<Utc>>,
    pub pr_merged_at: Option<DateTime<Utc>>,
    pub first_verified_trace_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait GitHubIntegrationStore: Send + Sync {
    async fn create_state(
        &self,
        state: NewGitHubInstallationState,
    ) -> Result<(), GitHubIntegrationStoreError>;
    async fn claim_state(
        &self,
        state_hash: &[u8],
        user_id: uuid::Uuid,
    ) -> Result<ClaimedGitHubInstallationState, GitHubIntegrationStoreError>;
    async fn upsert_installation(
        &self,
        workspace_id: &str,
        input: GitHubInstallationUpsert,
    ) -> Result<GitHubInstallationSummary, GitHubIntegrationStoreError>;
    async fn active_installation(
        &self,
        workspace_id: &str,
    ) -> Result<GitHubInstallationSummary, GitHubIntegrationStoreError>;
    async fn installation_for_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<GitHubInstallationSummary, GitHubIntegrationStoreError>;
    async fn create_connection(
        &self,
        workspace_id: &str,
        input: GitHubConnectionCreate,
    ) -> Result<GitHubConnectionSummary, GitHubIntegrationStoreError>;
    async fn list_connections(
        &self,
        workspace_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<GitHubConnectionSummary>, GitHubIntegrationStoreError>;
    async fn get_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<GitHubConnectionSummary, GitHubIntegrationStoreError>;
    async fn disconnect_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<(), GitHubIntegrationStoreError>;
    async fn mark_installation_removed(
        &self,
        installation_id: i64,
    ) -> Result<(), GitHubIntegrationStoreError>;
    async fn mark_pull_request_closed(
        &self,
        repository_id: i64,
        pull_request_number: i64,
        branch_name: &str,
        merged: bool,
        closed_at: DateTime<Utc>,
    ) -> Result<(), GitHubIntegrationStoreError>;
    async fn create_job(
        &self,
        workspace_id: &str,
        input: GitHubJobCreate,
    ) -> Result<GitHubIntegrationJobSummary, GitHubIntegrationStoreError>;
    async fn get_job(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<GitHubIntegrationJobSummary, GitHubIntegrationStoreError>;
    async fn transition_job(
        &self,
        workspace_id: &str,
        job_id: &str,
        expected: &[GitHubIntegrationJobStatus],
        next: GitHubIntegrationJobStatus,
        fields: GitHubJobUpdate,
    ) -> Result<GitHubIntegrationJobSummary, GitHubIntegrationStoreError>;
    async fn list_recoverable_jobs(
        &self,
    ) -> Result<Vec<(String, String, GitHubIntegrationJobStatus)>, GitHubIntegrationStoreError>;
}

#[derive(Clone)]
pub struct GitHubIntegrationState {
    pub store: Arc<dyn GitHubIntegrationStore>,
    pub team_store: Arc<dyn TeamStore>,
    pub agent_store: Arc<dyn AgentStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    pub trace_store: Arc<dyn TraceStore>,
    pub github: Option<Arc<dyn GitHubClient>>,
    pub llm: Option<Arc<dyn tl_llm::LlmClient>>,
    pub model: String,
    pub worker_tx: Option<tokio::sync::mpsc::Sender<GitHubIntegrationMessage>>,
}

pub(crate) fn store_error_response(error: GitHubIntegrationStoreError) -> axum::response::Response {
    use axum::http::StatusCode;
    use tl_core::{ApiError, ApiErrorCode};

    let (status, code, message) = match error {
        GitHubIntegrationStoreError::NotFound => (
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "GitHub integration resource not found".to_string(),
        ),
        GitHubIntegrationStoreError::Conflict => (
            StatusCode::CONFLICT,
            ApiErrorCode::Invalid,
            "GitHub integration lifecycle conflict".to_string(),
        ),
        GitHubIntegrationStoreError::Validation(message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            message,
        ),
        GitHubIntegrationStoreError::Unavailable(message) => (
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            message,
        ),
        GitHubIntegrationStoreError::Internal(message) => {
            tracing::error!(error = %message, "github integration internal error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "GitHub integration request failed".to_string(),
            )
        }
    };
    axum::Json(ApiError {
        code,
        message,
        retriable: status.is_server_error(),
        details: serde_json::Value::Null,
    })
    .into_response_with_status(status)
}

trait IntoResponseWithStatus {
    fn into_response_with_status(self, status: axum::http::StatusCode) -> axum::response::Response;
}

impl<T: serde::Serialize> IntoResponseWithStatus for axum::Json<T> {
    fn into_response_with_status(self, status: axum::http::StatusCode) -> axum::response::Response {
        (status, self).into_response()
    }
}
