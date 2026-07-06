use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    CreateFinancialActionRequest, FinancialActionListResponse, FinancialActionRecord,
    FinancialApprovalRequestListResponse,
};

use super::{response::financial_error_response, FinancialState};

#[utoipa::path(
    post,
    path = "/v1/financial/actions",
    tag = "financial",
    request_body = CreateFinancialActionRequest,
    responses(
        (status = 201, description = "Financial action created", body = FinancialActionRecord),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn create_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Json(input): Json<CreateFinancialActionRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.create_action(&workspace_id, input).await {
        Ok(action) => (StatusCode::CREATED, Json(action)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/actions",
    tag = "financial",
    responses(
        (status = 200, description = "Financial actions", body = FinancialActionListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_actions(State(state): State<FinancialState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.list_actions(&workspace_id).await {
        Ok(actions) => Json(actions).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/approval-requests",
    tag = "financial",
    responses(
        (status = 200, description = "Financial approval requests", body = FinancialApprovalRequestListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_approval_requests(
    State(state): State<FinancialState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.list_approval_requests(&workspace_id).await {
        Ok(approval_requests) => Json(approval_requests).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/actions/{id}",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses(
        (status = 200, description = "Financial action", body = FinancialActionRecord),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial action not found", body = ApiError),
    ),
)]
pub async fn get_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.get_action(&workspace_id, &id).await {
        Ok(action) => Json(action).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions/{id}/approve",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses(
        (status = 200, description = "Financial action authorized", body = FinancialActionRecord),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial action not found", body = ApiError),
        (status = 409, description = "Invalid status transition", body = ApiError),
    ),
)]
pub async fn approve_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.approve_action(&workspace_id, &id).await {
        Ok(action) => Json(action).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions/{id}/deny",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses(
        (status = 200, description = "Financial action denied", body = FinancialActionRecord),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial action not found", body = ApiError),
        (status = 409, description = "Invalid status transition", body = ApiError),
    ),
)]
pub async fn deny_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.deny_action(&workspace_id, &id).await {
        Ok(action) => Json(action).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions/{id}/execute",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses(
        (status = 200, description = "Financial action executed", body = FinancialActionRecord),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial action not found", body = ApiError),
        (status = 409, description = "Invalid status transition", body = ApiError),
    ),
)]
pub async fn execute_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.execute_action(&workspace_id, &id).await {
        Ok(action) => Json(action).into_response(),
        Err(error) => financial_error_response(error),
    }
}
