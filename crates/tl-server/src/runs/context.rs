use axum::{http::HeaderMap, response::Response};

use super::RunState;

pub(super) async fn resolve_environment_id(
    state: &RunState,
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
