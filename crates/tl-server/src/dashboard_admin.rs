//! Dashboard runtime admin endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode, ApiKeyListResponse, DashboardApiKey, WorkspaceSettings};

#[derive(Debug, thiserror::Error)]
pub enum DashboardAdminStoreError {
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError>;
}

#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get(&self, workspace_id: &str) -> Result<WorkspaceSettings, DashboardAdminStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryApiKeyStore;

#[async_trait]
impl ApiKeyStore for MemoryApiKeyStore {
    async fn list(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<DashboardApiKey>, DashboardAdminStoreError> {
        Ok(vec![])
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
}

/// `GET /v1/api-keys` - list workspace runtime API keys.
#[utoipa::path(
    get,
    path = "/v1/api-keys",
    tag = "api-keys",
    responses(
        (status = 200, description = "Workspace API keys", body = ApiKeyListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_api_keys(
    State(state): State<DashboardAdminState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.api_key_store.list(&workspace_id).await {
        Ok(api_keys) => Json(ApiKeyListResponse { api_keys }).into_response(),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
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
        config: json!({}),
        updated_at: None,
    }
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
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
