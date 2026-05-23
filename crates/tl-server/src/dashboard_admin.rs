//! Dashboard runtime admin endpoints.

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use axum::{
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rand::{rngs::OsRng, RngCore};
use serde_json::json;
use tl_core::{
    ApiError, ApiErrorCode, ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse,
    ApiKeyListResponse, CreateApiKeyRequest, CreateApiKeyResponse, DashboardApiKey, WorkspaceRole,
    WorkspaceSettings,
};
use uuid::Uuid;

use crate::{
    auth::{
        sha256_hex, InternalServiceContext, WorkspaceApiKeyVerifier, WorkspaceApiKeyVerifyError,
        WorkspaceKeyContext,
    },
    jwt::UserContext,
    team::TeamStore,
};

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
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub created_by_user_id: Option<Uuid>,
}

#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get(&self, workspace_id: &str) -> Result<WorkspaceSettings, DashboardAdminStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryApiKeyStore {
    keys: RwLock<Vec<MemoryApiKeyRecord>>,
}

#[derive(Debug, Clone)]
struct MemoryApiKeyRecord {
    id: String,
    workspace_id: String,
    name: String,
    key_prefix: String,
    key_hash: String,
    status: String,
    created_by_user_id: Option<Uuid>,
    created_at: String,
    last_used_at: Option<String>,
    revoked_at: Option<String>,
}

impl MemoryApiKeyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ApiKeyStore for MemoryApiKeyStore {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError> {
        let keys = self
            .keys
            .read()
            .map_err(|_| DashboardAdminStoreError::Internal("api key lock poisoned".into()))?;
        Ok(keys
            .iter()
            .filter(|key| key.workspace_id == workspace_id)
            .map(memory_api_key_to_wire)
            .collect())
    }

    async fn create(&self, input: NewApiKey) -> Result<DashboardApiKey, DashboardAdminStoreError> {
        let record = MemoryApiKeyRecord {
            id: input.id,
            workspace_id: input.workspace_id,
            name: input.name,
            key_prefix: input.key_prefix,
            key_hash: input.key_hash,
            status: "active".to_string(),
            created_by_user_id: input.created_by_user_id,
            created_at: Utc::now().to_rfc3339(),
            last_used_at: None,
            revoked_at: None,
        };
        let wire = memory_api_key_to_wire(&record);
        let mut keys = self
            .keys
            .write()
            .map_err(|_| DashboardAdminStoreError::Internal("api key lock poisoned".into()))?;
        keys.push(record);
        Ok(wire)
    }

    async fn batch_revoke(
        &self,
        workspace_id: &str,
        ids: &[String],
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError> {
        let ids = normalize_ids(ids);
        let mut keys = self
            .keys
            .write()
            .map_err(|_| DashboardAdminStoreError::Internal("api key lock poisoned".into()))?;
        if ids.iter().any(|id| {
            !keys
                .iter()
                .any(|key| key.workspace_id == workspace_id && key.id == *id)
        }) {
            return Err(DashboardAdminStoreError::NotFound);
        }

        let revoked_at = Utc::now().to_rfc3339();
        for key in keys
            .iter_mut()
            .filter(|key| key.workspace_id == workspace_id && ids.iter().any(|id| id == &key.id))
        {
            key.status = "revoked".to_string();
            key.revoked_at = Some(revoked_at.clone());
        }

        Ok(keys
            .iter()
            .filter(|key| key.workspace_id == workspace_id && ids.iter().any(|id| id == &key.id))
            .map(memory_api_key_to_wire)
            .collect())
    }
}

#[async_trait]
impl WorkspaceApiKeyVerifier for MemoryApiKeyStore {
    async fn verify_workspace_api_key(
        &self,
        key_hash: &str,
    ) -> Result<Option<WorkspaceKeyContext>, WorkspaceApiKeyVerifyError> {
        let mut keys = self
            .keys
            .write()
            .map_err(|_| WorkspaceApiKeyVerifyError::Internal("api key lock poisoned".into()))?;
        let Some(key) = keys
            .iter_mut()
            .find(|key| key.key_hash == key_hash && key.status == "active")
        else {
            return Ok(None);
        };
        key.last_used_at = Some(Utc::now().to_rfc3339());
        Ok(Some(WorkspaceKeyContext {
            api_key_id: key.id.clone(),
            workspace_id: key.workspace_id.clone(),
        }))
    }
}

#[derive(Debug, Default)]
pub struct MemorySettingsStore;

#[async_trait]
impl SettingsStore for MemorySettingsStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<WorkspaceSettings, DashboardAdminStoreError> {
        Ok(default_settings())
    }
}

#[derive(Clone)]
pub struct DashboardAdminState {
    pub api_key_store: Arc<dyn ApiKeyStore>,
    pub settings_store: Arc<dyn SettingsStore>,
    pub team_store: Arc<dyn TeamStore>,
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
    let plaintext_key = generate_plaintext_key();
    let key_prefix = plaintext_key.chars().take(18).collect::<String>();
    let input = NewApiKey {
        id: format!("apk_{}", Uuid::now_v7()),
        workspace_id,
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

async fn authorize_api_key_management(
    state: &DashboardAdminState,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
) -> Result<(String, Option<Uuid>), Response> {
    if runtime_key.is_some() {
        return Err(api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace runtime keys cannot manage API keys".to_string(),
        ));
    }

    let workspace_id = crate::policies::workspace_id_from_headers(headers);
    let user_id = match user {
        Some(Extension(ctx)) => ctx.user_id,
        None if internal.is_some() => match forwarded_user_id(headers) {
            Some(user_id) => user_id,
            None => {
                return Err(api_error_response(
                    StatusCode::FORBIDDEN,
                    ApiErrorCode::Forbidden,
                    "signed-in user context is required to manage API keys".to_string(),
                ));
            }
        },
        None => {
            return Err(api_error_response(
                StatusCode::UNAUTHORIZED,
                ApiErrorCode::Unauthorized,
                "authenticated user is required to manage API keys".to_string(),
            ));
        }
    };

    require_api_key_admin_role(&state.team_store, &workspace_id, user_id).await?;
    Ok((workspace_id, Some(user_id)))
}

async fn require_api_key_admin_role(
    team_store: &Arc<dyn TeamStore>,
    workspace_id: &str,
    user_id: Uuid,
) -> Result<(), Response> {
    let members = team_store
        .list_members(workspace_id)
        .await
        .map_err(|error| {
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                error.to_string(),
            )
        })?;

    let user_id = user_id.to_string();
    let role = members
        .iter()
        .find(|member| member.user_id == user_id)
        .map(|member| member.role);

    match role {
        Some(WorkspaceRole::Owner | WorkspaceRole::Admin) => Ok(()),
        Some(WorkspaceRole::Editor | WorkspaceRole::Viewer) | None => Err(api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace owner or admin role is required to manage API keys".to_string(),
        )),
    }
}

fn forwarded_user_id(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get("x-tlg-user-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
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

fn normalize_ids(ids: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for id in ids {
        if !normalized.iter().any(|existing: &String| existing == id) {
            normalized.push(id.clone());
        }
    }
    normalized
}

fn memory_api_key_to_wire(row: &MemoryApiKeyRecord) -> DashboardApiKey {
    DashboardApiKey {
        id: row.id.clone(),
        name: row.name.clone(),
        prefix: row.key_prefix.clone(),
        status: row.status.clone(),
        created_at: row.created_at.clone(),
        last_used_at: row.last_used_at.clone(),
        created_by: row.created_by_user_id.map(|value| value.to_string()),
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
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

pub fn default_settings() -> WorkspaceSettings {
    WorkspaceSettings {
        default_action: "allow".to_string(),
        escalation_webhook_url: None,
        telemetry_enabled: true,
        retention_days: "30".to_string(),
        data_handling_mode: tl_core::DataHandlingMode::RawAllowed,
        config: json!({}),
        updated_at: None,
    }
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    crate::log_api_error(status, code, &message);
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: serde_json::Value::Null,
    };
    (status, Json(body)).into_response()
}
