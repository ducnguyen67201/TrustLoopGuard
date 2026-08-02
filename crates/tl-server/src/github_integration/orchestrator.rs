use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{mpsc, Semaphore};

use super::recipe;
use super::{
    GitHubClient, GitHubDraftPrRequest, GitHubIntegrationStore, GitHubIntegrationStoreError,
    GitHubJobUpdate,
};
use tl_core::GitHubIntegrationJobStatus;

#[derive(Debug, Clone)]
pub enum GitHubIntegrationMessage {
    Analyze {
        workspace_id: String,
        job_id: String,
    },
    Apply {
        workspace_id: String,
        job_id: String,
    },
}

pub fn spawn_github_integration_worker(
    store: Arc<dyn GitHubIntegrationStore>,
    github: Arc<dyn GitHubClient>,
    llm: Arc<dyn tl_llm::LlmClient>,
    model: String,
) -> mpsc::Sender<GitHubIntegrationMessage> {
    let (tx, rx) = mpsc::channel(128);
    tokio::spawn(worker_loop(store.clone(), github, llm, model, rx));
    let recover_tx = tx.clone();
    tokio::spawn(async move {
        match store.list_recoverable_jobs().await {
            Ok(jobs) => {
                for (workspace_id, job_id, status) in jobs {
                    let message = match status {
                        GitHubIntegrationJobStatus::Applying => GitHubIntegrationMessage::Apply {
                            workspace_id,
                            job_id,
                        },
                        _ => GitHubIntegrationMessage::Analyze {
                            workspace_id,
                            job_id,
                        },
                    };
                    let _ = recover_tx.try_send(message);
                }
            }
            Err(error) => {
                tracing::error!(error = %error, "github integration: recovery scan failed");
            }
        }
    });
    tx
}

async fn worker_loop(
    store: Arc<dyn GitHubIntegrationStore>,
    github: Arc<dyn GitHubClient>,
    llm: Arc<dyn tl_llm::LlmClient>,
    model: String,
    mut rx: mpsc::Receiver<GitHubIntegrationMessage>,
) {
    let limiter = Arc::new(Semaphore::new(2));
    while let Some(message) = rx.recv().await {
        let Ok(permit) = limiter.clone().acquire_owned().await else {
            tracing::error!("github integration: semaphore closed");
            break;
        };
        let store = store.clone();
        let github = github.clone();
        let llm = llm.clone();
        let model = model.clone();
        tokio::spawn(async move {
            let _permit = permit;
            match message {
                GitHubIntegrationMessage::Analyze {
                    workspace_id,
                    job_id,
                } => run_analyze(store, github, llm, model, workspace_id, job_id).await,
                GitHubIntegrationMessage::Apply {
                    workspace_id,
                    job_id,
                } => run_apply(store, github, workspace_id, job_id).await,
            }
        });
    }
}

async fn run_analyze(
    store: Arc<dyn GitHubIntegrationStore>,
    github: Arc<dyn GitHubClient>,
    llm: Arc<dyn tl_llm::LlmClient>,
    model: String,
    workspace_id: String,
    job_id: String,
) {
    let job = match store
        .transition_job(
            &workspace_id,
            &job_id,
            &[
                GitHubIntegrationJobStatus::Queued,
                GitHubIntegrationJobStatus::Analyzing,
            ],
            GitHubIntegrationJobStatus::Analyzing,
            GitHubJobUpdate::default(),
        )
        .await
    {
        Ok(job) => job,
        Err(GitHubIntegrationStoreError::Conflict) => return,
        Err(error) => {
            tracing::error!(job_id, error = %error, "github integration: cannot start analysis");
            return;
        }
    };
    let connection = match store
        .get_connection(&workspace_id, &job.connection_id)
        .await
    {
        Ok(connection) => connection,
        Err(error) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "connection_missing",
                &error.to_string(),
            )
            .await;
            return;
        }
    };
    let installation = match store
        .installation_for_connection(&workspace_id, &connection.id)
        .await
    {
        Ok(installation) => installation,
        Err(error) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "installation_missing",
                &error.to_string(),
            )
            .await;
            return;
        }
    };
    let installation_id = match installation.installation_id.parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "installation_invalid",
                "invalid installation id",
            )
            .await;
            return;
        }
    };
    match recipe::analyze(
        github.as_ref(),
        llm.as_ref(),
        &model,
        installation_id,
        &connection,
        &job.risk_statement,
    )
    .await
    {
        Ok(result) => {
            let update = GitHubJobUpdate {
                analysis_summary: Some(result.summary),
                proposed_changes: Some(result.proposed_changes),
                manual_steps: Some(result.manual_steps),
                base_sha: Some(result.base_sha),
                analysis_completed_at: Some(Utc::now()),
                ..GitHubJobUpdate::default()
            };
            if let Err(error) = store
                .transition_job(
                    &workspace_id,
                    &job_id,
                    &[GitHubIntegrationJobStatus::Analyzing],
                    GitHubIntegrationJobStatus::AwaitingApproval,
                    update,
                )
                .await
            {
                tracing::error!(job_id, error = %error, "github integration: cannot persist analysis");
            }
        }
        Err(error) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "analysis_failed",
                &error.to_string(),
            )
            .await;
        }
    }
}

async fn run_apply(
    store: Arc<dyn GitHubIntegrationStore>,
    github: Arc<dyn GitHubClient>,
    workspace_id: String,
    job_id: String,
) {
    let job_before = match store.get_job(&workspace_id, &job_id).await {
        Ok(job) => job,
        Err(error) => {
            tracing::error!(job_id, error = %error, "github integration: cannot load apply job");
            return;
        }
    };
    if job_before.proposed_changes.is_empty() {
        mark_error(
            store.as_ref(),
            &workspace_id,
            &job_id,
            "proposal_missing",
            "proposal was already redacted or missing",
        )
        .await;
        return;
    }
    let job = match store
        .transition_job(
            &workspace_id,
            &job_id,
            &[
                GitHubIntegrationJobStatus::AwaitingApproval,
                GitHubIntegrationJobStatus::Applying,
            ],
            GitHubIntegrationJobStatus::Applying,
            GitHubJobUpdate::default(),
        )
        .await
    {
        Ok(job) => job,
        Err(GitHubIntegrationStoreError::Conflict) => return,
        Err(error) => {
            tracing::error!(job_id, error = %error, "github integration: cannot start apply");
            return;
        }
    };
    let connection = match store
        .get_connection(&workspace_id, &job.connection_id)
        .await
    {
        Ok(connection) => connection,
        Err(error) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "connection_missing",
                &error.to_string(),
            )
            .await;
            return;
        }
    };
    let installation = match store
        .installation_for_connection(&workspace_id, &connection.id)
        .await
    {
        Ok(installation) => installation,
        Err(error) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "installation_missing",
                &error.to_string(),
            )
            .await;
            return;
        }
    };
    let installation_id = match installation.installation_id.parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "installation_invalid",
                "invalid installation id",
            )
            .await;
            return;
        }
    };
    let Some(base_sha) = job_before.base_sha.as_deref() else {
        mark_error(
            store.as_ref(),
            &workspace_id,
            &job_id,
            "base_sha_missing",
            "analysis did not capture a base SHA",
        )
        .await;
        return;
    };
    let branch_name = format!(
        "featherlane-ai/integrate-{}",
        &job_id[..8.min(job_id.len())]
    );
    let body = pr_body(&job_before, &connection, &branch_name);
    match github
        .create_draft_pr(GitHubDraftPrRequest {
            installation_id,
            owner: connection.owner.clone(),
            repo: connection.name.clone(),
            base_branch: connection.default_branch.clone(),
            base_sha: base_sha.to_string(),
            branch_name,
            changes: job_before.proposed_changes.clone(),
            title: "Integrate Featherlane AI".to_string(),
            body,
        })
        .await
    {
        Ok(pr) => {
            let update = GitHubJobUpdate {
                branch_name: Some(pr.branch_name),
                commit_sha: Some(pr.commit_sha),
                pull_request_number: Some(pr.number),
                pull_request_url: Some(pr.url),
                pr_opened_at: Some(Utc::now()),
                clear_proposed_changes: true,
                ..GitHubJobUpdate::default()
            };
            if let Err(error) = store
                .transition_job(
                    &workspace_id,
                    &job_id,
                    &[GitHubIntegrationJobStatus::Applying],
                    GitHubIntegrationJobStatus::DraftPrOpen,
                    update,
                )
                .await
            {
                tracing::error!(job_id, error = %error, "github integration: cannot persist draft PR");
            }
        }
        Err(error) => {
            mark_error(
                store.as_ref(),
                &workspace_id,
                &job_id,
                "apply_failed",
                &error.to_string(),
            )
            .await;
        }
    }
}

async fn mark_error(
    store: &dyn GitHubIntegrationStore,
    workspace_id: &str,
    job_id: &str,
    code: &str,
    message: &str,
) {
    let _ = store
        .transition_job(
            workspace_id,
            job_id,
            &[
                GitHubIntegrationJobStatus::Queued,
                GitHubIntegrationJobStatus::Analyzing,
                GitHubIntegrationJobStatus::Applying,
                GitHubIntegrationJobStatus::AwaitingApproval,
            ],
            GitHubIntegrationJobStatus::Error,
            GitHubJobUpdate {
                error_code: Some(code.to_string()),
                error_message: Some(message.to_string()),
                clear_proposed_changes: true,
                ..GitHubJobUpdate::default()
            },
        )
        .await;
}

fn pr_body(
    job: &tl_core::GitHubIntegrationJobSummary,
    connection: &tl_core::GitHubConnectionSummary,
    branch: &str,
) -> String {
    let files = job
        .proposed_changes
        .iter()
        .map(|change| format!("- `{}`: {}", change.path, change.rationale))
        .collect::<Vec<_>>()
        .join("\n");
    let manual_steps = job
        .manual_steps
        .iter()
        .map(|step| format!("- {}: `{}` ({})", step.label, step.command, step.reason))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "## Featherlane AI integration\n\n\
         This draft PR was opened after explicit approval in Featherlane AI. It has not been merged, deployed, or tested by Featherlane AI.\n\n\
         Risk guarded: {}\n\n\
         Agent: `{}`\n\
         Environment: `{}`\n\
         Activation marker: `{}`\n\
         Branch: `{}`\n\n\
         Files:\n{}\n\n\
         Manual steps:\n{}\n\n\
         Configure `FEATHERLANE_AI_API_KEY` and `FEATHERLANE_AI_URL` in your deployment environment before merging. Run your repository CI and refresh the lockfile locally if package.json changed.",
        job.risk_statement,
        connection.agent_id,
        connection.environment_id,
        connection.id,
        branch,
        files,
        if manual_steps.is_empty() { "- Run your existing tests and CI.".to_string() } else { manual_steps },
    )
}
