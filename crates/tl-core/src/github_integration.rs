//! Wire types for GitHub-assisted agent installation.
//!
//! Rust owns the durable installation, repository connection, job lifecycle,
//! proposal, and verification state. Web routes are same-origin adapters only.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

pub const GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1: &str = "typescript-nextjs-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GitHubInstallationStatus {
    Active,
    Suspended,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GitHubConnectionStatus {
    Active,
    AccessRemoved,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GitHubIntegrationJobStatus {
    Queued,
    Analyzing,
    AwaitingApproval,
    Applying,
    DraftPrOpen,
    AwaitingVerification,
    Verified,
    ClosedUnmerged,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GitHubRepositorySelection {
    Selected,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GitHubProposedFileOperation {
    Create,
    Update,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubInstallUrlRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub redirect_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubInstallUrlResponse {
    pub install_url: String,
    /// RFC 3339 timestamp.
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubCallbackRequest {
    pub code: String,
    pub state: String,
    pub installation_id: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub setup_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubCallbackResponse {
    pub installation: GitHubInstallationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubInstallationSummary {
    pub id: String,
    pub workspace_id: String,
    pub installation_id: String,
    pub account_login: String,
    pub account_type: String,
    pub repository_selection: GitHubRepositorySelection,
    pub status: GitHubInstallationStatus,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubRepositorySummary {
    pub repository_id: String,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub private: bool,
    pub archived: bool,
    #[serde(default)]
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubRepositoryListResponse {
    pub repositories: Vec<GitHubRepositorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubConnectionCreateRequest {
    pub repository_id: String,
    pub root_path: String,
    pub agent_id: String,
    pub environment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubConnectionSummary {
    pub id: String,
    pub workspace_id: String,
    pub installation_id: String,
    pub repository_id: String,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    pub root_path: String,
    pub agent_id: String,
    pub environment_id: String,
    pub status: GitHubConnectionStatus,
    pub recipe_version: String,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubConnectionListResponse {
    pub connections: Vec<GitHubConnectionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubIntegrationJobCreateRequest {
    pub connection_id: String,
    pub risk_statement: String,
    #[serde(default)]
    pub source_processing_consent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubIntegrationJobSummary {
    pub id: String,
    pub workspace_id: String,
    pub connection_id: String,
    pub status: GitHubIntegrationJobStatus,
    pub risk_statement: String,
    pub base_branch: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub base_sha: Option<String>,
    pub recipe_version: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub analysis_summary: Option<GitHubIntegrationAnalysisSummary>,
    #[serde(default)]
    pub proposed_changes: Vec<GitHubProposedFileChange>,
    #[serde(default)]
    pub manual_steps: Vec<GitHubIntegrationManualStep>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub branch_name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub commit_sha: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub pull_request_number: Option<i64>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub pull_request_url: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub error_code: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub error_message: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub pr_opened_at: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub pr_merged_at: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub first_verified_trace_at: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubIntegrationJobListResponse {
    pub jobs: Vec<GitHubIntegrationJobSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubIntegrationAnalysisSummary {
    pub detected_framework: String,
    pub package_manager: String,
    pub summary: String,
    #[serde(default)]
    pub integration_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubProposedFileChange {
    pub path: String,
    pub operation: GitHubProposedFileOperation,
    pub content_sha: String,
    pub replacement: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubIntegrationManualStep {
    pub label: String,
    pub command: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubIntegrationApproveResponse {
    pub job: GitHubIntegrationJobSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GitHubIntegrationCancelResponse {
    pub job: GitHubIntegrationJobSummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_wire_values_are_snake_case() {
        assert_eq!(
            serde_json::to_string(&GitHubIntegrationJobStatus::AwaitingApproval).unwrap(),
            "\"awaiting_approval\""
        );
        assert_eq!(
            serde_json::to_string(&GitHubInstallationStatus::Active).unwrap(),
            "\"active\""
        );
    }

    #[test]
    fn connection_status_wire_values_are_snake_case() {
        assert_eq!(
            serde_json::to_string(&GitHubConnectionStatus::AccessRemoved).unwrap(),
            "\"access_removed\""
        );
        assert_eq!(
            serde_json::from_str::<GitHubProposedFileOperation>("\"create\"").unwrap(),
            GitHubProposedFileOperation::Create
        );
    }
}
