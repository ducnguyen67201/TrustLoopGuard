use axum::{http::HeaderMap, response::Response};
use tl_core::DEFAULT_WORKSPACE_ID;

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

pub(crate) fn workspace_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-tlg-workspace-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_WORKSPACE_ID)
        .to_string()
}
