//! Direct `GuardEvent` ingestion endpoint (observe-only).

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
#[allow(unused_imports)]
use tl_core::Decision;
use tl_core::{GuardEvent, DEFAULT_WORKSPACE_ID};

use crate::{environments, services::event_service::execute_event_submission, AppState};

#[utoipa::path(
    post,
    path = "/v1/events",
    tag = "events",
    request_body = GuardEvent,
    responses(
        (status = 200, description = "Event recorded; observe-only decision returned (verdict is always allow until checker phases ship)", body = Decision),
        (status = 400, description = "Malformed request or workspace data handling mode rejects raw events", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Referenced run not found", body = ApiError),
        (status = 422, description = "Event failed validation limits", body = ApiError),
        (status = 500, description = "Workspace resolution failed", body = ApiError),
    ),
)]
pub async fn submit_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(event): Json<GuardEvent>,
) -> Response {
    let start = std::time::Instant::now();
    let workspace_id = workspace_id_for_event(&headers, &event);
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
    match execute_event_submission(&state, &workspace_id, &environment_id, event, start).await {
        Ok(decision) => Json(decision).into_response(),
        Err(response) => response,
    }
}

/// Header wins over the caller-claimed principal; the pipeline then
/// overwrites the principal with the server-resolved values regardless,
/// so the claimed workspace can never survive into evidence.
fn workspace_id_for_event(headers: &HeaderMap, event: &GuardEvent) -> String {
    headers
        .get("x-tlg-workspace-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            let claimed = event.principal.workspace_id.trim();
            (!claimed.is_empty()).then(|| claimed.to_string())
        })
        .unwrap_or_else(|| DEFAULT_WORKSPACE_ID.to_string())
}
