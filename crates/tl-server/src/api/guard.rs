use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
#[allow(unused_imports)]
use tl_core::Decision;
use tl_core::{ApiErrorCode, CheckRequest, DEFAULT_WORKSPACE_ID};

use crate::{
    app::error::api_error_response, environments, services::guard_service::execute_check_request,
    AppState,
};

#[utoipa::path(
    post,
    path = "/v1/check",
    tag = "guard",
    request_body = CheckRequest,
    responses(
        (status = 200, description = "Decision returned", body = Decision),
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 410, description = "API version no longer served", body = ApiError),
        (status = 429, description = "Rate limited", body = ApiError),
        (status = 500, description = "Runtime policy resolution failed", body = ApiError),
    ),
)]
pub async fn check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut req): Json<CheckRequest>,
) -> Response {
    let check_start = std::time::Instant::now();
    if let Some(info) = req.redaction.as_ref() {
        if let Err(reason) = info.validate() {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("invalid redaction info: {reason}"),
            );
        }
    }
    let workspace_id = workspace_id_for_check(&headers, &req);
    let environment_id = match environments::resolve_environment_id(
        &headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(environment_id) => environment_id,
        Err(error) => return environments::environment_error_response(error),
    };
    req.workspace_id = Some(workspace_id.clone());
    match execute_check_request(&state, &workspace_id, &environment_id, req, check_start).await {
        Ok(decision) => Json(decision).into_response(),
        Err(response) => response,
    }
}

fn workspace_id_for_check(headers: &HeaderMap, req: &CheckRequest) -> String {
    headers
        .get("x-tlg-workspace-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            req.workspace_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| DEFAULT_WORKSPACE_ID.to_string())
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "guard",
    responses((status = 200, description = "Liveness probe", body = String)),
)]
pub async fn health() -> &'static str {
    "ok"
}
