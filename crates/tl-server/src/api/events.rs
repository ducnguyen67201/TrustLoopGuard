//! Direct `GuardEvent` ingestion endpoint.

use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{AuthorizationDecision, GuardEvent};

use crate::{
    auth::WorkspaceKeyContext, environments, services::event_service::execute_event_submission,
    AppState,
};

#[utoipa::path(
    post,
    path = "/v1/events",
    tag = "events",
    request_body = GuardEvent,
    responses(
        (status = 200, description = "Event evaluated; authorization decision returned", body = AuthorizationDecision),
        (status = 400, description = "Malformed request or workspace data handling mode rejects raw events", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Referenced run not found", body = ApiError),
        (status = 422, description = "Event failed validation limits", body = ApiError),
        (status = 500, description = "Workspace resolution failed", body = ApiError),
    ),
)]
pub async fn submit_event(
    State(state): State<AppState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(mut event): Json<GuardEvent>,
) -> Response {
    let start = std::time::Instant::now();
    let workspace_id = match workspace_id_for_event(&headers, &event) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    if let Some(Extension(key)) = runtime_key {
        if let Some(principal_id) = key.principal_id {
            event.principal.agent_id = principal_id;
        }
    }
    match execute_event_submission(&state, &workspace_id, &environment_id, event, start).await {
        Ok(result) => Json(result.authorization).into_response(),
        Err(response) => response,
    }
}

/// Header wins over the caller-claimed principal; the pipeline then
/// overwrites the principal with the server-resolved values regardless,
/// so the claimed workspace can never survive into evidence.
fn workspace_id_for_event(headers: &HeaderMap, _event: &GuardEvent) -> Result<String, Response> {
    crate::policies::workspace_id_from_headers(headers)
}
