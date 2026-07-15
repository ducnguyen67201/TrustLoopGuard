use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentRollbackRequest,
    CreateFinancialActionRequest, CreateFinancialPolicyRequest, FinancialActionListResponse,
    FinancialActionOutcome, FinancialActionRecord, FinancialOutcomeListResponse,
    FinancialPolicyListResponse, FinancialPolicyRecord, FinancialReceipt, DEFAULT_ENVIRONMENT_ID,
};

use super::{response::financial_error_response, FinancialState};
use crate::auth::WorkspaceKeyContext;

fn scope(headers: &HeaderMap) -> Result<(String, String), Response> {
    let workspace_id = crate::policies::workspace_id_from_headers(headers)?;
    Ok((
        workspace_id,
        crate::environments::environment_id_from_headers(headers)
            .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string()),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions",
    tag = "financial",
    request_body = CreateFinancialActionRequest,
    responses(
        (status = 201, description = "Financial action created and authorized", body = FinancialActionRecord),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn create_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Json(input): Json<CreateFinancialActionRequest>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
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
    responses((status = 200, description = "Financial actions", body = FinancialActionListResponse)),
)]
pub async fn list_actions(State(state): State<FinancialState>, headers: HeaderMap) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .list_actions(&workspace_id, Some(&environment_id))
        .await
    {
        Ok(actions) => Json(actions).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/actions/{id}",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    request_body = tl_core::ExecuteFinancialActionRequest,
    responses((status = 200, description = "Financial action", body = FinancialActionRecord)),
)]
pub async fn get_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .get_action(&workspace_id, &environment_id, &id)
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
    responses((status = 200, description = "Financial action execution state", body = FinancialActionRecord)),
)]
pub async fn execute_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<tl_core::ExecuteFinancialActionRequest>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .execute_action(&workspace_id, &environment_id, &id, input)
        .await
    {
        Ok(action) => Json(action).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/agentic-payments/authorize",
    tag = "financial",
    request_body = AgenticPaymentAuthorizeRequest,
    responses((status = 201, description = "x402 payment authorization", body = AgenticPaymentAuthorizationResponse)),
)]
pub async fn authorize_agentic_payment(
    State(state): State<FinancialState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(input): Json<AgenticPaymentAuthorizeRequest>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .authorize_agentic_payment_in_environment(
            &workspace_id,
            &environment_id,
            runtime_key.map(|Extension(key)| key),
            input,
        )
        .await
    {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/agentic-payments/{id}",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses((status = 200, description = "x402 payment", body = AgenticPaymentRecord)),
)]
pub async fn get_agentic_payment(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .get_agentic_payment(&workspace_id, &environment_id, &id)
        .await
    {
        Ok(record) => Json(record).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/agentic-payments/{id}/commit",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    request_body = AgenticPaymentCommitRequest,
    responses((status = 200, description = "x402 payment committed", body = AgenticPaymentRecord)),
)]
pub async fn commit_agentic_payment(
    State(state): State<FinancialState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<AgenticPaymentCommitRequest>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .commit_agentic_payment(
            &workspace_id,
            &environment_id,
            &id,
            runtime_key.map(|Extension(key)| key),
            input,
        )
        .await
    {
        Ok(record) => Json(record).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/agentic-payments/{id}/rollback",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    request_body = AgenticPaymentRollbackRequest,
    responses((status = 200, description = "x402 payment reservation released", body = AgenticPaymentRecord)),
)]
pub async fn rollback_agentic_payment(
    State(state): State<FinancialState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<AgenticPaymentRollbackRequest>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .rollback_agentic_payment(
            &workspace_id,
            &environment_id,
            &id,
            runtime_key.map(|Extension(key)| key),
            input,
        )
        .await
    {
        Ok(record) => Json(record).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/agentic-payments/{id}/receipt",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses((status = 200, description = "Financial execution receipt", body = FinancialReceipt)),
)]
pub async fn get_agentic_payment_receipt(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    get_receipt(State(state), headers, Path(id)).await
}

#[utoipa::path(
    get,
    path = "/v1/financial/receipts/{id}",
    tag = "financial",
    params(("id" = String, Path, description = "Financial receipt id")),
    responses((status = 200, description = "Financial execution receipt", body = FinancialReceipt)),
)]
pub async fn get_receipt(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, _) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state.service.get_receipt(&workspace_id, &id).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/policies",
    tag = "financial",
    responses((status = 200, description = "Financial policies", body = FinancialPolicyListResponse)),
)]
pub async fn list_policies(State(state): State<FinancialState>, headers: HeaderMap) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .list_financial_policies(&workspace_id, &environment_id)
        .await
    {
        Ok(policies) => Json(policies).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/policies",
    tag = "financial",
    request_body = CreateFinancialPolicyRequest,
    responses((status = 201, description = "Financial policy created", body = FinancialPolicyRecord)),
)]
pub async fn create_policy(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Json(input): Json<CreateFinancialPolicyRequest>,
) -> Response {
    let (workspace_id, environment_id) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state
        .service
        .create_financial_policy(&workspace_id, &environment_id, input)
        .await
    {
        Ok(policy) => (StatusCode::CREATED, Json(policy)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions/{id}/outcomes",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    request_body = FinancialActionOutcome,
    responses((status = 201, description = "Financial outcome recorded", body = FinancialActionOutcome)),
)]
pub async fn record_action_outcome(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<FinancialActionOutcome>,
) -> Response {
    let (workspace_id, _) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
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
    responses((status = 200, description = "Financial outcomes", body = FinancialOutcomeListResponse)),
)]
pub async fn list_action_outcomes(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, _) = match scope(&headers) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    match state.service.list_action_outcomes(&workspace_id, &id).await {
        Ok(outcomes) => Json(outcomes).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, StatusCode};

    use super::scope;

    #[test]
    fn financial_scope_requires_workspace_header() {
        let response = scope(&HeaderMap::new()).expect_err("workspace is required");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
