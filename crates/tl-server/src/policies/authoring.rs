use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, ApiErrorCode, PolicyBatchSetEnabledRequest, PolicyBatchSetEnabledResponse,
    PolicyDocument, PolicyListResponse, PolicySetEnabledRequest, PolicyValidateResponse,
};

use super::context::{resolve_environment_id, workspace_id_from_headers};
use super::mapping::normalize_policy_ids;
use super::response::{api_error_response, policy_store_error_response};
use super::validation::{parse_policy_body, policy_validation_error_response, validate_raw_policy};
use super::PolicyState;

/// `POST /v1/policies` — create or update a policy from YAML or JSON.
#[utoipa::path(
    post,
    path = "/v1/policies",
    tag = "policies",
    request_body(
        description = "Policy document, YAML or JSON",
        content_type = "application/yaml",
        content = String,
    ),
    responses(
        (status = 201, description = "Policy created or updated", body = PolicyDocument),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 422, description = "Policy failed validation", body = ApiError),
    ),
)]
pub async fn upsert_policy(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let parsed = match parse_policy_body(&headers, &body) {
        Ok(parsed) => parsed,
        Err(resp) => return *resp,
    };
    if let Err(issues) = tl_policy::validate_policy(&parsed.policy) {
        return policy_validation_error_response(&issues);
    }

    match state
        .store
        .upsert(
            &workspace_id,
            &environment_id,
            &parsed.policy,
            &parsed.source_yaml,
        )
        .await
    {
        Ok(document) => (StatusCode::CREATED, Json(document)).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies` — list active policy summaries.
#[utoipa::path(
    get,
    path = "/v1/policies",
    tag = "policies",
    responses(
        (status = 200, description = "All active policies", body = PolicyListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_policies(State(state): State<PolicyState>, headers: HeaderMap) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state.store.list(&workspace_id, &environment_id).await {
        Ok(policies) => Json(PolicyListResponse { policies }).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies/families` — list enabled family policies (payment caps
/// etc.). Read-only: family policies are created via their own surfaces (the
/// pay MCP today), never via `POST /v1/policies`, and are deliberately
/// excluded from the content-policy list above.
#[utoipa::path(
    get,
    path = "/v1/policies/families",
    tag = "policies",
    responses(
        (status = 200, description = "Enabled family policies", body = serde_json::Value),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_family_policies(
    State(state): State<PolicyState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state
        .store
        .list_enabled_families(&workspace_id, &environment_id)
        .await
    {
        Ok(families) => {
            Json(serde_json::json!({ "policies": families.as_slice() })).into_response()
        }
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies/:id` — fetch a policy document.
#[utoipa::path(
    get,
    path = "/v1/policies/{id}",
    tag = "policies",
    params(("id" = String, Path, description = "Policy identifier")),
    responses(
        (status = 200, description = "Policy found", body = PolicyDocument),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Policy not found", body = ApiError),
    ),
)]
pub async fn get_policy(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state.store.get(&workspace_id, &environment_id, &id).await {
        Ok(document) => Json(document).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `PATCH /v1/policies/:id/enabled` — enable or disable a policy.
#[utoipa::path(
    patch,
    path = "/v1/policies/{id}/enabled",
    tag = "policies",
    request_body = PolicySetEnabledRequest,
    params(("id" = String, Path, description = "Policy identifier")),
    responses(
        (status = 200, description = "Updated policy", body = PolicyDocument),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Policy not found", body = ApiError),
    ),
)]
pub async fn set_policy_enabled(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: bytes::Bytes,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let req: PolicySetEnabledRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    match state
        .store
        .set_enabled(&workspace_id, &environment_id, &id, req.enabled)
        .await
    {
        Ok(document) => Json(document).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `PATCH /v1/policies/batch/enabled` — enable or disable multiple policies.
#[utoipa::path(
    patch,
    path = "/v1/policies/batch/enabled",
    tag = "policies",
    request_body = PolicyBatchSetEnabledRequest,
    responses(
        (status = 200, description = "Updated policies", body = PolicyBatchSetEnabledResponse),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "One or more policies were not found", body = ApiError),
    ),
)]
pub async fn batch_set_policy_enabled(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let req: PolicyBatchSetEnabledRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    let policy_ids = match normalize_policy_ids(req.ids) {
        Ok(ids) => ids,
        Err(message) => {
            return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message);
        }
    };
    match state
        .store
        .batch_set_enabled(&workspace_id, &environment_id, &policy_ids, req.enabled)
        .await
    {
        Ok(policies) => Json(PolicyBatchSetEnabledResponse { policies }).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `DELETE /v1/policies/:id` — soft-delete a policy.
#[utoipa::path(
    delete,
    path = "/v1/policies/{id}",
    tag = "policies",
    params(("id" = String, Path, description = "Policy identifier")),
    responses(
        (status = 204, description = "Policy deleted"),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Policy not found", body = ApiError),
    ),
)]
pub async fn delete_policy(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.delete(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `POST /v1/policies/validate` — validate policy YAML or JSON without saving it.
#[utoipa::path(
    post,
    path = "/v1/policies/validate",
    tag = "policies",
    request_body(
        description = "Policy document, YAML or JSON",
        content_type = "application/yaml",
        content = String,
    ),
    responses(
        (status = 200, description = "Validation result", body = PolicyValidateResponse),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn validate_policy(headers: HeaderMap, body: bytes::Bytes) -> Response {
    let raw = match std::str::from_utf8(&body) {
        Ok(raw) => raw,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("body is not valid UTF-8: {e}"),
            );
        }
    };

    let result: PolicyValidateResponse = validate_raw_policy(&headers, raw);
    Json(result).into_response()
}
