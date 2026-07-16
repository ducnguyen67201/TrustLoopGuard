use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let parsed = match parse_policy_body(&headers, &body) {
        Ok(parsed) => parsed,
        Err(resp) => return *resp,
    };
    match parsed.policy {
        tl_policy::AnyPolicy::Content(policy) => {
            if let Err(issues) = tl_policy::validate_policy(&policy) {
                return policy_validation_error_response(&issues);
            }
            match state
                .store
                .upsert(&workspace_id, &environment_id, &policy, &parsed.source_yaml)
                .await
            {
                Ok(document) => (StatusCode::CREATED, Json(document)).into_response(),
                Err(e) => policy_store_error_response(e),
            }
        }
        tl_policy::AnyPolicy::Family(policy) => {
            if let Err(issues) = tl_policy::validate_family_policy(&policy) {
                return policy_validation_error_response(&issues);
            }
            match state
                .store
                .upsert_family(&workspace_id, &environment_id, &policy, &parsed.source_yaml)
                .await
            {
                Ok(()) => match state
                    .store
                    .get(&workspace_id, &environment_id, policy.id())
                    .await
                {
                    Ok(document) => (StatusCode::CREATED, Json(document)).into_response(),
                    Err(e) => policy_store_error_response(e),
                },
                Err(e) => policy_store_error_response(e),
            }
        }
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
pub async fn list_policies(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state.store.list(&workspace_id, &environment_id).await {
        Ok(mut policies) => {
            if let Some(family) = read_policy_family(uri.query()) {
                policies.retain(|policy| policy.family == family);
            }
            Json(PolicyListResponse { policies }).into_response()
        }
        Err(e) => policy_store_error_response(e),
    }
}

fn read_policy_family(query: Option<&str>) -> Option<tl_core::PolicyFamily> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| {
            (key == "family")
                .then_some(value)
                .and_then(parse_policy_family)
        })
}

fn parse_policy_family(raw: &str) -> Option<tl_core::PolicyFamily> {
    match raw {
        "content" => Some(tl_core::PolicyFamily::Content),
        "flow" => Some(tl_core::PolicyFamily::Flow),
        "parameter_source" => Some(tl_core::PolicyFamily::ParameterSource),
        "approval" => Some(tl_core::PolicyFamily::Approval),
        "memory" => Some(tl_core::PolicyFamily::Memory),
        "financial" => Some(tl_core::PolicyFamily::Financial),
        "source_label" => Some(tl_core::PolicyFamily::SourceLabel),
        "tool" => Some(tl_core::PolicyFamily::Tool),
        _ => None,
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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
