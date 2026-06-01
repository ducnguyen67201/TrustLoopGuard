use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, CreateEnforcementProfileRequest, EnforcementProfile, EnforcementProfileListResponse,
    UpdateEnforcementProfileRequest,
};

use super::{reject_runtime_key_config_access, GatewayState};
use crate::policies::workspace_id_from_headers;

use crate::gateway::errors::{api_error_response, gateway_store_error_response};
use crate::gateway::normalization::{
    normalize_enforcement_profile, normalize_enforcement_profile_patch,
};

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
