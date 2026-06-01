use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{CreateHumanReviewEventRequest, HumanReviewEventListResponse};
#[allow(unused_imports)]
use tl_core::{HumanReviewAnalyticsResponse, HumanReviewEvent};

use super::{
    query::{read_filter, read_limit},
    response::review_error_response,
    validation::validate_create_event,
    HumanReviewState,
};

#[utoipa::path(
    post,
    path = "/v1/traces/{trace_id}/review-events",
    tag = "human-review",
    params(("trace_id" = String, Path, description = "Decision trace id")),
    request_body = CreateHumanReviewEventRequest,
    responses(
        (status = 201, description = "Human review event created", body = HumanReviewEvent),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Trace not found", body = ApiError),
    ),
)]
pub async fn create_review_event(
    State(state): State<HumanReviewState>,
    headers: HeaderMap,
    Path(trace_id): Path<String>,
    Json(input): Json<CreateHumanReviewEventRequest>,
) -> Response {
    if let Err(error) = validate_create_event(&input) {
        return review_error_response(error);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let reviewer_id = headers
        .get("x-tlg-user-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    match state
        .store
        .create_event(&workspace_id, &trace_id, input, reviewer_id)
        .await
    {
        Ok(event) => (StatusCode::CREATED, Json(event)).into_response(),
        Err(error) => review_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/traces/{trace_id}/review-events",
    tag = "human-review",
    params(
        ("trace_id" = String, Path, description = "Decision trace id"),
        ("limit" = Option<usize>, Query, description = "Maximum events to return, capped at 100"),
    ),
    responses(
        (status = 200, description = "Human review events", body = HumanReviewEventListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_review_events(
    State(state): State<HumanReviewState>,
    headers: HeaderMap,
    Path(trace_id): Path<String>,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let limit = read_limit(uri.query()).unwrap_or(50).clamp(1, 100);
    match state
        .store
        .list_events(&workspace_id, &trace_id, limit)
        .await
    {
        Ok(review_events) => Json(HumanReviewEventListResponse { review_events }).into_response(),
        Err(error) => review_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/analytics/human-review",
    tag = "human-review",
    params(
        ("agent_id" = Option<String>, Query, description = "Filter by agent id"),
        ("policy_id" = Option<String>, Query, description = "Filter by policy id"),
        ("run_kind" = Option<String>, Query, description = "Filter by run kind"),
        ("workflow_step" = Option<String>, Query, description = "Filter by workflow step"),
    ),
    responses(
        (status = 200, description = "Human review analytics", body = HumanReviewAnalyticsResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn human_review_analytics(
    State(state): State<HumanReviewState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let filter = read_filter(uri.query());
    match state.store.analytics(&workspace_id, filter).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => review_error_response(error),
    }
}
