use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    CreateFinancialActionRequest, CreateFinancialMandateRequest, FinancialActionListResponse,
    FinancialActionOutcome, FinancialActionRecord, FinancialApprovalRequestListResponse,
    FinancialMandate, FinancialMandateListResponse, FinancialOutcomeListResponse, FinancialReceipt,
    DEFAULT_ENVIRONMENT_ID,
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
    let environment_id = crate::environments::environment_id_from_headers(&headers)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string());
    match state
        .service
        .create_action_in_environment(&workspace_id, &environment_id, input)
        .await
    {
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
    post,
    path = "/v1/financial/mandates",
    tag = "financial",
    request_body = CreateFinancialMandateRequest,
    responses(
        (status = 201, description = "Financial mandate created", body = FinancialMandate),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn create_mandate(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Json(input): Json<CreateFinancialMandateRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.create_mandate(&workspace_id, input).await {
        Ok(mandate) => (StatusCode::CREATED, Json(mandate)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/mandates",
    tag = "financial",
    responses(
        (status = 200, description = "Financial mandates", body = FinancialMandateListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_mandates(State(state): State<FinancialState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.list_mandates(&workspace_id).await {
        Ok(mandates) => Json(mandates).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/mandates/{id}/revoke",
    tag = "financial",
    params(("id" = String, Path, description = "Financial mandate id")),
    responses(
        (status = 200, description = "Financial mandate revoked", body = FinancialMandate),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial mandate not found", body = ApiError),
    ),
)]
pub async fn revoke_mandate(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.revoke_mandate(&workspace_id, &id).await {
        Ok(mandate) => Json(mandate).into_response(),
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
    path = "/v1/financial/receipts/{id}",
    tag = "financial",
    params(("id" = String, Path, description = "Financial receipt id")),
    responses(
        (status = 200, description = "Financial receipt", body = FinancialReceipt),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial receipt not found", body = ApiError),
    ),
)]
pub async fn get_receipt(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.get_receipt(&workspace_id, &id).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions/{id}/outcomes",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    request_body = FinancialActionOutcome,
    responses(
        (status = 201, description = "Financial action outcome recorded", body = FinancialActionOutcome),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial action not found", body = ApiError),
    ),
)]
pub async fn record_action_outcome(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<FinancialActionOutcome>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .service
        .record_action_outcome(&workspace_id, &id, input)
        .await
    {
        Ok(outcome) => (StatusCode::CREATED, Json(outcome)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/actions/{id}/outcomes",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses(
        (status = 200, description = "Financial action outcomes", body = FinancialOutcomeListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial action not found", body = ApiError),
    ),
)]
pub async fn list_action_outcomes(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.list_action_outcomes(&workspace_id, &id).await {
        Ok(outcomes) => Json(outcomes).into_response(),
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
    let actor_id = financial_actor_id_from_headers(&headers);
    match state
        .service
        .approve_action_as(&workspace_id, &id, actor_id.as_deref())
        .await
    {
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
    let actor_id = financial_actor_id_from_headers(&headers);
    match state
        .service
        .deny_action_as(&workspace_id, &id, actor_id.as_deref())
        .await
    {
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

fn financial_actor_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-tlg-user-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
