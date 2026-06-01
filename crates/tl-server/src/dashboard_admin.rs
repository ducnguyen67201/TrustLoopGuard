//! Dashboard runtime admin endpoints.

mod authorization;
mod memory_store;
mod response;
mod settings;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    ApiErrorCode, ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse,
    CreateApiKeyRequest, CreateApiKeyResponse, DashboardApiKey, WorkspaceSettings,
};
use uuid::Uuid;

use crate::environments::EnvironmentStore;
use crate::{
    auth::{sha256_hex, InternalServiceContext, WorkspaceApiKeyVerifier, WorkspaceKeyContext},
    jwt::UserContext,
    team::TeamStore,
};

pub use memory_store::{MemoryApiKeyStore, MemorySettingsStore};
pub use settings::default_settings;

use authorization::authorize_api_key_management;
use response::api_error_response;

#[derive(Debug, thiserror::Error)]
pub enum DashboardAdminStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait ApiKeyStore: WorkspaceApiKeyVerifier + Send + Sync {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError>;

    async fn create(&self, input: NewApiKey) -> Result<DashboardApiKey, DashboardAdminStoreError>;

    async fn batch_revoke(
        &self,
        workspace_id: &str,
        ids: &[String],
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError>;
}

#[derive(Debug, Clone)]
pub struct NewApiKey {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub created_by_user_id: Option<Uuid>,
}

#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get(&self, workspace_id: &str) -> Result<WorkspaceSettings, DashboardAdminStoreError>;
}

#[derive(Clone)]
pub struct DashboardAdminState {
    pub api_key_store: Arc<dyn ApiKeyStore>,
    pub settings_store: Arc<dyn SettingsStore>,
    pub team_store: Arc<dyn TeamStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
}

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
    let (workspace_id, _) =
        match authorize_api_key_management(&state, &headers, user, internal, runtime_key).await {
            Ok(authorized) => authorized,
            Err(response) => return response,
        };
    match state.api_key_store.list(&workspace_id).await {
        Ok(api_keys) => Json(ApiKeyListResponse { api_keys }).into_response(),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
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
    let (workspace_id, created_by_user_id) =
        match authorize_api_key_management(&state, &headers, user, internal, runtime_key).await {
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
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
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
    let (workspace_id, _) =
        match authorize_api_key_management(&state, &headers, user, internal, runtime_key).await {
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
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

fn generate_plaintext_key() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
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
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}
