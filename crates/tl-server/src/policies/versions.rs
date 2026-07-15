use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};

use super::context::workspace_id_from_headers;
use super::response::policy_store_error_response;
use super::PolicyState;

/// `GET /v1/policies/:id/versions` - list saved YAML versions newest first.
pub async fn list_policy_versions(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
) -> Response {
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.list_versions(&workspace_id, &policy_id).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies/:id/versions/:version` - fetch one historical version.
pub async fn get_policy_version(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path((policy_id, version)): Path<(String, i32)>,
) -> Response {
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state
        .store
        .get_version(&workspace_id, &policy_id, version)
        .await
    {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}
