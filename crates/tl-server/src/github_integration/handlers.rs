use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{
    ApiError, ApiErrorCode, GitHubCallbackRequest, GitHubCallbackResponse,
    GitHubConnectionCreateRequest, GitHubConnectionListResponse, GitHubInstallUrlRequest,
    GitHubInstallUrlResponse, GitHubIntegrationApproveResponse, GitHubIntegrationCancelResponse,
    GitHubIntegrationJobCreateRequest, GitHubIntegrationJobStatus, GitHubIntegrationJobSummary,
    GitHubRepositoryListResponse, GitHubRepositorySummary,
};
use uuid::Uuid;

use super::config::GitHubAppConfig;
use super::validation::{normalize_root_path, state_hash, validate_risk_statement};
use super::{
    store_error_response, GitHubClientError, GitHubConnectionCreate, GitHubInstallationUpsert,
    GitHubIntegrationMessage, GitHubIntegrationState, GitHubJobCreate, GitHubJobUpdate,
    NewGitHubInstallationState,
};
use crate::auth::{InternalServiceContext, WorkspaceKeyContext};
use crate::dashboard_admin::{authorize_workspace_admin, authorize_workspace_admin_for_workspace};
use crate::jwt::UserContext;

const INSTALL_STATE_TTL_MINUTES: i64 = 10;

#[utoipa::path(
    post,
    path = "/v1/github-integration/install-url",
    tag = "github-integration",
    request_body = GitHubInstallUrlRequest,
    responses(
        (status = 200, description = "GitHub App installation URL", body = GitHubInstallUrlResponse),
        (status = 403, description = "Workspace owner/admin required", body = ApiError),
        (status = 503, description = "GitHub App not configured", body = ApiError),
    ),
)]
pub async fn install_url(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Json(_input): Json<GitHubInstallUrlRequest>,
) -> Response {
    let (workspace_id, Some(user_id)) = (match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "connect GitHub repositories",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    }) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            ApiErrorCode::Unauthorized,
            "authenticated user is required to connect GitHub repositories",
        );
    };
    let config = match GitHubAppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::info!(error = %error, "github integration unavailable");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                ApiErrorCode::Unavailable,
                "GitHub integration is not configured for this deployment",
            );
        }
    };
    let raw_state = random_state();
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(INSTALL_STATE_TTL_MINUTES);
    if let Err(error) = state
        .store
        .create_state(NewGitHubInstallationState {
            state_hash: state_hash(&raw_state),
            workspace_id,
            user_id,
            expires_at,
        })
        .await
    {
        return store_error_response(error);
    }
    Json(GitHubInstallUrlResponse {
        install_url: config.install_url(&raw_state),
        expires_at: expires_at.to_rfc3339(),
    })
    .into_response()
}

#[utoipa::path(
    post,
    path = "/v1/github-integration/callback",
    tag = "github-integration",
    request_body = GitHubCallbackRequest,
    responses(
        (status = 200, description = "GitHub App installation connected", body = GitHubCallbackResponse),
        (status = 403, description = "Invalid installation proof or role", body = ApiError),
        (status = 503, description = "GitHub App not configured", body = ApiError),
    ),
)]
pub async fn callback(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Json(input): Json<GitHubCallbackRequest>,
) -> Response {
    let user_id = match request_user_id(&headers, user.clone(), internal) {
        Ok(user_id) => user_id,
        Err(response) => return *response,
    };
    if runtime_key.is_some() {
        return api_error(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace runtime keys cannot connect GitHub repositories",
        );
    }
    let claimed = match state
        .store
        .claim_state(&state_hash(&input.state), user_id)
        .await
    {
        Ok(claimed) => claimed,
        Err(error) => return store_error_response(error),
    };
    let caller_user_id = match authorize_workspace_admin_for_workspace(
        &state.team_store,
        &claimed.workspace_id,
        &headers,
        user,
        Some(Extension(InternalServiceContext)),
        None,
        "connect GitHub repositories",
    )
    .await
    {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    if caller_user_id != claimed.user_id {
        return api_error(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "GitHub callback state belongs to a different user",
        );
    }
    let Some(github) = state.github.clone() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "GitHub integration is not configured for this deployment",
        );
    };
    let installation_id = match input.installation_id.parse::<i64>() {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "invalid installation_id",
            );
        }
    };
    let proof = match github
        .verify_callback_installation(&input.code, installation_id)
        .await
    {
        Ok(proof) => proof,
        Err(GitHubClientError::Auth) => {
            return api_error(
                StatusCode::FORBIDDEN,
                ApiErrorCode::Forbidden,
                "GitHub installation could not be verified for this user",
            );
        }
        Err(error) => return upstream_error(error),
    };
    match state
        .store
        .upsert_installation(
            &claimed.workspace_id,
            GitHubInstallationUpsert {
                installation_id: proof.installation_id,
                account_login: proof.account_login,
                account_type: proof.account_type,
                repository_selection: proof.repository_selection,
                installed_by_user_id: user_id,
            },
        )
        .await
    {
        Ok(installation) => Json(GitHubCallbackResponse { installation }).into_response(),
        Err(error) => store_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/github-integration/repositories",
    tag = "github-integration",
    responses((status = 200, description = "Repositories available to the installation", body = GitHubRepositoryListResponse)),
)]
pub async fn repositories(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "list GitHub repositories",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(github) = state.github.clone() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "GitHub integration is not configured",
        );
    };
    let installation = match state.store.active_installation(&workspace_id).await {
        Ok(installation) => installation,
        Err(error) => return store_error_response(error),
    };
    let installation_id = match installation.installation_id.parse::<i64>() {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                ApiErrorCode::Unavailable,
                "stored GitHub installation is invalid",
            )
        }
    };
    match github.list_repositories(installation_id).await {
        Ok(repositories) => Json(GitHubRepositoryListResponse {
            repositories: repositories
                .into_iter()
                .map(|repo| GitHubRepositorySummary {
                    repository_id: repo.repository_id.to_string(),
                    owner: repo.owner,
                    name: repo.name,
                    full_name: repo.full_name,
                    default_branch: repo.default_branch,
                    private: repo.private,
                    archived: repo.archived,
                    connected: false,
                })
                .collect(),
        })
        .into_response(),
        Err(error) => upstream_error(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/github-integration/connections",
    tag = "github-integration",
    responses((status = 200, description = "GitHub repository connections", body = GitHubConnectionListResponse)),
)]
pub async fn list_connections(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    uri: Uri,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "list GitHub repository connections",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let agent_id = read_query(uri.query(), "agent_id");
    match state
        .store
        .list_connections(&workspace_id, agent_id.as_deref())
        .await
    {
        Ok(connections) => Json(GitHubConnectionListResponse { connections }).into_response(),
        Err(error) => store_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/github-integration/connections",
    tag = "github-integration",
    request_body = GitHubConnectionCreateRequest,
    responses((status = 201, description = "Repository connection created", body = tl_core::GitHubConnectionSummary)),
)]
pub async fn create_connection(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Json(input): Json<GitHubConnectionCreateRequest>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "create GitHub repository connections",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let root_path = match normalize_root_path(&input.root_path) {
        Ok(value) => value,
        Err(message) => return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message),
    };
    if let Err(error) = state.agent_store.get(&workspace_id, &input.agent_id).await {
        tracing::info!(error = %error, "github connection agent validation failed");
        return api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "agent not found",
        );
    }
    if let Err(error) = state
        .environment_store
        .get(&workspace_id, &input.environment_id)
        .await
    {
        tracing::info!(error = %error, "github connection environment validation failed");
        return api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "environment not found",
        );
    }
    let installation = match state.store.active_installation(&workspace_id).await {
        Ok(installation) => installation,
        Err(error) => return store_error_response(error),
    };
    let installation_uuid = match Uuid::parse_str(&installation.id) {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                ApiErrorCode::Unavailable,
                "stored GitHub installation is invalid",
            )
        }
    };
    let repository_id = match input.repository_id.parse::<i64>() {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "repository_id must be a string integer",
            )
        }
    };
    let Some(github) = state.github.clone() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "GitHub integration is not configured",
        );
    };
    let github_installation_id = match installation.installation_id.parse::<i64>() {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                ApiErrorCode::Unavailable,
                "stored GitHub installation is invalid",
            )
        }
    };
    let repo = match github.list_repositories(github_installation_id).await {
        Ok(repos) => repos
            .into_iter()
            .find(|repo| repo.repository_id == repository_id),
        Err(error) => return upstream_error(error),
    };
    let Some(repo) = repo else {
        return api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "repository is not available to the GitHub App installation",
        );
    };
    match state
        .store
        .create_connection(
            &workspace_id,
            GitHubConnectionCreate {
                installation_id: installation_uuid,
                repository_id,
                owner: repo.owner,
                name: repo.name,
                default_branch: repo.default_branch,
                root_path,
                agent_id: input.agent_id,
                environment_id: input.environment_id,
            },
        )
        .await
    {
        Ok(connection) => (StatusCode::CREATED, Json(connection)).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub async fn disconnect_connection(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "disconnect GitHub repository connections",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.disconnect_connection(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/github-integration/jobs",
    tag = "github-integration",
    request_body = GitHubIntegrationJobCreateRequest,
    responses((status = 201, description = "Integration analysis job queued", body = GitHubIntegrationJobSummary)),
)]
pub async fn create_job(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Json(input): Json<GitHubIntegrationJobCreateRequest>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "analyze GitHub repository connections",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !input.source_processing_consent {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "source_processing_consent is required",
        );
    }
    let risk_statement = match validate_risk_statement(&input.risk_statement) {
        Ok(value) => value,
        Err(message) => return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message),
    };
    if state.github.is_none() || state.llm.is_none() || state.worker_tx.is_none() {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "GitHub integration analysis is not configured",
        );
    }
    let connection = match state
        .store
        .get_connection(&workspace_id, &input.connection_id)
        .await
    {
        Ok(connection) => connection,
        Err(error) => return store_error_response(error),
    };
    let connection_id = match Uuid::parse_str(&connection.id) {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "invalid connection_id",
            )
        }
    };
    match state
        .store
        .create_job(
            &workspace_id,
            GitHubJobCreate {
                connection_id,
                risk_statement,
                base_branch: connection.default_branch,
                base_sha: None,
                installation_connected_at: None,
                repository_connected_at: Some(chrono::Utc::now()),
            },
        )
        .await
    {
        Ok(job) => {
            if let Some(tx) = state.worker_tx.clone() {
                if tx
                    .try_send(GitHubIntegrationMessage::Analyze {
                        workspace_id,
                        job_id: job.id.clone(),
                    })
                    .is_err()
                {
                    return api_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        ApiErrorCode::Unavailable,
                        "GitHub integration worker queue is full",
                    );
                }
            }
            (StatusCode::CREATED, Json(job)).into_response()
        }
        Err(error) => store_error_response(error),
    }
}

pub async fn get_job(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "read GitHub integration jobs",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    match maybe_verify_job(&state, &workspace_id, &id).await {
        Ok(job) => Json(job).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub async fn approve_job(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "open GitHub integration draft PRs",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(tx) = state.worker_tx.clone() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "GitHub integration worker is unavailable",
        );
    };
    let job = match state.store.get_job(&workspace_id, &id).await {
        Ok(job) => job,
        Err(error) => return store_error_response(error),
    };
    if matches!(
        job.status,
        GitHubIntegrationJobStatus::AwaitingApproval | GitHubIntegrationJobStatus::Applying
    ) {
        let _ = tx.try_send(GitHubIntegrationMessage::Apply {
            workspace_id,
            job_id: id,
        });
    }
    Json(GitHubIntegrationApproveResponse { job }).into_response()
}

pub async fn cancel_job(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "cancel GitHub integration jobs",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .transition_job(
            &workspace_id,
            &id,
            &[
                GitHubIntegrationJobStatus::Queued,
                GitHubIntegrationJobStatus::Analyzing,
                GitHubIntegrationJobStatus::AwaitingApproval,
            ],
            GitHubIntegrationJobStatus::Cancelled,
            GitHubJobUpdate {
                clear_proposed_changes: true,
                ..GitHubJobUpdate::default()
            },
        )
        .await
    {
        Ok(job) => Json(GitHubIntegrationCancelResponse { job }).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn maybe_verify_job(
    state: &GitHubIntegrationState,
    workspace_id: &str,
    job_id: &str,
) -> Result<GitHubIntegrationJobSummary, super::GitHubIntegrationStoreError> {
    let job = state.store.get_job(workspace_id, job_id).await?;
    if job.status != GitHubIntegrationJobStatus::AwaitingVerification {
        return Ok(job);
    }
    let Some(merged_at) = job
        .pr_merged_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&chrono::Utc))
    else {
        return Ok(job);
    };
    let connection = state
        .store
        .get_connection(workspace_id, &job.connection_id)
        .await?;
    match state
        .trace_store
        .find_github_integration_marker(
            workspace_id,
            &connection.environment_id,
            &connection.agent_id,
            &connection.id,
            merged_at,
        )
        .await
    {
        Ok(Some(trace)) => {
            state
                .store
                .transition_job(
                    workspace_id,
                    job_id,
                    &[GitHubIntegrationJobStatus::AwaitingVerification],
                    GitHubIntegrationJobStatus::Verified,
                    GitHubJobUpdate {
                        first_verified_trace_at: Some(
                            chrono::DateTime::parse_from_rfc3339(&trace.created_at)
                                .map(|value| value.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now()),
                        ),
                        ..GitHubJobUpdate::default()
                    },
                )
                .await
        }
        Ok(None) => Ok(job),
        Err(error) => Err(super::GitHubIntegrationStoreError::Internal(
            error.to_string(),
        )),
    }
}

fn request_user_id(
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
) -> Result<Uuid, Box<Response>> {
    if let Some(Extension(user)) = user {
        return Ok(user.user_id);
    }
    if internal.is_some() {
        if let Some(value) = headers
            .get("x-tlg-user-id")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Uuid::parse_str(value).ok())
        {
            return Ok(value);
        }
    }
    Err(Box::new(api_error(
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::Unauthorized,
        "authenticated user is required to connect GitHub repositories",
    )))
}

fn random_state() -> String {
    use base64::Engine as _;
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn read_query(query: Option<&str>, key: &str) -> Option<String> {
    query.and_then(|query| {
        url::form_urlencoded::parse(query.as_bytes()).find_map(|(name, value)| {
            if name == key && !value.trim().is_empty() {
                Some(value.into_owned())
            } else {
                None
            }
        })
    })
}

fn api_error(status: StatusCode, code: ApiErrorCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(ApiError {
            code,
            message: message.into(),
            retriable: status.is_server_error(),
            details: serde_json::Value::Null,
        }),
    )
        .into_response()
}

fn upstream_error(error: GitHubClientError) -> Response {
    match error {
        GitHubClientError::Auth => api_error(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "GitHub authorization failed",
        ),
        GitHubClientError::NotFound => api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "GitHub resource not found",
        ),
        GitHubClientError::Conflict => api_error(
            StatusCode::CONFLICT,
            ApiErrorCode::Invalid,
            "GitHub repository changed; rerun analysis",
        ),
        GitHubClientError::Status { status: 429 } => api_error(
            StatusCode::TOO_MANY_REQUESTS,
            ApiErrorCode::RateLimited,
            "GitHub rate limit exceeded",
        ),
        _ => api_error(
            StatusCode::BAD_GATEWAY,
            ApiErrorCode::Unavailable,
            "GitHub upstream request failed",
        ),
    }
}
