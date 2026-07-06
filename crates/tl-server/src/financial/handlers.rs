use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{CreateFinancialActionRequest, FinancialActionRecord, FinancialActionStatus};

use super::{
    response::financial_error_response, validation::validate_create_action, FinancialState,
};

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
    if let Err(error) = validate_create_action(&input) {
        return financial_error_response(error);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.create_action(&workspace_id, input).await {
        Ok(action) => (StatusCode::CREATED, Json(action)).into_response(),
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
    match state.store.get_action(&workspace_id, &id).await {
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
    transition(
        state,
        headers,
        id,
        FinancialActionStatus::Authorized,
        "approved",
    )
    .await
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
    transition(state, headers, id, FinancialActionStatus::Denied, "denied").await
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
    transition(
        state,
        headers,
        id,
        FinancialActionStatus::Executed,
        "executed",
    )
    .await
}

async fn transition(
    state: FinancialState,
    headers: HeaderMap,
    action_id: String,
    status: FinancialActionStatus,
    event_type: &str,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .store
        .transition_action(&workspace_id, &action_id, status, event_type)
        .await
    {
        Ok(action) => Json(action).into_response(),
        Err(error) => financial_error_response(error),
    }
}
