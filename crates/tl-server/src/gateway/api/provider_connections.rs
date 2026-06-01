use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, CreateGatewayProviderConnectionRequest, GatewayProviderConnection,
    GatewayProviderConnectionListResponse, UpdateGatewayProviderConnectionRequest,
};

use super::{reject_runtime_key_config_access, GatewayState};
use crate::policies::workspace_id_from_headers;

use crate::gateway::errors::{api_error_response, gateway_store_error_response};
use crate::gateway::normalization::{
    normalize_provider_connection, normalize_provider_connection_patch,
};

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
