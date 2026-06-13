use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, JobStatus, RedteamAttackRecordListResponse, RedteamDispatchRequest, RedteamJobDetail,
    RedteamJobListResponse, RedteamJobResultListResponse, RedteamJobSummary,
};

use super::context::resolve_environment_id;
use super::response::job_error_response;
use super::validation::{clean_optional, validate_dispatch};
use super::{
    DispatchJob, RedteamAttackRecordFilter, RedteamJobListFilter, RedteamJobStoreError,
    RedteamState,
};

/// `POST /v1/redteam/dispatch` — create a job and hand it to the worker.
#[utoipa::path(
    post,
    path = "/v1/redteam/dispatch",
    tag = "redteam",
    request_body = RedteamDispatchRequest,
    responses(
        (status = 201, description = "Job dispatched", body = RedteamJobSummary),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 503, description = "Dispatch worker unavailable", body = ApiError),
    ),
)]
pub async fn dispatch_job(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Json(input): Json<RedteamDispatchRequest>,
) -> Response {
    if let Err(e) = validate_dispatch(&input) {
        return job_error_response(e);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let Some(dispatch_tx) = state.dispatch_tx.clone() else {
        return job_error_response(RedteamJobStoreError::Unavailable(
            "redteam runner not configured (set REDTEAM_RUNNER_URL)".into(),
        ));
    };
    let job = match state
        .store
        .create(&workspace_id, &environment_id, &input)
        .await
    {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    let message = DispatchJob {
        workspace_id: workspace_id.clone(),
        environment_id,
        job_id: job.id.clone(),
        request: input,
    };
    match dispatch_tx.try_send(message) {
        Ok(()) => (StatusCode::CREATED, Json(job)).into_response(),
        Err(_) => {
            // Queue full or closed — best-effort mark the job `Error` so it isn't
            // stranded in `queued`. Log if even that fails so a stuck job is
            // diagnosable rather than silent.
            if let Err(status_err) = state
                .store
                .set_status(
                    &workspace_id,
                    &job.id,
                    JobStatus::Error,
                    None,
                    Some("dispatch queue unavailable"),
                )
                .await
            {
                tracing::error!(
                    job_id = %job.id,
                    error = %status_err,
                    "redteam: failed to mark job Error after dispatch send failed; job may be stranded"
                );
            }
            job_error_response(RedteamJobStoreError::Unavailable(
                "dispatch queue is full; retry shortly".into(),
            ))
        }
    }
}

/// `GET /v1/redteam/jobs` — list workspace jobs, newest first.
#[utoipa::path(
    get,
    path = "/v1/redteam/jobs",
    tag = "redteam",
    params(
        ("agent_id" = Option<String>, Query, description = "Filter by associated agent id"),
        ("limit" = Option<usize>, Query, description = "Maximum jobs to return, capped at 100"),
    ),
    responses(
        (status = 200, description = "Workspace jobs", body = RedteamJobListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_jobs(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .store
        .list(&workspace_id, read_filter(uri.query()))
        .await
    {
        Ok(jobs) => Json(RedteamJobListResponse { jobs }).into_response(),
        Err(e) => job_error_response(e),
    }
}

/// `GET /v1/redteam/jobs/{id}` — a job plus its per-attack results.
#[utoipa::path(
    get,
    path = "/v1/redteam/jobs/{id}",
    tag = "redteam",
    params(("id" = String, Path, description = "Job id")),
    responses(
        (status = 200, description = "Job detail", body = RedteamJobDetail),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Job not found", body = ApiError),
    ),
)]
pub async fn get_job(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let job = match state.store.get(&workspace_id, &id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    match state.store.list_results(&workspace_id, &id).await {
        Ok(results) => Json(RedteamJobDetail { job, results }).into_response(),
        Err(e) => job_error_response(e),
    }
}

/// `GET /v1/redteam/jobs/{id}/results` — per-attack results only.
#[utoipa::path(
    get,
    path = "/v1/redteam/jobs/{id}/results",
    tag = "redteam",
    params(("id" = String, Path, description = "Job id")),
    responses(
        (status = 200, description = "Job results", body = RedteamJobResultListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Job not found", body = ApiError),
    ),
)]
pub async fn list_results(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list_results(&workspace_id, &id).await {
        Ok(results) => Json(RedteamJobResultListResponse { results }).into_response(),
        Err(e) => job_error_response(e),
    }
}

/// `POST /v1/redteam/jobs/{id}/cancel` — cooperatively cancel a job.
#[utoipa::path(
    post,
    path = "/v1/redteam/jobs/{id}/cancel",
    tag = "redteam",
    params(("id" = String, Path, description = "Job id")),
    responses(
        (status = 200, description = "Job cancelled (or already terminal)", body = RedteamJobSummary),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Job not found", body = ApiError),
    ),
)]
pub async fn cancel_job(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.cancel(&workspace_id, &id).await {
        Ok(job) => Json(job).into_response(),
        Err(e) => job_error_response(e),
    }
}

/// `GET /v1/redteam/attacks` — every attack result in the workspace, newest first.
#[utoipa::path(
    get,
    path = "/v1/redteam/attacks",
    tag = "redteam",
    params(
        ("attack" = Option<String>, Query, description = "Filter by attack name"),
        ("outcome" = Option<String>, Query, description = "Filter by outcome (landed|blocked|clean|error)"),
        ("limit" = Option<usize>, Query, description = "Maximum records to return, capped at 100"),
    ),
    responses(
        (status = 200, description = "Workspace attack records", body = RedteamAttackRecordListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_attack_records(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state
        .store
        .list_attack_records(&workspace_id, read_attack_filter(uri.query()))
        .await
    {
        Ok(records) => Json(RedteamAttackRecordListResponse { records }).into_response(),
        Err(e) => job_error_response(e),
    }
}

/// Parse `agent_id` + `limit` from the query string. Unknown keys ignored;
/// `limit` defaults to 20 and is clamped by the store.
fn read_filter(query: Option<&str>) -> RedteamJobListFilter {
    let mut filter = RedteamJobListFilter {
        limit: 20,
        ..RedteamJobListFilter::default()
    };
    let parts = query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned());
    for (key, value) in parts {
        match key.as_str() {
            "agent_id" => filter.agent_id = clean_optional(Some(value)),
            "limit" => filter.limit = value.parse().unwrap_or(20),
            _ => {}
        }
    }
    filter
}

/// Parse `attack` + `outcome` + `limit` from the query string. Unknown keys
/// ignored; `limit` defaults to 50 and is clamped 1..=100 by the store.
fn read_attack_filter(query: Option<&str>) -> RedteamAttackRecordFilter {
    let mut filter = RedteamAttackRecordFilter {
        limit: 50,
        ..RedteamAttackRecordFilter::default()
    };
    let parts = query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned());
    for (key, value) in parts {
        match key.as_str() {
            "attack" => filter.attack = clean_optional(Some(value)),
            "outcome" => filter.outcome = clean_optional(Some(value)),
            "limit" => filter.limit = value.parse().unwrap_or(50),
            _ => {}
        }
    }
    filter
}
