use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use bytes::Bytes;
#[allow(unused_imports)]
use tl_core::{
    ApiError, CreateEnforcementProfileRequest, CreateGatewayProviderConnectionRequest,
    CreateGatewayRouteRequest, EnforcementProfile, EnforcementProfileListResponse,
    GatewayProviderConnection, GatewayProviderConnectionListResponse, GatewayProviderKind,
    GatewayRoute, GatewayRouteListResponse, UpdateEnforcementProfileRequest,
    UpdateGatewayProviderConnectionRequest, UpdateGatewayRouteRequest,
};

use crate::policies::workspace_id_from_headers;
use crate::AppState;

use super::errors::{api_error_response, gateway_store_error_response};
use super::normalization::{
    normalize_enforcement_profile, normalize_enforcement_profile_patch, normalize_gateway_route,
    normalize_gateway_route_patch, normalize_provider_connection,
    normalize_provider_connection_patch,
};
use super::provider::{AnthropicGatewayProvider, OpenAiCompatibleGatewayProvider};
use super::service::proxy_provider_request;
use super::store::GatewayStore;

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
