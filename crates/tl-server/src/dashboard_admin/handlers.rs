use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::Rng;
#[allow(unused_imports)]
use tl_core::{
    ApiError, EnvironmentCheckerModes, UpdateEnvironmentCheckerModesRequest,
    UpdateWorkspaceSettingsRequest, WorkspaceSettings,
};
use tl_core::{
    ApiErrorCode, ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse,
    CreateApiKeyRequest, CreateApiKeyResponse,
};
use uuid::Uuid;

use crate::{
    auth::{sha256_hex, InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
};

use super::{
    authorization::{authorize_api_key_management, authorize_workspace_admin},
    response::api_error_response,
    DashboardAdminState, DashboardAdminStoreError, NewApiKey,
};

/// `GET /v1/api-keys` - list workspace runtime API keys.
#[utoipa::path(
    get,
    path = "/v1/api-keys",
    tag = "api-keys",
    responses(
        (status = 200, description = "Workspace API keys", body = ApiKeyListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Caller cannot manage API keys for this workspace", body = ApiError),
    ),
)]
pub async fn list_api_keys(
    State(state): State<DashboardAdminState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let (workspace_id, _) = match authorize_api_key_management(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.api_key_store.list(&workspace_id).await {
        Ok(api_keys) => Json(ApiKeyListResponse { api_keys }).into_response(),
        Err(e) => {
            tracing::error!(workspace_id = %workspace_id, error = %e, "api key list failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// `POST /v1/api-keys` - create a workspace runtime API key.
#[utoipa::path(
    post,
    path = "/v1/api-keys",
    tag = "api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "Workspace API key created", body = CreateApiKeyResponse),
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Caller cannot manage API keys for this workspace", body = ApiError),
    ),
)]
pub async fn create_api_key(
    State(state): State<DashboardAdminState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<CreateApiKeyRequest>,
) -> Response {
    let name = req.name.trim();
    if name.is_empty() {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "api key name is required".to_string(),
        );
    }
    // Empty/whitespace-only principal is treated as "not bound" rather
    // than rejected, so callers can send the field unconditionally.
    let principal_id = req
        .principal_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(principal_id) = principal_id.as_deref() {
        if principal_id.chars().count() > MAX_PRINCIPAL_ID_CHARS {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("principal_id must be at most {MAX_PRINCIPAL_ID_CHARS} characters"),
            );
        }
    }
    let (workspace_id, created_by_user_id) = match authorize_api_key_management(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    let environment_id = match req
        .environment_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(environment_id) => environment_id.to_string(),
        None => match state
            .environment_store
            .default_environment_id(&workspace_id)
            .await
        {
            Ok(environment_id) => environment_id,
            Err(error) => {
                return api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    error.to_string(),
                );
            }
        },
    };
    let plaintext_key = generate_plaintext_key();
    let key_prefix = plaintext_key.chars().take(18).collect::<String>();
    let input = NewApiKey {
        id: format!("apk_{}", Uuid::now_v7()),
        workspace_id,
        environment_id,
        name: name.to_string(),
        key_prefix,
        key_hash: sha256_hex(plaintext_key.as_bytes()),
        created_by_user_id,
        principal_id,
    };
    match state.api_key_store.create(input).await {
        Ok(api_key) => (
            StatusCode::CREATED,
            Json(CreateApiKeyResponse {
                api_key,
                plaintext_key,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "api key create failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// `PATCH /v1/api-keys/batch/revoke` - revoke workspace runtime API keys.
#[utoipa::path(
    patch,
    path = "/v1/api-keys/batch/revoke",
    tag = "api-keys",
    request_body = ApiKeyBatchRevokeRequest,
    responses(
        (status = 200, description = "Workspace API keys revoked", body = ApiKeyBatchRevokeResponse),
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Caller cannot manage API keys for this workspace", body = ApiError),
        (status = 404, description = "One or more API keys were not found", body = ApiError),
    ),
)]
pub async fn batch_revoke_api_keys(
    State(state): State<DashboardAdminState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<ApiKeyBatchRevokeRequest>,
) -> Response {
    let ids = match normalize_api_key_ids(req.ids) {
        Ok(ids) => ids,
        Err(message) => {
            return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message);
        }
    };
    let (workspace_id, _) = match authorize_api_key_management(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.api_key_store.batch_revoke(&workspace_id, &ids).await {
        Ok(api_keys) => Json(ApiKeyBatchRevokeResponse { api_keys }).into_response(),
        Err(DashboardAdminStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "one or more API keys were not found".to_string(),
        ),
        Err(e) => {
            tracing::error!(workspace_id = %workspace_id, error = %e, "api key revoke failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// `GET /v1/settings` - read workspace runtime settings.
#[utoipa::path(
    get,
    path = "/v1/settings",
    tag = "settings",
    responses(
        (status = 200, description = "Workspace runtime settings", body = WorkspaceSettings),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn get_settings(
    State(state): State<DashboardAdminState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.settings_store.get(&workspace_id).await {
        Ok(settings) => Json(settings).into_response(),
        Err(e) => {
            tracing::error!(workspace_id = %workspace_id, error = %e, "settings read failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// `PATCH /v1/settings` - partially update workspace runtime settings.
/// Absent fields are left unchanged. Settings gate security enforcement
/// (checker modes), so writes require an Owner/Admin user; workspace
/// runtime keys are rejected — a running agent must never be able to
/// weaken the controls that govern it.
#[utoipa::path(
    patch,
    path = "/v1/settings",
    tag = "settings",
    request_body = UpdateWorkspaceSettingsRequest,
    responses(
        (status = 200, description = "Updated workspace runtime settings", body = WorkspaceSettings),
        (status = 401, description = "Missing or invalid credentials", body = ApiError),
        (status = 403, description = "Caller cannot modify settings for this workspace", body = ApiError),
        (status = 422, description = "Malformed request body", body = ApiError),
    ),
)]
pub async fn update_settings(
    State(state): State<DashboardAdminState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<UpdateWorkspaceSettingsRequest>,
) -> Response {
    if let Err(message) = validate_settings_update(&req) {
        return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message);
    }
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "modify workspace settings",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.settings_store.update(&workspace_id, req).await {
        Ok(settings) => Json(settings).into_response(),
        Err(e) => {
            tracing::error!(workspace_id = %workspace_id, error = %e, "settings update failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// `GET /v1/environments/{environment_id}/checker-modes` - read
/// per-environment checker-mode overrides. Fields set to `null` (or an
/// all-empty body) inherit the workspace-level modes.
#[utoipa::path(
    get,
    path = "/v1/environments/{environment_id}/checker-modes",
    tag = "settings",
    params(("environment_id" = String, Path, description = "Environment id")),
    responses(
        (status = 200, description = "Per-environment checker-mode overrides", body = EnvironmentCheckerModes),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn get_environment_checker_modes(
    State(state): State<DashboardAdminState>,
    Path(environment_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .settings_store
        .get_environment_modes(&workspace_id, &environment_id)
        .await
    {
        Ok(modes) => Json(modes.unwrap_or_default()).into_response(),
        Err(e) => {
            tracing::error!(workspace_id = %workspace_id, environment_id = %environment_id, error = %e, "environment checker-mode read failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// `PUT /v1/environments/{environment_id}/checker-modes` - replace
/// per-environment checker-mode overrides. Omitted fields inherit the
/// workspace-level modes. Writes require an Owner/Admin user; workspace
/// runtime keys are rejected like `PATCH /v1/settings`.
#[utoipa::path(
    put,
    path = "/v1/environments/{environment_id}/checker-modes",
    tag = "settings",
    params(("environment_id" = String, Path, description = "Environment id")),
    request_body = UpdateEnvironmentCheckerModesRequest,
    responses(
        (status = 200, description = "Persisted per-environment checker-mode overrides", body = EnvironmentCheckerModes),
        (status = 401, description = "Missing or invalid credentials", body = ApiError),
        (status = 403, description = "Caller cannot modify settings for this workspace", body = ApiError),
        (status = 404, description = "Environment not found", body = ApiError),
        (status = 422, description = "Malformed request body", body = ApiError),
    ),
)]
pub async fn put_environment_checker_modes(
    State(state): State<DashboardAdminState>,
    Path(environment_id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<UpdateEnvironmentCheckerModesRequest>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "modify workspace settings",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state
        .settings_store
        .put_environment_modes(&workspace_id, &environment_id, req.into())
        .await
    {
        Ok(modes) => Json(modes).into_response(),
        Err(DashboardAdminStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "environment was not found in this workspace".to_string(),
        ),
        Err(e) => {
            tracing::error!(workspace_id = %workspace_id, environment_id = %environment_id, error = %e, "environment checker-mode write failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// AuthorizationEffect values `default_action` may take. The field gates no runtime
/// decision yet; validating here keeps stored data meaningful before a
/// consumer appears.
const VALID_DEFAULT_ACTIONS: [&str; 5] =
    ["permit", "deny", "transform", "require_approval", "defer"];
const MAX_RETENTION_DAYS: u32 = 3650;

/// Cap on the free-form principal binding accepted at key creation.
const MAX_PRINCIPAL_ID_CHARS: usize = 256;

fn validate_settings_update(req: &UpdateWorkspaceSettingsRequest) -> Result<(), String> {
    if let Some(default_action) = req.default_action.as_deref() {
        if !VALID_DEFAULT_ACTIONS.contains(&default_action) {
            return Err(format!(
                "default_action must be one of: {}",
                VALID_DEFAULT_ACTIONS.join(", ")
            ));
        }
    }
    if let Some(retention_days) = req.retention_days.as_deref() {
        match retention_days.parse::<u32>() {
            Ok(days) if (1..=MAX_RETENTION_DAYS).contains(&days) => {}
            _ => {
                return Err(format!(
                    "retention_days must be an integer between 1 and {MAX_RETENTION_DAYS}"
                ));
            }
        }
    }
    Ok(())
}

fn generate_plaintext_key() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    format!("tl_live_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn normalize_api_key_ids(ids: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for id in ids {
        let id = id.trim();
        if id.is_empty() {
            return Err("api key ids must not be empty".into());
        }
        if !normalized.iter().any(|existing: &String| existing == id) {
            normalized.push(id.to_string());
        }
    }
    if normalized.is_empty() {
        return Err("at least one API key id is required".into());
    }
    Ok(normalized)
}
