use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, CreateGatewayRouteRequest, GatewayRoute, GatewayRouteListResponse,
    UpdateGatewayRouteRequest,
};

use super::{reject_runtime_key_config_access, GatewayState};
use crate::policies::workspace_id_from_headers;

use crate::gateway::errors::{api_error_response, gateway_store_error_response};
use crate::gateway::normalization::{normalize_gateway_route, normalize_gateway_route_patch};

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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
