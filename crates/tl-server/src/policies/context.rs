use axum::{http::HeaderMap, http::StatusCode, response::Response};
use tl_core::ApiErrorCode;

use super::PolicyState;

pub(super) async fn resolve_environment_id(
    state: &PolicyState,
    headers: &HeaderMap,
    workspace_id: &str,
) -> Result<String, Response> {
    crate::environments::resolve_environment_id(
        headers,
        state.environment_store.as_ref(),
        workspace_id,
    )
    .await
    .map_err(crate::environments::environment_error_response)
}

#[allow(clippy::result_large_err)]
pub(crate) fn workspace_id_from_headers(headers: &HeaderMap) -> Result<String, Response> {
    headers
        .get("x-tlg-workspace-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            crate::app::error::api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "workspace id is required".into(),
            )
        })
}
