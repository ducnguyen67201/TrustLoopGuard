use chrono::{DateTime, Utc};
use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use tl_core::{
    GitHubConnectionStatus, GitHubConnectionSummary, GitHubInstallationStatus,
    GitHubInstallationSummary, GitHubIntegrationAnalysisSummary, GitHubIntegrationJobStatus,
    GitHubIntegrationJobSummary, GitHubIntegrationManualStep, GitHubProposedFileChange,
    GitHubRepositorySelection, GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1,
};
use uuid::Uuid;

use crate::models::{
    GitHubInstallationRecord, GitHubIntegrationJobRecord, GitHubRepositoryConnectionRecord,
    NewGitHubInstallation, NewGitHubInstallationState, NewGitHubIntegrationJob,
    NewGitHubRepositoryConnection,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    github_installation_states, github_installations, github_integration_jobs,
    github_repository_connections,
};
use crate::StorageError;

#[derive(Clone)]
pub struct GitHubIntegrationRepo {
    pool: DbPool,
}

#[derive(Debug, Clone)]
pub struct NewInstallationState {
    pub state_hash: Vec<u8>,
    pub workspace_id: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ClaimedInstallationState {
    pub workspace_id: String,
    pub user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct UpsertInstallation {
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub repository_selection: GitHubRepositorySelection,
    pub installed_by_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CreateConnection {
    pub installation_id: Uuid,
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    pub root_path: String,
    pub agent_id: String,
    pub environment_id: String,
}

#[derive(Debug, Clone)]
pub struct CreateJob {
    pub connection_id: Uuid,
    pub risk_statement: String,
    pub base_branch: String,
    pub base_sha: Option<String>,
    pub installation_connected_at: Option<DateTime<Utc>>,
    pub repository_connected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct JobTransition {
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

impl GitHubIntegrationRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_state(&self, state: NewInstallationState) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let row = NewGitHubInstallationState {
            state_hash: state.state_hash,
            workspace_id: state.workspace_id,
            user_id: state.user_id,
            expires_at: state.expires_at,
        };
        diesel::insert_into(github_installation_states::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("github state create: {e}")))?;
        Ok(())
    }

    pub async fn claim_state(
        &self,
        state_hash: &[u8],
        user_id: Uuid,
    ) -> Result<ClaimedInstallationState, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::update(
            github_installation_states::table
                .filter(github_installation_states::state_hash.eq(state_hash))
                .filter(github_installation_states::user_id.eq(user_id))
                .filter(github_installation_states::consumed_at.is_null())
                .filter(github_installation_states::expires_at.gt(now)),
        )
        .set(github_installation_states::consumed_at.eq(now))
        .returning((
            github_installation_states::workspace_id,
            github_installation_states::user_id,
        ))
        .get_result::<(String, Uuid)>(&mut conn)
        .await
        .optional()
        .map_err(|e| StorageError::Internal(format!("github state claim: {e}")))?
        .ok_or(StorageError::NotFound)?;
        Ok(ClaimedInstallationState {
            workspace_id: row.0,
            user_id: row.1,
        })
    }

    pub async fn upsert_installation(
        &self,
        workspace_id: &str,
        input: UpsertInstallation,
    ) -> Result<GitHubInstallationSummary, StorageError> {
        let id = Uuid::now_v7();
        let row = NewGitHubInstallation {
            workspace_id: workspace_id.to_string(),
            id,
            installation_id: input.installation_id,
            account_login: input.account_login,
            account_type: input.account_type,
            repository_selection: selection_text(input.repository_selection).to_string(),
            status: installation_status_text(GitHubInstallationStatus::Active).to_string(),
            installed_by_user_id: input.installed_by_user_id,
        };
        let mut conn = self.connection().await?;
        let record = diesel::insert_into(github_installations::table)
            .values(&row)
            .on_conflict((
                github_installations::workspace_id,
                github_installations::installation_id,
            ))
            .do_update()
            .set((
                github_installations::account_login
                    .eq(excluded(github_installations::account_login)),
                github_installations::account_type.eq(excluded(github_installations::account_type)),
                github_installations::repository_selection
                    .eq(excluded(github_installations::repository_selection)),
                github_installations::status.eq(excluded(github_installations::status)),
                github_installations::installed_by_user_id
                    .eq(excluded(github_installations::installed_by_user_id)),
                github_installations::updated_at.eq(now),
            ))
            .returning(GitHubInstallationRecord::as_returning())
            .get_result::<GitHubInstallationRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("github installation upsert: {e}")))?;
        Ok(installation_summary(record))
    }

    pub async fn active_installation(
        &self,
        workspace_id: &str,
    ) -> Result<GitHubInstallationSummary, StorageError> {
        let mut conn = self.connection().await?;
        let record = github_installations::table
            .filter(github_installations::workspace_id.eq(workspace_id))
            .filter(github_installations::status.eq("active"))
            .order(github_installations::updated_at.desc())
            .select(GitHubInstallationRecord::as_select())
            .first::<GitHubInstallationRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("github installation get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(installation_summary(record))
    }

    pub async fn installation_for_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<GitHubInstallationSummary, StorageError> {
        let connection_id = parse_uuid(connection_id)?;
        let mut conn = self.connection().await?;
        let installation_id = github_repository_connections::table
            .filter(github_repository_connections::workspace_id.eq(workspace_id))
            .filter(github_repository_connections::id.eq(connection_id))
            .select(github_repository_connections::installation_id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("github connection install get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        let record = github_installations::table
            .filter(github_installations::workspace_id.eq(workspace_id))
            .filter(github_installations::id.eq(installation_id))
            .filter(github_installations::status.eq("active"))
            .select(GitHubInstallationRecord::as_select())
            .first::<GitHubInstallationRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("github installation by connection: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(installation_summary(record))
    }

    pub async fn create_connection(
        &self,
        workspace_id: &str,
        input: CreateConnection,
    ) -> Result<GitHubConnectionSummary, StorageError> {
        let new_row = NewGitHubRepositoryConnection {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            installation_id: input.installation_id,
            repository_id: input.repository_id,
            owner: input.owner,
            name: input.name,
            default_branch: input.default_branch,
            root_path: input.root_path,
            agent_id: input.agent_id,
            environment_id: input.environment_id,
            status: connection_status_text(GitHubConnectionStatus::Active).to_string(),
            recipe_version: GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1.to_string(),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(github_repository_connections::table)
            .values(&new_row)
            .execute(&mut conn)
            .await
            .map_err(|e| map_constraint(e, "github connection create"))?;
        self.get_connection(workspace_id, &new_row.id.to_string())
            .await
    }

    pub async fn list_connections(
        &self,
        workspace_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<GitHubConnectionSummary>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = github_repository_connections::table
            .filter(github_repository_connections::workspace_id.eq(workspace_id))
            .filter(github_repository_connections::status.ne("disconnected"))
            .into_boxed();
        if let Some(agent_id) = agent_id {
            query = query.filter(github_repository_connections::agent_id.eq(agent_id));
        }
        let rows = query
            .select(GitHubRepositoryConnectionRecord::as_select())
            .order(github_repository_connections::created_at.desc())
            .load::<GitHubRepositoryConnectionRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("github connection list: {e}")))?;
        Ok(rows.into_iter().map(connection_summary).collect())
    }

    pub async fn get_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<GitHubConnectionSummary, StorageError> {
        let id = parse_uuid(connection_id)?;
        let mut conn = self.connection().await?;
        let record = github_repository_connections::table
            .filter(github_repository_connections::workspace_id.eq(workspace_id))
            .filter(github_repository_connections::id.eq(id))
            .select(GitHubRepositoryConnectionRecord::as_select())
            .first::<GitHubRepositoryConnectionRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("github connection get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(connection_summary(record))
    }

    pub async fn disconnect_connection(
        &self,
        workspace_id: &str,
        connection_id: &str,
    ) -> Result<(), StorageError> {
        let id = parse_uuid(connection_id)?;
        let mut conn = self.connection().await?;
        let changed = diesel::update(
            github_repository_connections::table
                .filter(github_repository_connections::workspace_id.eq(workspace_id))
                .filter(github_repository_connections::id.eq(id)),
        )
        .set((
            github_repository_connections::status.eq("disconnected"),
            github_repository_connections::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("github connection disconnect: {e}")))?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub async fn mark_installation_removed(
        &self,
        installation_id: i64,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::update(
            github_installations::table
                .filter(github_installations::installation_id.eq(installation_id)),
        )
        .set((
            github_installations::status.eq("removed"),
            github_installations::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("github installation remove: {e}")))?;
        diesel::update(
            github_repository_connections::table.filter(
                github_repository_connections::installation_id.eq_any(
                    github_installations::table
                        .filter(github_installations::installation_id.eq(installation_id))
                        .select(github_installations::id),
                ),
            ),
        )
        .set((
            github_repository_connections::status.eq("access_removed"),
            github_repository_connections::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("github connections remove: {e}")))?;
        Ok(())
    }

    pub async fn mark_pull_request_closed(
        &self,
        repository_id: i64,
        pull_request_number: i64,
        branch_name: &str,
        merged: bool,
        closed_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let next = if merged {
            GitHubIntegrationJobStatus::AwaitingVerification
        } else {
            GitHubIntegrationJobStatus::ClosedUnmerged
        };
        let pr_merged_at = if merged { Some(closed_at) } else { None };
        let mut conn = self.connection().await?;
        diesel::update(
            github_integration_jobs::table
                .filter(github_integration_jobs::status.eq("draft_pr_open"))
                .filter(github_integration_jobs::pull_request_number.eq(pull_request_number))
                .filter(github_integration_jobs::branch_name.eq(branch_name))
                .filter(
                    github_integration_jobs::connection_id.eq_any(
                        github_repository_connections::table
                            .filter(github_repository_connections::repository_id.eq(repository_id))
                            .select(github_repository_connections::id),
                    ),
                ),
        )
        .set((
            github_integration_jobs::status.eq(job_status_text(next)),
            github_integration_jobs::proposed_changes.eq(serde_json::Value::Array(vec![])),
            github_integration_jobs::pr_merged_at.eq(pr_merged_at),
            github_integration_jobs::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("github pull request close: {e}")))?;
        Ok(())
    }

    pub async fn create_job(
        &self,
        workspace_id: &str,
        input: CreateJob,
    ) -> Result<GitHubIntegrationJobSummary, StorageError> {
        let id = Uuid::now_v7();
        let row = NewGitHubIntegrationJob {
            workspace_id: workspace_id.to_string(),
            id,
            connection_id: input.connection_id,
            status: job_status_text(GitHubIntegrationJobStatus::Queued).to_string(),
            risk_statement: input.risk_statement,
            base_branch: input.base_branch,
            base_sha: input.base_sha,
            recipe_version: GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1.to_string(),
            proposed_changes: serde_json::Value::Array(vec![]),
            manual_steps: serde_json::Value::Array(vec![]),
            installation_connected_at: input.installation_connected_at,
            repository_connected_at: input.repository_connected_at,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(github_integration_jobs::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .map_err(|e| map_constraint(e, "github job create"))?;
        self.get_job(workspace_id, &id.to_string()).await
    }

    pub async fn get_job(
        &self,
        workspace_id: &str,
        job_id: &str,
    ) -> Result<GitHubIntegrationJobSummary, StorageError> {
        let id = parse_uuid(job_id)?;
        let mut conn = self.connection().await?;
        let record = github_integration_jobs::table
            .filter(github_integration_jobs::workspace_id.eq(workspace_id))
            .filter(github_integration_jobs::id.eq(id))
            .select(GitHubIntegrationJobRecord::as_select())
            .first::<GitHubIntegrationJobRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("github job get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        job_summary(record)
    }

    pub async fn transition_job(
        &self,
        workspace_id: &str,
        job_id: &str,
        expected: &[GitHubIntegrationJobStatus],
        next: GitHubIntegrationJobStatus,
        fields: JobTransition,
    ) -> Result<GitHubIntegrationJobSummary, StorageError> {
        let id = parse_uuid(job_id)?;
        let expected = expected
            .iter()
            .map(|s| job_status_text(*s))
            .collect::<Vec<_>>();
        let proposed_changes = if fields.clear_proposed_changes {
            Some(serde_json::Value::Array(vec![]))
        } else {
            fields
                .proposed_changes
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|e| StorageError::Internal(format!("github proposal encode: {e}")))?
        };
        let analysis_summary = fields
            .analysis_summary
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| StorageError::Internal(format!("github analysis encode: {e}")))?;
        let manual_steps = fields
            .manual_steps
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| StorageError::Internal(format!("github manual steps encode: {e}")))?;
        let mut conn = self.connection().await?;
        let current = github_integration_jobs::table
            .filter(github_integration_jobs::workspace_id.eq(workspace_id))
            .filter(github_integration_jobs::id.eq(id))
            .select(GitHubIntegrationJobRecord::as_select())
            .first::<GitHubIntegrationJobRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("github job transition load: {e}")))?
            .ok_or(StorageError::NotFound)?;
        if !expected.contains(&current.status.as_str()) {
            return Err(StorageError::Conflict);
        }
        let analysis_summary = analysis_summary.or(current.analysis_summary);
        let proposed_changes = proposed_changes.unwrap_or(current.proposed_changes);
        let manual_steps = manual_steps.unwrap_or(current.manual_steps);
        let base_sha = fields.base_sha.or(current.base_sha);
        let branch_name = fields.branch_name.or(current.branch_name);
        let commit_sha = fields.commit_sha.or(current.commit_sha);
        let pull_request_number = fields.pull_request_number.or(current.pull_request_number);
        let pull_request_url = fields.pull_request_url.or(current.pull_request_url);
        let error_code = fields.error_code.or(current.error_code);
        let error_message = fields.error_message.or(current.error_message);
        let analysis_completed_at = fields
            .analysis_completed_at
            .or(current.analysis_completed_at);
        let pr_opened_at = fields.pr_opened_at.or(current.pr_opened_at);
        let pr_merged_at = fields.pr_merged_at.or(current.pr_merged_at);
        let first_verified_trace_at = fields
            .first_verified_trace_at
            .or(current.first_verified_trace_at);
        let changed = diesel::update(
            github_integration_jobs::table
                .filter(github_integration_jobs::workspace_id.eq(workspace_id))
                .filter(github_integration_jobs::id.eq(id))
                .filter(github_integration_jobs::status.eq_any(expected)),
        )
        .set((
            github_integration_jobs::status.eq(job_status_text(next)),
            github_integration_jobs::analysis_summary.eq(analysis_summary),
            github_integration_jobs::proposed_changes.eq(proposed_changes),
            github_integration_jobs::manual_steps.eq(manual_steps),
            github_integration_jobs::base_sha.eq(base_sha),
            github_integration_jobs::branch_name.eq(branch_name),
            github_integration_jobs::commit_sha.eq(commit_sha),
            github_integration_jobs::pull_request_number.eq(pull_request_number),
            github_integration_jobs::pull_request_url.eq(pull_request_url),
            github_integration_jobs::error_code.eq(error_code),
            github_integration_jobs::error_message.eq(error_message),
            github_integration_jobs::analysis_completed_at.eq(analysis_completed_at),
            github_integration_jobs::pr_opened_at.eq(pr_opened_at),
            github_integration_jobs::pr_merged_at.eq(pr_merged_at),
            github_integration_jobs::first_verified_trace_at.eq(first_verified_trace_at),
            github_integration_jobs::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("github job transition: {e}")))?;
        if changed == 0 {
            return Err(StorageError::Conflict);
        }
        self.get_job(workspace_id, job_id).await
    }

    pub async fn list_recoverable_jobs(
        &self,
    ) -> Result<Vec<(String, String, GitHubIntegrationJobStatus)>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = github_integration_jobs::table
            .filter(github_integration_jobs::status.eq_any(["queued", "analyzing", "applying"]))
            .select((
                github_integration_jobs::workspace_id,
                github_integration_jobs::id,
                github_integration_jobs::status,
            ))
            .load::<(String, Uuid, String)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("github recoverable jobs: {e}")))?;
        rows.into_iter()
            .map(|(workspace_id, id, status)| {
                let status = parse_job_status(&status)?;
                Ok((workspace_id, id.to_string(), status))
            })
            .collect()
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

fn parse_uuid(id: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(id).map_err(|_| StorageError::NotFound)
}

fn map_constraint(error: diesel::result::Error, operation: &str) -> StorageError {
    match error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => StorageError::Conflict,
        other => StorageError::Internal(format!("{operation}: {other}")),
    }
}

fn installation_summary(record: GitHubInstallationRecord) -> GitHubInstallationSummary {
    GitHubInstallationSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        installation_id: record.installation_id.to_string(),
        account_login: record.account_login,
        account_type: record.account_type,
        repository_selection: parse_selection(&record.repository_selection),
        status: parse_installation_status(&record.status),
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

fn connection_summary(record: GitHubRepositoryConnectionRecord) -> GitHubConnectionSummary {
    GitHubConnectionSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        installation_id: record.installation_id.to_string(),
        repository_id: record.repository_id.to_string(),
        owner: record.owner,
        name: record.name,
        default_branch: record.default_branch,
        root_path: record.root_path,
        agent_id: record.agent_id,
        environment_id: record.environment_id,
        status: parse_connection_status(&record.status),
        recipe_version: record.recipe_version,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

fn job_summary(
    record: GitHubIntegrationJobRecord,
) -> Result<GitHubIntegrationJobSummary, StorageError> {
    Ok(GitHubIntegrationJobSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        connection_id: record.connection_id.to_string(),
        status: parse_job_status(&record.status)?,
        risk_statement: record.risk_statement,
        base_branch: record.base_branch,
        base_sha: record.base_sha,
        recipe_version: record.recipe_version,
        analysis_summary: decode_optional(record.analysis_summary, "analysis_summary")?,
        proposed_changes: decode_value(record.proposed_changes, "proposed_changes")?,
        manual_steps: decode_value(record.manual_steps, "manual_steps")?,
        branch_name: record.branch_name,
        commit_sha: record.commit_sha,
        pull_request_number: record.pull_request_number,
        pull_request_url: record.pull_request_url,
        error_code: record.error_code,
        error_message: record.error_message,
        pr_opened_at: record.pr_opened_at.map(|t| t.to_rfc3339()),
        pr_merged_at: record.pr_merged_at.map(|t| t.to_rfc3339()),
        first_verified_trace_at: record.first_verified_trace_at.map(|t| t.to_rfc3339()),
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    })
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    value: Option<serde_json::Value>,
    field: &str,
) -> Result<Option<T>, StorageError> {
    value
        .map(|v| {
            serde_json::from_value(v).map_err(|e| StorageError::Internal(format!("{field}: {e}")))
        })
        .transpose()
}

fn decode_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
    field: &str,
) -> Result<T, StorageError> {
    serde_json::from_value(value).map_err(|e| StorageError::Internal(format!("{field}: {e}")))
}

fn selection_text(status: GitHubRepositorySelection) -> &'static str {
    match status {
        GitHubRepositorySelection::Selected => "selected",
        GitHubRepositorySelection::All => "all",
    }
}

fn parse_selection(value: &str) -> GitHubRepositorySelection {
    match value {
        "all" => GitHubRepositorySelection::All,
        _ => GitHubRepositorySelection::Selected,
    }
}

fn installation_status_text(status: GitHubInstallationStatus) -> &'static str {
    match status {
        GitHubInstallationStatus::Active => "active",
        GitHubInstallationStatus::Suspended => "suspended",
        GitHubInstallationStatus::Removed => "removed",
    }
}

fn parse_installation_status(value: &str) -> GitHubInstallationStatus {
    match value {
        "suspended" => GitHubInstallationStatus::Suspended,
        "removed" => GitHubInstallationStatus::Removed,
        _ => GitHubInstallationStatus::Active,
    }
}

fn connection_status_text(status: GitHubConnectionStatus) -> &'static str {
    match status {
        GitHubConnectionStatus::Active => "active",
        GitHubConnectionStatus::AccessRemoved => "access_removed",
        GitHubConnectionStatus::Disconnected => "disconnected",
    }
}

fn parse_connection_status(value: &str) -> GitHubConnectionStatus {
    match value {
        "access_removed" => GitHubConnectionStatus::AccessRemoved,
        "disconnected" => GitHubConnectionStatus::Disconnected,
        _ => GitHubConnectionStatus::Active,
    }
}

pub fn job_status_text(status: GitHubIntegrationJobStatus) -> &'static str {
    match status {
        GitHubIntegrationJobStatus::Queued => "queued",
        GitHubIntegrationJobStatus::Analyzing => "analyzing",
        GitHubIntegrationJobStatus::AwaitingApproval => "awaiting_approval",
        GitHubIntegrationJobStatus::Applying => "applying",
        GitHubIntegrationJobStatus::DraftPrOpen => "draft_pr_open",
        GitHubIntegrationJobStatus::AwaitingVerification => "awaiting_verification",
        GitHubIntegrationJobStatus::Verified => "verified",
        GitHubIntegrationJobStatus::ClosedUnmerged => "closed_unmerged",
        GitHubIntegrationJobStatus::Error => "error",
        GitHubIntegrationJobStatus::Cancelled => "cancelled",
    }
}

pub fn parse_job_status(value: &str) -> Result<GitHubIntegrationJobStatus, StorageError> {
    match value {
        "queued" => Ok(GitHubIntegrationJobStatus::Queued),
        "analyzing" => Ok(GitHubIntegrationJobStatus::Analyzing),
        "awaiting_approval" => Ok(GitHubIntegrationJobStatus::AwaitingApproval),
        "applying" => Ok(GitHubIntegrationJobStatus::Applying),
        "draft_pr_open" => Ok(GitHubIntegrationJobStatus::DraftPrOpen),
        "awaiting_verification" => Ok(GitHubIntegrationJobStatus::AwaitingVerification),
        "verified" => Ok(GitHubIntegrationJobStatus::Verified),
        "closed_unmerged" => Ok(GitHubIntegrationJobStatus::ClosedUnmerged),
        "error" => Ok(GitHubIntegrationJobStatus::Error),
        "cancelled" => Ok(GitHubIntegrationJobStatus::Cancelled),
        other => Err(StorageError::Internal(format!(
            "unknown github job status `{other}`"
        ))),
    }
}
