use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Extension, Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentRollbackRequest,
    CommitFinancialActionRequest, CommitFinancialActionResponse, CreateFinancialActionRequest,
    CreateFinancialExecutionConnectorRequest, CreateFinancialExecutionConnectorResponse,
    CreateFinancialMandateRequest, CreateFinancialObservationReviewRequest,
    CreateFinancialPolicyRequest, FinancialActionDecisionReceipt, FinancialActionListResponse,
    FinancialActionOutcome, FinancialActionRecord, FinancialApprovalRequestListResponse,
    FinancialExecutionConnector, FinancialExecutionConnectorListResponse, FinancialMandate,
    FinancialMandateListResponse, FinancialObservationReview,
    FinancialObservationReviewListResponse, FinancialObservationSummaryResponse,
    FinancialOutcomeListResponse, FinancialPolicyListResponse, FinancialPolicyRecord,
    FinancialReceipt, DEFAULT_ENVIRONMENT_ID,
};

use super::{response::financial_error_response, FinancialState};
use crate::auth::{InternalServiceContext, WorkspaceKeyContext};
use crate::jwt::UserContext;

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
        .create_action_in_environment_mode(
            &workspace_id,
            &environment_id,
            match resolve_financial_mode(&state, &workspace_id, &environment_id).await {
                Ok(mode) => mode,
                Err(response) => return response,
            },
            input,
        )
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
    path = "/v1/financial/agentic-payments/authorize",
    tag = "financial",
    request_body = AgenticPaymentAuthorizeRequest,
    responses(
        (status = 201, description = "x402 agentic payment authorized or held", body = AgenticPaymentAuthorizationResponse),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 409, description = "Reservation conflict or budget exceeded", body = ApiError),
    ),
)]
pub async fn authorize_agentic_payment(
    State(state): State<FinancialState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(input): Json<AgenticPaymentAuthorizeRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = crate::environments::environment_id_from_headers(&headers)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string());
    let runtime_mode = match resolve_financial_mode(&state, &workspace_id, &environment_id).await {
        Ok(mode) => mode,
        Err(response) => return response,
    };
    match state
        .service
        .authorize_agentic_payment_in_environment_mode(
            &workspace_id,
            &environment_id,
            runtime_mode,
            runtime_key.map(|Extension(key)| key),
            input,
        )
        .await
    {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

async fn resolve_financial_mode(
    state: &FinancialState,
    workspace_id: &str,
    environment_id: &str,
) -> Result<tl_core::FinancialRuntimeMode, Response> {
    let settings = state.settings_store.get(workspace_id).await.map_err(|error| {
        tracing::error!(workspace_id, environment_id, error = %error, "financial mode resolution failed");
        financial_error_response(super::FinancialStoreError::Internal(
            "financial mode resolution failed".into(),
        ))
    })?;
    let overrides = state
        .settings_store
        .get_environment_modes(workspace_id, environment_id)
        .await
        .map_err(|error| {
            tracing::error!(workspace_id, environment_id, error = %error, "financial mode resolution failed");
            financial_error_response(super::FinancialStoreError::Internal(
                "financial mode resolution failed".into(),
            ))
        })?;
    Ok(crate::services::effective_financial_mode(
        &settings,
        overrides.as_ref(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/financial/execution-connectors",
    tag = "financial",
    request_body = CreateFinancialExecutionConnectorRequest,
    responses(
        (status = 201, description = "Execution connector created; secret returned once", body = CreateFinancialExecutionConnectorResponse),
        (status = 403, description = "Workspace admin required", body = ApiError),
    ),
)]
pub async fn create_execution_connector(
    State(state): State<FinancialState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(input): Json<CreateFinancialExecutionConnectorRequest>,
) -> Response {
    let (workspace_id, _) = match crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "create financial execution connectors",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state
        .service
        .create_execution_connector(&workspace_id, input)
        .await
    {
        Ok(connector) => (StatusCode::CREATED, Json(connector)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/execution-connectors",
    tag = "financial",
    responses(
        (status = 200, description = "Execution connectors without secrets", body = FinancialExecutionConnectorListResponse),
        (status = 403, description = "Workspace admin required", body = ApiError),
    ),
)]
pub async fn list_execution_connectors(
    State(state): State<FinancialState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let (workspace_id, _) = match crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "list financial execution connectors",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.service.list_execution_connectors(&workspace_id).await {
        Ok(connectors) => Json(connectors).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/execution-connectors/{id}/revoke",
    tag = "financial",
    params(("id" = String, Path, description = "Execution connector id")),
    responses(
        (status = 200, description = "Execution connector revoked", body = FinancialExecutionConnector),
        (status = 403, description = "Workspace admin required", body = ApiError),
    ),
)]
pub async fn revoke_execution_connector(
    State(state): State<FinancialState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let (workspace_id, _) = match crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "revoke financial execution connectors",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state
        .service
        .revoke_execution_connector(&workspace_id, &id)
        .await
    {
        Ok(connector) => Json(connector).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions/{id}/commit",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    request_body = CommitFinancialActionRequest,
    responses(
        (status = 200, description = "External execution committed", body = CommitFinancialActionResponse),
        (status = 400, description = "Invalid execution attestation", body = ApiError),
        (status = 409, description = "Grant or lifecycle conflict", body = ApiError),
    ),
)]
pub async fn commit_external_action(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<CommitFinancialActionRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .service
        .commit_external_action(&workspace_id, &id, input)
        .await
    {
        Ok(committed) => Json(committed).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/observations/summary",
    tag = "financial",
    params(
        ("start" = String, Query, description = "Inclusive RFC3339 start"),
        ("end" = String, Query, description = "Exclusive RFC3339 end")
    ),
    responses((status = 200, description = "Financial observation summary", body = FinancialObservationSummaryResponse)),
)]
pub async fn financial_observation_summary(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = crate::environments::environment_id_from_headers(&headers)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string());
    let query = uri
        .query()
        .map(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect::<std::collections::HashMap<_, _>>()
        })
        .unwrap_or_default();
    let start = match query
        .get("start")
        .ok_or(())
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).map_err(|_| ()))
    {
        Ok(value) => value.with_timezone(&chrono::Utc),
        Err(_) => {
            return financial_error_response(super::FinancialStoreError::Validation(
                "start must be RFC3339".into(),
            ))
        }
    };
    let end = match query
        .get("end")
        .ok_or(())
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).map_err(|_| ()))
    {
        Ok(value) => value.with_timezone(&chrono::Utc),
        Err(_) => {
            return financial_error_response(super::FinancialStoreError::Validation(
                "end must be RFC3339".into(),
            ))
        }
    };
    match state
        .service
        .observation_summary(&workspace_id, &environment_id, start, end)
        .await
    {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/actions/{id}/observation-reviews",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses((status = 200, description = "Observation review history", body = FinancialObservationReviewListResponse)),
)]
pub async fn list_observation_reviews(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .service
        .list_observation_reviews(&workspace_id, &id)
        .await
    {
        Ok(reviews) => Json(reviews).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/actions/{id}/observation-reviews",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    request_body = CreateFinancialObservationReviewRequest,
    responses(
        (status = 201, description = "Observation review recorded", body = FinancialObservationReview),
        (status = 403, description = "Workspace admin required", body = ApiError)
    ),
)]
pub async fn create_observation_review(
    State(state): State<FinancialState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<CreateFinancialObservationReviewRequest>,
) -> Response {
    let (workspace_id, reviewed_by) = match crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "review financial observations",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    let reviewed_by = reviewed_by
        .map(|id| id.to_string())
        .unwrap_or_else(|| "workspace-admin".into());
    match state
        .service
        .create_observation_review(&workspace_id, &id, input.outcome, input.note, &reviewed_by)
        .await
    {
        Ok(review) => (StatusCode::CREATED, Json(review)).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/financial/agentic-payments/{id}",
    tag = "financial",
    params(("id" = String, Path, description = "Canonical financial action id")),
    responses(
        (status = 200, description = "x402 agentic payment record", body = AgenticPaymentRecord),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agentic payment not found", body = ApiError),
    ),
)]
pub async fn get_agentic_payment(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.service.get_agentic_payment(&workspace_id, &id).await {
        Ok(record) => Json(record).into_response(),
        Err(error) => financial_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/financial/agentic-payments/{id}/commit",
    tag = "financial",
    params(("id" = String, Path, description = "Canonical financial action id")),
    request_body = AgenticPaymentCommitRequest,
    responses(
        (status = 200, description = "x402 agentic payment committed", body = AgenticPaymentRecord),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agentic payment not found", body = ApiError),
        (status = 409, description = "Invalid lifecycle transition", body = ApiError),
    ),
)]
pub async fn commit_agentic_payment(
    State(state): State<FinancialState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<AgenticPaymentCommitRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .service
        .commit_agentic_payment(
            &workspace_id,
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
    params(("id" = String, Path, description = "Canonical financial action id")),
    request_body = AgenticPaymentRollbackRequest,
    responses(
        (status = 200, description = "x402 agentic payment reservation released", body = AgenticPaymentRecord),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agentic payment not found", body = ApiError),
        (status = 409, description = "Invalid lifecycle transition", body = ApiError),
    ),
)]
pub async fn rollback_agentic_payment(
    State(state): State<FinancialState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<AgenticPaymentRollbackRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .service
        .rollback_agentic_payment(
            &workspace_id,
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
    params(("id" = String, Path, description = "Canonical financial action id")),
    responses(
        (status = 200, description = "x402 agentic payment receipt", body = FinancialReceipt),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agentic payment receipt not found", body = ApiError),
    ),
)]
pub async fn get_agentic_payment_receipt(
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
    get,
    path = "/v1/financial/policies",
    tag = "financial",
    responses(
        (status = 200, description = "Financial spending controls", body = FinancialPolicyListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_policies(State(state): State<FinancialState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = crate::environments::environment_id_from_headers(&headers)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string());
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
    responses(
        (status = 201, description = "Financial spending control created", body = FinancialPolicyRecord),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn create_policy(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Json(input): Json<CreateFinancialPolicyRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = crate::environments::environment_id_from_headers(&headers)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string());
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
    get,
    path = "/v1/financial/actions/{id}/decision-receipt",
    tag = "financial",
    params(("id" = String, Path, description = "Financial action id")),
    responses(
        (status = 200, description = "Financial action decision receipt", body = FinancialActionDecisionReceipt),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Financial action not found", body = ApiError),
    ),
)]
pub async fn get_decision_receipt(
    State(state): State<FinancialState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = crate::environments::environment_id_from_headers(&headers)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string());
    match state
        .service
        .get_decision_receipt(&workspace_id, &environment_id, &id)
        .await
    {
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
