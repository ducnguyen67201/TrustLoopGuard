//! Gateway/proxy integration surface.
//!
//! SDK callers receive a `Decision` and handle it in their code. Gateway
//! callers route provider traffic through TrustLoopGuard, so this module
//! resolves dashboard config and applies the decision before returning a
//! provider-compatible response.

use std::sync::{Arc, RwLock};
use std::time::Instant;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use bytes::Bytes;
use reqwest::header;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tl_core::{
    ApiError, ApiErrorCode, Channel, CheckRequest, CreateEnforcementProfileRequest,
    CreateGatewayProviderConnectionRequest, CreateGatewayRouteRequest, Decision,
    EnforcementProfile, EnforcementProfileListResponse, FailMode, GatewayCredentialStatus,
    GatewayInputAction, GatewayOutputAction, GatewayProviderConnection,
    GatewayProviderConnectionListResponse, GatewayProviderKind, GatewayRoute,
    GatewayRouteListResponse, RetentionMode, UpdateEnforcementProfileRequest,
    UpdateGatewayProviderConnectionRequest, UpdateGatewayRouteRequest, Verdict,
};
use url::Url;
use uuid::Uuid;

use crate::policies::workspace_id_from_headers;
use crate::{execute_check_request, AppState};

#[derive(Debug, thiserror::Error)]
pub enum GatewayStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct NewGatewayProviderConnection {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub kind: GatewayProviderKind,
    pub base_url: Option<String>,
    pub default_model: String,
    pub encrypted_api_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderConnectionPatch {
    pub display_name: Option<String>,
    pub base_url: Option<Option<String>>,
    pub default_model: Option<String>,
    pub encrypted_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderConnectionSecret {
    pub connection: GatewayProviderConnection,
    pub encrypted_api_key: String,
}

#[derive(Debug, Clone)]
pub struct NewEnforcementProfile {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub input_action: GatewayInputAction,
    pub output_action: GatewayOutputAction,
    pub fail_mode: FailMode,
    pub retention_mode: RetentionMode,
    pub fallback_message: String,
    pub max_regenerations: u32,
}

#[derive(Debug, Clone, Default)]
pub struct EnforcementProfilePatch {
    pub display_name: Option<String>,
    pub input_action: Option<GatewayInputAction>,
    pub output_action: Option<GatewayOutputAction>,
    pub fail_mode: Option<FailMode>,
    pub retention_mode: Option<RetentionMode>,
    pub fallback_message: Option<String>,
    pub max_regenerations: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct NewGatewayRoute {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub provider_connection_id: String,
    pub agent_id: String,
    pub enforcement_profile_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayRoutePatch {
    pub display_name: Option<String>,
    pub provider_connection_id: Option<String>,
    pub agent_id: Option<String>,
    pub enforcement_profile_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedGatewayRoute {
    pub route: GatewayRoute,
    pub provider_connection: GatewayProviderConnection,
    pub enforcement_profile: EnforcementProfile,
    pub encrypted_api_key: String,
}

#[async_trait]
pub trait GatewayStore: Send + Sync {
    async fn list_provider_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayProviderConnection>, GatewayStoreError>;
    async fn create_provider_connection(
        &self,
        input: NewGatewayProviderConnection,
    ) -> Result<GatewayProviderConnection, GatewayStoreError>;
    async fn update_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
        patch: ProviderConnectionPatch,
    ) -> Result<GatewayProviderConnection, GatewayStoreError>;
    async fn get_provider_connection_secret(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<ProviderConnectionSecret, GatewayStoreError>;

    async fn list_enforcement_profiles(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<EnforcementProfile>, GatewayStoreError>;
    async fn create_enforcement_profile(
        &self,
        input: NewEnforcementProfile,
    ) -> Result<EnforcementProfile, GatewayStoreError>;
    async fn update_enforcement_profile(
        &self,
        workspace_id: &str,
        id: &str,
        patch: EnforcementProfilePatch,
    ) -> Result<EnforcementProfile, GatewayStoreError>;

    async fn list_gateway_routes(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayRoute>, GatewayStoreError>;
    async fn create_gateway_route(
        &self,
        input: NewGatewayRoute,
    ) -> Result<GatewayRoute, GatewayStoreError>;
    async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: GatewayRoutePatch,
    ) -> Result<GatewayRoute, GatewayStoreError>;
    async fn resolve_gateway_route(
        &self,
        workspace_id: &str,
        route_id: &str,
    ) -> Result<ResolvedGatewayRoute, GatewayStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryGatewayStore {
    provider_connections: RwLock<Vec<MemoryProviderConnection>>,
    enforcement_profiles: RwLock<Vec<MemoryEnforcementProfile>>,
    gateway_routes: RwLock<Vec<MemoryGatewayRoute>>,
}

#[derive(Debug, Clone)]
struct MemoryProviderConnection {
    workspace_id: String,
    connection: GatewayProviderConnection,
    encrypted_api_key: String,
}

#[derive(Debug, Clone)]
struct MemoryEnforcementProfile {
    workspace_id: String,
    profile: EnforcementProfile,
}

#[derive(Debug, Clone)]
struct MemoryGatewayRoute {
    workspace_id: String,
    route: GatewayRoute,
}

impl MemoryGatewayStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl GatewayStore for MemoryGatewayStore {
    async fn list_provider_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayProviderConnection>, GatewayStoreError> {
        let rows = self.provider_connections.read().map_err(lock_error)?;
        Ok(rows
            .iter()
            .filter(|row| row.workspace_id == workspace_id)
            .map(|row| row.connection.clone())
            .collect())
    }

    async fn create_provider_connection(
        &self,
        input: NewGatewayProviderConnection,
    ) -> Result<GatewayProviderConnection, GatewayStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let connection = GatewayProviderConnection {
            id: input.id,
            display_name: input.display_name,
            kind: input.kind,
            base_url: input.base_url,
            default_model: input.default_model,
            credential_status: GatewayCredentialStatus::Configured,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut rows = self.provider_connections.write().map_err(lock_error)?;
        rows.push(MemoryProviderConnection {
            workspace_id: input.workspace_id,
            connection: connection.clone(),
            encrypted_api_key: input.encrypted_api_key,
        });
        Ok(connection)
    }

    async fn update_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
        patch: ProviderConnectionPatch,
    ) -> Result<GatewayProviderConnection, GatewayStoreError> {
        let mut rows = self.provider_connections.write().map_err(lock_error)?;
        let row = rows
            .iter_mut()
            .find(|row| row.workspace_id == workspace_id && row.connection.id == id)
            .ok_or(GatewayStoreError::NotFound)?;
        if let Some(value) = patch.display_name {
            row.connection.display_name = value;
        }
        if let Some(value) = patch.base_url {
            row.connection.base_url = value;
        }
        if let Some(value) = patch.default_model {
            row.connection.default_model = value;
        }
        if let Some(value) = patch.encrypted_api_key {
            row.encrypted_api_key = value;
            row.connection.credential_status = GatewayCredentialStatus::Configured;
        }
        row.connection.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(row.connection.clone())
    }

    async fn get_provider_connection_secret(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<ProviderConnectionSecret, GatewayStoreError> {
        let rows = self.provider_connections.read().map_err(lock_error)?;
        let row = rows
            .iter()
            .find(|row| row.workspace_id == workspace_id && row.connection.id == id)
            .ok_or(GatewayStoreError::NotFound)?;
        Ok(ProviderConnectionSecret {
            connection: row.connection.clone(),
            encrypted_api_key: row.encrypted_api_key.clone(),
        })
    }

    async fn list_enforcement_profiles(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<EnforcementProfile>, GatewayStoreError> {
        let rows = self.enforcement_profiles.read().map_err(lock_error)?;
        Ok(rows
            .iter()
            .filter(|row| row.workspace_id == workspace_id)
            .map(|row| row.profile.clone())
            .collect())
    }

    async fn create_enforcement_profile(
        &self,
        input: NewEnforcementProfile,
    ) -> Result<EnforcementProfile, GatewayStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let profile = EnforcementProfile {
            id: input.id,
            display_name: input.display_name,
            input_action: input.input_action,
            output_action: input.output_action,
            fail_mode: input.fail_mode,
            retention_mode: input.retention_mode,
            fallback_message: input.fallback_message,
            max_regenerations: input.max_regenerations,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut rows = self.enforcement_profiles.write().map_err(lock_error)?;
        rows.push(MemoryEnforcementProfile {
            workspace_id: input.workspace_id,
            profile: profile.clone(),
        });
        Ok(profile)
    }

    async fn update_enforcement_profile(
        &self,
        workspace_id: &str,
        id: &str,
        patch: EnforcementProfilePatch,
    ) -> Result<EnforcementProfile, GatewayStoreError> {
        let mut rows = self.enforcement_profiles.write().map_err(lock_error)?;
        let row = rows
            .iter_mut()
            .find(|row| row.workspace_id == workspace_id && row.profile.id == id)
            .ok_or(GatewayStoreError::NotFound)?;
        if let Some(value) = patch.display_name {
            row.profile.display_name = value;
        }
        if let Some(value) = patch.input_action {
            row.profile.input_action = value;
        }
        if let Some(value) = patch.output_action {
            row.profile.output_action = value;
        }
        if let Some(value) = patch.fail_mode {
            row.profile.fail_mode = value;
        }
        if let Some(value) = patch.retention_mode {
            row.profile.retention_mode = value;
        }
        if let Some(value) = patch.fallback_message {
            row.profile.fallback_message = value;
        }
        if let Some(value) = patch.max_regenerations {
            row.profile.max_regenerations = value;
        }
        row.profile.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(row.profile.clone())
    }

    async fn list_gateway_routes(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayRoute>, GatewayStoreError> {
        let rows = self.gateway_routes.read().map_err(lock_error)?;
        Ok(rows
            .iter()
            .filter(|row| row.workspace_id == workspace_id)
            .map(|row| row.route.clone())
            .collect())
    }

    async fn create_gateway_route(
        &self,
        input: NewGatewayRoute,
    ) -> Result<GatewayRoute, GatewayStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let route = GatewayRoute {
            id: input.id,
            display_name: input.display_name,
            provider_connection_id: input.provider_connection_id,
            agent_id: input.agent_id,
            enforcement_profile_id: input.enforcement_profile_id,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut rows = self.gateway_routes.write().map_err(lock_error)?;
        rows.push(MemoryGatewayRoute {
            workspace_id: input.workspace_id,
            route: route.clone(),
        });
        Ok(route)
    }

    async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: GatewayRoutePatch,
    ) -> Result<GatewayRoute, GatewayStoreError> {
        let mut rows = self.gateway_routes.write().map_err(lock_error)?;
        let row = rows
            .iter_mut()
            .find(|row| row.workspace_id == workspace_id && row.route.id == id)
            .ok_or(GatewayStoreError::NotFound)?;
        if let Some(value) = patch.display_name {
            row.route.display_name = value;
        }
        if let Some(value) = patch.provider_connection_id {
            row.route.provider_connection_id = value;
        }
        if let Some(value) = patch.agent_id {
            row.route.agent_id = value;
        }
        if let Some(value) = patch.enforcement_profile_id {
            row.route.enforcement_profile_id = value;
        }
        row.route.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(row.route.clone())
    }

    async fn resolve_gateway_route(
        &self,
        workspace_id: &str,
        route_id: &str,
    ) -> Result<ResolvedGatewayRoute, GatewayStoreError> {
        let route = {
            let rows = self.gateway_routes.read().map_err(lock_error)?;
            rows.iter()
                .find(|row| row.workspace_id == workspace_id && row.route.id == route_id)
                .map(|row| row.route.clone())
                .ok_or(GatewayStoreError::NotFound)?
        };
        let provider = self
            .get_provider_connection_secret(workspace_id, &route.provider_connection_id)
            .await?;
        let profile = {
            let rows = self.enforcement_profiles.read().map_err(lock_error)?;
            rows.iter()
                .find(|row| {
                    row.workspace_id == workspace_id
                        && row.profile.id == route.enforcement_profile_id
                })
                .map(|row| row.profile.clone())
                .ok_or(GatewayStoreError::NotFound)?
        };
        Ok(ResolvedGatewayRoute {
            route,
            provider_connection: provider.connection,
            enforcement_profile: profile,
            encrypted_api_key: provider.encrypted_api_key,
        })
    }
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> GatewayStoreError {
    GatewayStoreError::Internal("gateway store lock poisoned".into())
}

#[derive(Clone)]
pub struct GatewayState {
    pub app: AppState,
    pub store: Arc<dyn GatewayStore>,
    pub http: reqwest::Client,
    pub seal_key: [u8; 32],
}

fn reject_runtime_key_config_access(
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
) -> Option<Response> {
    runtime_key.map(|_| {
        api_error_response(
            StatusCode::FORBIDDEN,
            "workspace runtime keys cannot manage gateway configuration".into(),
        )
    })
}

/// Derive the AES-256-GCM seal key from env at startup.
///
/// Requires `TL_GATEWAY_CREDENTIAL_KEY`. Falls back to `TL_API_KEY` with a
/// warning for development compatibility. Panics if neither is set unless the
/// explicit `TL_GATEWAY_ALLOW_INSECURE_DEV_KEY` override is enabled.
pub fn build_seal_key() -> [u8; 32] {
    let allow_insecure_dev_key = std::env::var("TL_GATEWAY_ALLOW_INSECURE_DEV_KEY")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    seal_key_material(
        std::env::var("TL_GATEWAY_CREDENTIAL_KEY").ok(),
        std::env::var("TL_API_KEY").ok(),
        allow_insecure_dev_key,
    )
    .unwrap_or_else(|message| panic!("{message}"))
}

fn seal_key_material(
    gateway_secret: Option<String>,
    api_key: Option<String>,
    allow_insecure_dev_key: bool,
) -> Result<[u8; 32], String> {
    if let Some(secret) = gateway_secret.filter(|value| !value.trim().is_empty()) {
        return Ok(Sha256::digest(secret.as_bytes()).into());
    }
    if let Some(secret) = api_key.filter(|value| !value.trim().is_empty()) {
        tracing::warn!(
            "TL_GATEWAY_CREDENTIAL_KEY is not set; \
             falling back to TL_API_KEY for gateway credential encryption. \
             Set TL_GATEWAY_CREDENTIAL_KEY to a dedicated 32-byte secret before deploying."
        );
        return Ok(Sha256::digest(secret.as_bytes()).into());
    }
    if allow_insecure_dev_key {
        tracing::error!(
            "SECURITY: TL_GATEWAY_ALLOW_INSECURE_DEV_KEY enabled. \
             Using an insecure dev-only gateway credential key."
        );
        return Ok(Sha256::digest(b"trustloopguard-local-gateway-key").into());
    }
    Err(
        "TL_GATEWAY_CREDENTIAL_KEY must be set before gateway provider credentials can be sealed"
            .to_string(),
    )
}

#[utoipa::path(
    get,
    path = "/v1/gateway/provider-connections",
    tag = "gateway",
    responses(
        (status = 200, description = "Gateway provider connections", body = GatewayProviderConnectionListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn list_gateway_provider_connections(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.list_provider_connections(&workspace_id).await {
        Ok(provider_connections) => Json(GatewayProviderConnectionListResponse {
            provider_connections,
        })
        .into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/gateway/provider-connections",
    tag = "gateway",
    request_body = CreateGatewayProviderConnectionRequest,
    responses(
        (status = 201, description = "Gateway provider connection created", body = GatewayProviderConnection),
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn create_gateway_provider_connection(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<CreateGatewayProviderConnectionRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    let input = match normalize_provider_connection(&workspace_id, req, &state.seal_key) {
        Ok(input) => input,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    match state.store.create_provider_connection(input).await {
        Ok(connection) => (StatusCode::CREATED, Json(connection)).into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    patch,
    path = "/v1/gateway/provider-connections/{id}",
    tag = "gateway",
    params(("id" = String, Path, description = "Provider connection id")),
    request_body = UpdateGatewayProviderConnectionRequest,
    responses(
        (status = 200, description = "Gateway provider connection updated", body = GatewayProviderConnection),
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
        (status = 404, description = "Provider connection not found", body = ApiError),
    ),
)]
pub async fn patch_gateway_provider_connection(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateGatewayProviderConnectionRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    let patch = match normalize_provider_connection_patch(req, &state.seal_key) {
        Ok(patch) => patch,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .update_provider_connection(&workspace_id, &id, patch)
        .await
    {
        Ok(connection) => Json(connection).into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/enforcement-profiles",
    tag = "gateway",
    responses(
        (status = 200, description = "Enforcement profiles", body = EnforcementProfileListResponse),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn list_enforcement_profiles(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.list_enforcement_profiles(&workspace_id).await {
        Ok(enforcement_profiles) => Json(EnforcementProfileListResponse {
            enforcement_profiles,
        })
        .into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/enforcement-profiles",
    tag = "gateway",
    request_body = CreateEnforcementProfileRequest,
    responses(
        (status = 201, description = "Enforcement profile created", body = EnforcementProfile),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn create_enforcement_profile(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<CreateEnforcementProfileRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    let input = match normalize_enforcement_profile(&workspace_id, req) {
        Ok(input) => input,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    match state.store.create_enforcement_profile(input).await {
        Ok(profile) => (StatusCode::CREATED, Json(profile)).into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    patch,
    path = "/v1/enforcement-profiles/{id}",
    tag = "gateway",
    params(("id" = String, Path, description = "Enforcement profile id")),
    request_body = UpdateEnforcementProfileRequest,
    responses(
        (status = 200, description = "Enforcement profile updated", body = EnforcementProfile),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn patch_enforcement_profile(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateEnforcementProfileRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    let patch = match normalize_enforcement_profile_patch(req) {
        Ok(patch) => patch,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .update_enforcement_profile(&workspace_id, &id, patch)
        .await
    {
        Ok(profile) => Json(profile).into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/gateway/routes",
    tag = "gateway",
    responses(
        (status = 200, description = "Gateway routes", body = GatewayRouteListResponse),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn list_gateway_routes(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.list_gateway_routes(&workspace_id).await {
        Ok(gateway_routes) => Json(GatewayRouteListResponse { gateway_routes }).into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/gateway/routes",
    tag = "gateway",
    request_body = CreateGatewayRouteRequest,
    responses(
        (status = 201, description = "Gateway route created", body = GatewayRoute),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn create_gateway_route(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<CreateGatewayRouteRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    let input = match normalize_gateway_route(&workspace_id, req) {
        Ok(input) => input,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    match state.store.create_gateway_route(input).await {
        Ok(route) => (StatusCode::CREATED, Json(route)).into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    patch,
    path = "/v1/gateway/routes/{id}",
    tag = "gateway",
    params(("id" = String, Path, description = "Gateway route id")),
    request_body = UpdateGatewayRouteRequest,
    responses(
        (status = 200, description = "Gateway route updated", body = GatewayRoute),
        (status = 403, description = "Workspace runtime keys cannot manage gateway configuration", body = ApiError),
    ),
)]
pub async fn patch_gateway_route(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateGatewayRouteRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = workspace_id_from_headers(&headers);
    let patch = match normalize_gateway_route_patch(req) {
        Ok(patch) => patch,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .update_gateway_route(&workspace_id, &id, patch)
        .await
    {
        Ok(route) => Json(route).into_response(),
        Err(error) => gateway_store_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/gateway/{route_id}/openai/chat/completions",
    tag = "gateway",
    params(("route_id" = String, Path, description = "Gateway route id")),
    responses(
        (status = 200, description = "OpenAI-compatible chat completion response"),
        (status = 400, description = "Unsupported or malformed request", body = ApiError),
        (status = 404, description = "Gateway route not found", body = ApiError),
        (status = 502, description = "Provider request failed", body = ApiError),
    ),
)]
pub async fn proxy_openai_chat_completions(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(route_id): Path<String>,
    body: Bytes,
) -> Response {
    proxy_provider_request(
        state,
        headers,
        route_id,
        body,
        GatewayProviderKind::OpenaiCompatible,
        OpenAiCompatibleGatewayProvider,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/v1/gateway/{route_id}/anthropic/v1/messages",
    tag = "gateway",
    params(("route_id" = String, Path, description = "Gateway route id")),
    responses(
        (status = 200, description = "Anthropic messages response"),
        (status = 400, description = "Unsupported or malformed request", body = ApiError),
        (status = 404, description = "Gateway route not found", body = ApiError),
        (status = 502, description = "Provider request failed", body = ApiError),
    ),
)]
pub async fn proxy_anthropic_messages(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(route_id): Path<String>,
    body: Bytes,
) -> Response {
    proxy_provider_request(
        state,
        headers,
        route_id,
        body,
        GatewayProviderKind::Anthropic,
        AnthropicGatewayProvider,
    )
    .await
}

async fn proxy_provider_request<P: GatewayProvider>(
    state: GatewayState,
    headers: HeaderMap,
    route_id: String,
    body: Bytes,
    expected_kind: GatewayProviderKind,
    provider: P,
) -> Response {
    let gateway_request_id = Uuid::now_v7().to_string();
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = crate::environments::environment_id_from_headers(&headers);
    let resolved = match state
        .store
        .resolve_gateway_route(&workspace_id, &route_id)
        .await
    {
        Ok(resolved) => resolved,
        Err(GatewayStoreError::NotFound) => {
            return api_error_response(StatusCode::NOT_FOUND, "gateway route not found".into());
        }
        Err(error) => return gateway_store_error_response(error),
    };

    if resolved.provider_connection.kind != expected_kind {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            "gateway route provider kind does not match endpoint".into(),
        );
    }

    const MAX_BODY_BYTES: usize = 4 * 1024 * 1024; // 4 MB
    if body.len() > MAX_BODY_BYTES {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            format!("request body exceeds maximum size of {MAX_BODY_BYTES} bytes"),
        );
    }

    let mut request = match serde_json::from_slice::<Value>(&body) {
        Ok(value) => value,
        Err(error) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                format!("provider request body must be JSON: {error}"),
            );
        }
    };
    if provider.is_streaming(&request) {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            "streaming gateway requests are not supported yet".into(),
        );
    }

    let provider_api_key = match unseal_provider_key(&resolved.encrypted_api_key, &state.seal_key) {
        Ok(key) => key,
        Err(message) => {
            tracing::error!(
                workspace_id = %workspace_id,
                route_id = %route_id,
                connection_id = %resolved.provider_connection.id,
                "provider credential decryption failed"
            );
            return api_error_response(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
    };

    let input = provider.extract_input(&request);
    let input_decision = match check_gateway_content(
        &state.app,
        &workspace_id,
        &environment_id,
        &resolved,
        "gateway_input_check",
        &input,
        &input,
    )
    .await
    {
        Ok(decision) => decision,
        Err(response) => return response,
    };

    if input_decision.verdict != Verdict::Allow {
        match resolved.enforcement_profile.input_action {
            GatewayInputAction::Allow => {
                tracing::info!(
                    workspace_id = %workspace_id,
                    route_id = %route_id,
                    verdict = ?input_decision.verdict,
                    "input verdict is non-allow but enforcement profile input_action=allow; request proceeds"
                );
            }
            GatewayInputAction::Block => {
                let pid = input_decision
                    .triggered_policies
                    .first()
                    .map(|p| p.id.as_str());
                return blocked_response(
                    provider.safe_response(&request, &resolved.enforcement_profile),
                    "blocked",
                    &input_decision.trace_id,
                    "input",
                    pid,
                );
            }
            GatewayInputAction::Redact => {
                let safe_input = input_decision
                    .safe_output
                    .clone()
                    .unwrap_or_else(|| "[redacted]".to_string());
                provider.apply_input_rewrite(&mut request, &safe_input);
            }
        }
    }

    let provider_response = match provider
        .forward(
            &state.http,
            &resolved.provider_connection,
            &provider_api_key,
            request.clone(),
        )
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return handle_provider_failure(
                &provider,
                &request,
                &resolved.enforcement_profile,
                error,
            );
        }
    };

    let output = provider.extract_output(&provider_response);
    let output_decision = match check_gateway_content(
        &state.app,
        &workspace_id,
        &environment_id,
        &resolved,
        "gateway_output_check",
        &input,
        &output,
    )
    .await
    {
        Ok(decision) => decision,
        Err(response) => return response,
    };

    if output_decision.verdict == Verdict::Allow {
        return Json(provider_response).into_response();
    }

    match resolved.enforcement_profile.output_action {
        GatewayOutputAction::Allow => Json(provider_response).into_response(),
        GatewayOutputAction::Block => {
            let pid = output_decision
                .triggered_policies
                .first()
                .map(|p| p.id.as_str());
            blocked_response(
                provider.safe_response(&request, &resolved.enforcement_profile),
                "blocked",
                &output_decision.trace_id,
                "output",
                pid,
            )
        }
        GatewayOutputAction::Escalate => {
            let pid = output_decision
                .triggered_policies
                .first()
                .map(|p| p.id.as_str());
            blocked_response(
                provider.safe_response(&request, &resolved.enforcement_profile),
                "escalated",
                &output_decision.trace_id,
                "output",
                pid,
            )
        }
        GatewayOutputAction::Rewrite => {
            if let Some(safe_out) = output_decision.safe_output.as_deref() {
                return Json(provider.apply_output_rewrite(provider_response, safe_out))
                    .into_response();
            }

            if resolved.enforcement_profile.max_regenerations > 0 {
                match check_and_maybe_regenerate(
                    &state.app,
                    &provider,
                    &state.http,
                    &resolved.provider_connection,
                    &provider_api_key,
                    &workspace_id,
                    &environment_id,
                    &resolved,
                    request.clone(),
                    provider_response,
                    output_decision,
                    &gateway_request_id,
                )
                .await
                {
                    Ok(clean) => return Json(clean).into_response(),
                    Err(final_decision) => {
                        let pid = final_decision
                            .triggered_policies
                            .first()
                            .map(|p| p.id.as_str());
                        return blocked_response(
                            provider.safe_response(&request, &resolved.enforcement_profile),
                            "blocked",
                            &final_decision.trace_id,
                            "output",
                            pid,
                        );
                    }
                }
            }

            let pid = output_decision
                .triggered_policies
                .first()
                .map(|p| p.id.as_str());
            blocked_response(
                provider.safe_response(&request, &resolved.enforcement_profile),
                "blocked",
                &output_decision.trace_id,
                "output",
                pid,
            )
        }
    }
}

async fn check_gateway_content(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    resolved: &ResolvedGatewayRoute,
    phase: &str,
    input: &str,
    proposed_output: &str,
) -> Result<Decision, Response> {
    let mut context = json!({
        "integration_mode": "gateway",
        "gateway_phase": phase,
        "provider": provider_kind_text(resolved.provider_connection.kind),
        "route_id": resolved.route.id,
        "enforcement_profile_id": resolved.enforcement_profile.id,
        "retention_mode": retention_mode_text(resolved.enforcement_profile.retention_mode),
    });
    if resolved.enforcement_profile.retention_mode == RetentionMode::MetadataOnly {
        context["body_retention"] = json!("omitted");
    }
    let req = CheckRequest {
        workspace_id: Some(workspace_id.to_string()),
        agent_id: resolved.route.agent_id.clone(),
        channel: Channel::Chat,
        input: input.to_string(),
        proposed_output: proposed_output.to_string(),
        domain: Some(phase.to_string()),
        context,
        ..CheckRequest::default()
    };
    execute_check_request(state, workspace_id, environment_id, req, Instant::now()).await
}

fn blocked_response(
    body: Value,
    verdict_label: &'static str,
    trace_id: &str,
    phase: &'static str,
    first_policy_id: Option<&str>,
) -> Response {
    let mut response = Json(body).into_response();
    let h = response.headers_mut();
    h.insert(
        HeaderName::from_static("x-trustloopguard-verdict"),
        HeaderValue::from_static(verdict_label),
    );
    h.insert(
        HeaderName::from_static("x-trustloopguard-trace-id"),
        HeaderValue::from_str(trace_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    h.insert(
        HeaderName::from_static("x-trustloopguard-phase"),
        HeaderValue::from_static(phase),
    );
    if let Some(pid) = first_policy_id {
        if let Ok(v) = HeaderValue::from_str(pid) {
            h.insert(HeaderName::from_static("x-trustloopguard-policy-id"), v);
        }
    }
    response
}

fn append_assistant_turn(request: &mut Value, content: String) {
    if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
        messages.push(json!({ "role": "assistant", "content": content }));
    }
}

#[allow(clippy::too_many_arguments)]
async fn check_and_maybe_regenerate<P: GatewayProvider>(
    app_state: &AppState,
    provider: &P,
    http: &reqwest::Client,
    connection: &GatewayProviderConnection,
    api_key: &str,
    workspace_id: &str,
    environment_id: &str,
    resolved: &ResolvedGatewayRoute,
    initial_request: Value,
    initial_response: Value,
    initial_decision: Decision,
    gateway_request_id: &str,
) -> Result<Value, Decision> {
    let max = resolved.enforcement_profile.max_regenerations as usize;
    let original_input = provider.extract_input(&initial_request);
    let mut req = initial_request;
    let mut last_decision = initial_decision;

    append_assistant_turn(&mut req, provider.extract_output(&initial_response));
    provider.inject_feedback(&mut req, &last_decision.reason);

    for attempt in 1..=max {
        tracing::info!(
            gateway_request_id,
            attempt,
            max,
            trace_id = %last_decision.trace_id,
            "max_regenerations: re-forwarding to provider"
        );

        let retry_resp = match provider
            .forward(http, connection, api_key, req.clone())
            .await
        {
            Ok(r) => r,
            Err(error) => {
                tracing::warn!(
                    gateway_request_id,
                    attempt,
                    error,
                    "regeneration attempt failed at provider"
                );
                break;
            }
        };

        let retry_output = provider.extract_output(&retry_resp);
        let retry_decision = match check_gateway_content(
            app_state,
            workspace_id,
            environment_id,
            resolved,
            "gateway_output_check",
            &original_input,
            &retry_output,
        )
        .await
        {
            Ok(d) => d,
            Err(_) => break,
        };

        if retry_decision.verdict == Verdict::Allow {
            tracing::info!(
                gateway_request_id,
                attempt,
                "max_regenerations: output passed on retry; self-healing succeeded"
            );
            return Ok(retry_resp);
        }

        last_decision = retry_decision;
        if attempt < max {
            append_assistant_turn(&mut req, provider.extract_output(&retry_resp));
            provider.inject_feedback(&mut req, &last_decision.reason);
        }
    }

    tracing::warn!(
        gateway_request_id,
        max,
        "max_regenerations: all attempts exhausted; falling back to safe response"
    );
    Err(last_decision)
}

fn handle_provider_failure<P: GatewayProvider>(
    provider: &P,
    request: &Value,
    profile: &EnforcementProfile,
    error: String,
) -> Response {
    match profile.fail_mode {
        FailMode::Open => {
            tracing::warn!(error = %error, "upstream provider request failed");
            api_error_response(
                StatusCode::BAD_GATEWAY,
                "upstream provider request failed".into(),
            )
        }
        FailMode::Closed => {
            tracing::warn!(error = %error, "provider failure suppressed by fail_mode=closed; returning safe response");
            blocked_response(
                provider.safe_response(request, profile),
                "blocked",
                &Uuid::now_v7().to_string(),
                "output",
                None,
            )
        }
    }
}

#[async_trait]
trait GatewayProvider: Send + Sync {
    fn is_streaming(&self, request: &Value) -> bool {
        request
            .get("stream")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    fn extract_input(&self, request: &Value) -> String {
        messages_input_text(request)
    }

    fn extract_output(&self, response: &Value) -> String;

    fn apply_input_rewrite(&self, request: &mut Value, safe_input: &str) {
        if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
            if let Some(last) = messages.iter_mut().rev().find(|message| {
                message
                    .get("role")
                    .and_then(Value::as_str)
                    .map(|role| role == "user")
                    .unwrap_or(false)
            }) {
                last["content"] = json!(safe_input);
            }
        }
    }

    fn inject_feedback(&self, request: &mut Value, reason: &str) {
        if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
            messages.push(json!({
                "role": "system",
                "content": format!(
                    "Your previous response violated policy: {reason}. Please revise to comply."
                )
            }));
        }
    }

    fn apply_output_rewrite(&self, response: Value, safe_output: &str) -> Value;
    fn safe_response(&self, request: &Value, profile: &EnforcementProfile) -> Value;
    async fn forward(
        &self,
        http: &reqwest::Client,
        connection: &GatewayProviderConnection,
        api_key: &str,
        request: Value,
    ) -> Result<Value, String>;
}

struct OpenAiCompatibleGatewayProvider;

#[async_trait]
impl GatewayProvider for OpenAiCompatibleGatewayProvider {
    fn extract_output(&self, response: &Value) -> String {
        response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    fn apply_output_rewrite(&self, mut response: Value, safe_output: &str) -> Value {
        if let Some(content) = response.pointer_mut("/choices/0/message/content") {
            *content = json!(safe_output);
        }
        response
    }

    fn safe_response(&self, request: &Value, profile: &EnforcementProfile) -> Value {
        json!({
            "id": format!("chatcmpl_tlg_{}", Uuid::now_v7()),
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": request.get("model").cloned().unwrap_or_else(|| json!("trustloopguard-gateway")),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": profile.fallback_message,
                },
                "finish_reason": "content_filter",
            }],
            "usage": { "prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0 },
        })
    }

    async fn forward(
        &self,
        http: &reqwest::Client,
        connection: &GatewayProviderConnection,
        api_key: &str,
        mut request: Value,
    ) -> Result<Value, String> {
        if request.get("model").is_none() {
            request["model"] = json!(connection.default_model);
        }
        let url = provider_url(connection, "https://api.openai.com", "/v1/chat/completions");
        let response = http
            .post(url)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("provider request failed: {e}"))?;
        provider_json_response(response).await
    }
}

struct AnthropicGatewayProvider;

#[async_trait]
impl GatewayProvider for AnthropicGatewayProvider {
    fn extract_input(&self, request: &Value) -> String {
        let mut parts = Vec::new();
        if let Some(system) = request.get("system") {
            let system = message_content_text(system);
            if !system.is_empty() {
                parts.push(format!("system: {system}"));
            }
        }
        let messages = messages_input_text(request);
        if !messages.is_empty() {
            parts.push(messages);
        }
        parts.join("\n")
    }

    fn extract_output(&self, response: &Value) -> String {
        response
            .get("content")
            .and_then(Value::as_array)
            .map(|content| {
                content
                    .iter()
                    .filter_map(|item| item.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    fn inject_feedback(&self, request: &mut Value, reason: &str) {
        if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
            messages.push(json!({
                "role": "user",
                "content": format!(
                    "Your previous response violated policy: {reason}. Please revise to comply."
                )
            }));
        }
    }

    fn apply_output_rewrite(&self, mut response: Value, safe_output: &str) -> Value {
        if let Some(text) = response.pointer_mut("/content/0/text") {
            *text = json!(safe_output);
        }
        response
    }

    fn safe_response(&self, request: &Value, profile: &EnforcementProfile) -> Value {
        json!({
            "id": format!("msg_tlg_{}", Uuid::now_v7()),
            "type": "message",
            "role": "assistant",
            "model": request.get("model").cloned().unwrap_or_else(|| json!("trustloopguard-gateway")),
            "content": [{ "type": "text", "text": profile.fallback_message }],
            "stop_reason": "content_filter",
            "stop_sequence": null,
            "usage": { "input_tokens": 0, "output_tokens": 0 },
        })
    }

    async fn forward(
        &self,
        http: &reqwest::Client,
        connection: &GatewayProviderConnection,
        api_key: &str,
        mut request: Value,
    ) -> Result<Value, String> {
        if request.get("model").is_none() {
            request["model"] = json!(connection.default_model);
        }
        let url = provider_url(connection, "https://api.anthropic.com", "/v1/messages");
        let response = http
            .post(url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("provider request failed: {e}"))?;
        provider_json_response(response).await
    }
}

fn message_content_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn messages_input_text(request: &Value) -> String {
    request
        .get("messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages
                .iter()
                .filter_map(|message| {
                    let role = message.get("role").and_then(Value::as_str).unwrap_or("");
                    let content = message_content_text(message.get("content")?);
                    Some(format!("{role}: {content}"))
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn provider_url(connection: &GatewayProviderConnection, default_base: &str, path: &str) -> String {
    let base = connection
        .base_url
        .as_deref()
        .unwrap_or(default_base)
        .trim_end_matches('/');
    if base.ends_with(path.trim_start_matches('/')) {
        base.to_string()
    } else {
        format!("{base}{path}")
    }
}

async fn provider_json_response(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(status = status.as_u16(), body = %body, "upstream provider returned error");
        return Err(format!("provider returned status {}", status.as_u16()));
    }
    response
        .json::<Value>()
        .await
        .map_err(|e| format!("provider response must be JSON: {e}"))
}

fn normalize_provider_connection(
    workspace_id: &str,
    req: CreateGatewayProviderConnectionRequest,
    seal_key: &[u8; 32],
) -> Result<NewGatewayProviderConnection, String> {
    let display_name = required_trimmed(req.display_name, "display_name")?;
    let default_model = required_trimmed(req.default_model, "default_model")?;
    let provider_api_key = required_trimmed(req.provider_api_key, "provider_api_key")?;
    Ok(NewGatewayProviderConnection {
        id: req.id.unwrap_or_else(|| format!("gpc_{}", Uuid::now_v7())),
        workspace_id: workspace_id.to_string(),
        display_name,
        kind: req.kind,
        base_url: normalize_optional_url(req.base_url)?,
        default_model,
        encrypted_api_key: seal_provider_key(&provider_api_key, seal_key),
    })
}

fn normalize_provider_connection_patch(
    req: UpdateGatewayProviderConnectionRequest,
    seal_key: &[u8; 32],
) -> Result<ProviderConnectionPatch, String> {
    Ok(ProviderConnectionPatch {
        display_name: normalize_optional_text(req.display_name, "display_name")?,
        base_url: match req.base_url {
            None => None,
            Some(v) => Some(normalize_optional_url(Some(v))?),
        },
        default_model: normalize_optional_text(req.default_model, "default_model")?,
        encrypted_api_key: req
            .provider_api_key
            .map(|value| {
                required_trimmed(value, "provider_api_key")
                    .map(|key| seal_provider_key(&key, seal_key))
            })
            .transpose()?,
    })
}

fn normalize_enforcement_profile(
    workspace_id: &str,
    req: CreateEnforcementProfileRequest,
) -> Result<NewEnforcementProfile, String> {
    Ok(NewEnforcementProfile {
        id: req.id.unwrap_or_else(|| format!("ep_{}", Uuid::now_v7())),
        workspace_id: workspace_id.to_string(),
        display_name: required_trimmed(req.display_name, "display_name")?,
        input_action: req.input_action,
        output_action: req.output_action,
        fail_mode: req.fail_mode,
        retention_mode: req.retention_mode,
        fallback_message: required_trimmed(req.fallback_message, "fallback_message")?,
        max_regenerations: req.max_regenerations,
    })
}

fn normalize_enforcement_profile_patch(
    req: UpdateEnforcementProfileRequest,
) -> Result<EnforcementProfilePatch, String> {
    Ok(EnforcementProfilePatch {
        display_name: normalize_optional_text(req.display_name, "display_name")?,
        input_action: req.input_action,
        output_action: req.output_action,
        fail_mode: req.fail_mode,
        retention_mode: req.retention_mode,
        fallback_message: normalize_optional_text(req.fallback_message, "fallback_message")?,
        max_regenerations: req.max_regenerations,
    })
}

fn normalize_gateway_route(
    workspace_id: &str,
    req: CreateGatewayRouteRequest,
) -> Result<NewGatewayRoute, String> {
    Ok(NewGatewayRoute {
        id: req.id.unwrap_or_else(|| format!("gr_{}", Uuid::now_v7())),
        workspace_id: workspace_id.to_string(),
        display_name: required_trimmed(req.display_name, "display_name")?,
        provider_connection_id: required_trimmed(
            req.provider_connection_id,
            "provider_connection_id",
        )?,
        agent_id: required_trimmed(req.agent_id, "agent_id")?,
        enforcement_profile_id: required_trimmed(
            req.enforcement_profile_id,
            "enforcement_profile_id",
        )?,
    })
}

fn normalize_gateway_route_patch(
    req: UpdateGatewayRouteRequest,
) -> Result<GatewayRoutePatch, String> {
    Ok(GatewayRoutePatch {
        display_name: normalize_optional_text(req.display_name, "display_name")?,
        provider_connection_id: normalize_optional_text(
            req.provider_connection_id,
            "provider_connection_id",
        )?,
        agent_id: normalize_optional_text(req.agent_id, "agent_id")?,
        enforcement_profile_id: normalize_optional_text(
            req.enforcement_profile_id,
            "enforcement_profile_id",
        )?,
    })
}

fn normalize_optional_url(value: Option<String>) -> Result<Option<String>, String> {
    let Some(raw) = value else { return Ok(None) };
    let raw = raw.trim().trim_end_matches('/').to_string();
    if raw.is_empty() {
        return Ok(None);
    }
    let parsed = Url::parse(&raw).map_err(|_| "base_url must be a valid URL".to_string())?;
    match parsed.scheme() {
        "https" | "http" => {}
        scheme => {
            return Err(format!(
                "base_url scheme '{scheme}' is not allowed; use https or http"
            ))
        }
    }
    let host = parsed
        .host()
        .ok_or_else(|| "base_url must have a host".to_string())?;
    match host {
        url::Host::Ipv4(addr) => {
            let [a, b, ..] = addr.octets();
            // Hard-block cloud metadata endpoints (AWS IMDSv1, GCP, Azure).
            if a == 169 && b == 254 {
                return Err(
                    "base_url cannot point to a link-local address (169.254.x.x)".to_string(),
                );
            }
            // Warn for other private ranges — some on-premise deployments are legitimate.
            if a == 127 || a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
            {
                tracing::warn!(
                    base_url = %raw,
                    "SECURITY: provider base_url targets a private network address; \
                     ensure this deployment intentionally routes to an on-premise provider"
                );
            }
        }
        url::Host::Ipv6(addr) => {
            if addr.is_loopback() || addr.is_unspecified() {
                tracing::warn!(
                    base_url = %raw,
                    "SECURITY: provider base_url targets a loopback IPv6 address"
                );
            }
        }
        url::Host::Domain(host) => {
            // Hard-block k8s/mDNS internal domains.
            if host.ends_with(".local")
                || host.ends_with(".internal")
                || host.ends_with(".cluster.local")
            {
                return Err("base_url cannot point to an internal cluster domain".to_string());
            }
            if host == "localhost" || host.ends_with(".localhost") {
                tracing::warn!(
                    base_url = %raw,
                    "SECURITY: provider base_url targets localhost; \
                     ensure this is intentional (local dev or test environment only)"
                );
            }
        }
    }
    Ok(Some(raw))
}

fn normalize_optional_text(value: Option<String>, field: &str) -> Result<Option<String>, String> {
    value
        .map(|value| required_trimmed(value, field))
        .transpose()
}

fn required_trimmed(value: String, field: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(format!("{field} is required"))
    } else {
        Ok(value)
    }
}

fn seal_provider_key(provider_key: &str, seal_key: &[u8; 32]) -> String {
    let unbound = UnboundKey::new(&AES_256_GCM, seal_key).expect("valid AES-256-GCM key");
    let sealing_key = LessSafeKey::new(unbound);
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .expect("system random available for gateway credential sealing");
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut buffer = provider_key.as_bytes().to_vec();
    sealing_key
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut buffer)
        .expect("gateway credential seal succeeds");

    let mut sealed = nonce_bytes.to_vec();
    sealed.extend(buffer);
    format!("tlgw1_{}", URL_SAFE_NO_PAD.encode(sealed))
}

fn unseal_provider_key(ciphertext: &str, seal_key: &[u8; 32]) -> Result<String, String> {
    let encoded = ciphertext
        .strip_prefix("tlgw1_")
        .ok_or_else(|| "provider credential has unsupported seal format".to_string())?;
    let sealed = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| format!("provider credential decode failed: {e}"))?;
    if sealed.len() <= NONCE_LEN {
        return Err("provider credential is truncated".to_string());
    }
    let (nonce_bytes, ciphertext) = sealed.split_at(NONCE_LEN);
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| "provider credential nonce is invalid".to_string())?;
    let unbound = UnboundKey::new(&AES_256_GCM, seal_key)
        .map_err(|_| "gateway credential seal key is invalid".to_string())?;
    let key = LessSafeKey::new(unbound);
    let mut buffer = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut buffer)
        .map_err(|_| "provider credential could not be decrypted".to_string())?;
    String::from_utf8(plaintext.to_vec())
        .map_err(|e| format!("provider credential utf8 decode failed: {e}"))
}

fn gateway_store_error_response(error: GatewayStoreError) -> Response {
    match error {
        GatewayStoreError::NotFound => {
            api_error_response(StatusCode::NOT_FOUND, "gateway resource not found".into())
        }
        GatewayStoreError::Internal(message) => {
            api_error_response(StatusCode::INTERNAL_SERVER_ERROR, message)
        }
    }
}

fn api_error_response(status: StatusCode, message: String) -> Response {
    crate::log_api_error(status, ApiErrorCode::Invalid, &message);
    let code = if status == StatusCode::NOT_FOUND {
        ApiErrorCode::NotFound
    } else if status == StatusCode::UNAUTHORIZED {
        ApiErrorCode::Unauthorized
    } else if status == StatusCode::FORBIDDEN {
        ApiErrorCode::Forbidden
    } else if status == StatusCode::BAD_GATEWAY {
        ApiErrorCode::Unavailable
    } else if status.is_server_error() {
        ApiErrorCode::Internal
    } else {
        ApiErrorCode::Invalid
    };
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    (
        status,
        Json(ApiError {
            code,
            message,
            retriable,
            details: Value::Null,
        }),
    )
        .into_response()
}

fn provider_kind_text(kind: GatewayProviderKind) -> &'static str {
    match kind {
        GatewayProviderKind::OpenaiCompatible => "openai_compatible",
        GatewayProviderKind::Anthropic => "anthropic",
    }
}

fn retention_mode_text(mode: RetentionMode) -> &'static str {
    match mode {
        RetentionMode::MetadataOnly => "metadata_only",
        RetentionMode::RedactedBody => "redacted_body",
        RetentionMode::FullBody => "full_body",
    }
}

pub(crate) fn provider_kind_storage_text(kind: GatewayProviderKind) -> &'static str {
    provider_kind_text(kind)
}

pub(crate) fn input_action_storage_text(action: GatewayInputAction) -> &'static str {
    match action {
        GatewayInputAction::Allow => "allow",
        GatewayInputAction::Block => "block",
        GatewayInputAction::Redact => "redact",
    }
}

pub(crate) fn output_action_storage_text(action: GatewayOutputAction) -> &'static str {
    match action {
        GatewayOutputAction::Allow => "allow",
        GatewayOutputAction::Block => "block",
        GatewayOutputAction::Rewrite => "rewrite",
        GatewayOutputAction::Escalate => "escalate",
    }
}

pub(crate) fn fail_mode_storage_text(mode: FailMode) -> &'static str {
    match mode {
        FailMode::Open => "open",
        FailMode::Closed => "closed",
    }
}

pub(crate) fn retention_mode_storage_text(mode: RetentionMode) -> &'static str {
    retention_mode_text(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_key_config_requires_secret_without_explicit_dev_override() {
        let result = seal_key_material(None, None, false);

        assert!(result.is_err());
    }
}
