use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{ApiErrorCode, PolicyDraft, PolicyDraftRequest, PolicyDraftResponse};

use super::draft::{policy_draft_json_schema, POLICY_DRAFT_SYSTEM_PROMPT};
use super::response::api_error_response;
use super::PolicyState;

/// `POST /v1/policies/draft` — LLM-draft a policy skeleton from a natural-
/// language prompt. The server holds the provider key; callers see a
/// strict, typed response. Returns 503 when no LLM is configured.
#[utoipa::path(
    post,
    path = "/v1/policies/draft",
    tag = "policies",
    request_body = PolicyDraftRequest,
    responses(
        (status = 200, description = "Drafted policy", body = PolicyDraftResponse),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 502, description = "LLM provider failed or returned invalid shape", body = ApiError),
        (status = 503, description = "LLM is not configured on this deployment", body = ApiError),
    ),
)]
pub async fn draft_policy(State(state): State<PolicyState>, body: bytes::Bytes) -> Response {
    let req: PolicyDraftRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    let prompt = req.prompt.trim();
    if prompt.len() < 3 {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "prompt is too short".into(),
        );
    }

    let Some(client) = state.draft_llm.clone() else {
        return api_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "policy drafting is not configured on this deployment (no LLM key)".into(),
        );
    };

    let composed = format!("{POLICY_DRAFT_SYSTEM_PROMPT}\nUser request:\n{prompt}");
    let schema = policy_draft_json_schema();
    let out = match client
        .complete(
            &state.draft_model,
            &composed,
            &schema,
            std::time::Duration::from_secs(30),
        )
        .await
    {
        Ok(out) => out,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Unavailable,
                format!("llm provider error: {e}"),
            );
        }
    };

    let mut value = out.json;
    if value.get("rewrite") == Some(&serde_json::Value::Null) {
        if let Some(obj) = value.as_object_mut() {
            obj.remove("rewrite");
        }
    }

    let draft: PolicyDraft = match serde_json::from_value(value) {
        Ok(draft) => draft,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Internal,
                format!("model returned invalid policy shape: {e}"),
            );
        }
    };

    Json(PolicyDraftResponse { draft }).into_response()
}
