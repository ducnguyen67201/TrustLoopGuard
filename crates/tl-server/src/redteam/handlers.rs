use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, CreateReportRequest, JobStatus, RedteamDispatchRequest, RedteamJobDetail,
    RedteamJobListResponse, RedteamJobResultListResponse, RedteamJobSummary, RedteamReportPayload,
    RedteamReportShare,
};

use super::context::resolve_environment_id;
use super::report::build_report;
use super::response::job_error_response;
use super::share::{generate_share_token, NewReportShare};
use super::validation::{clean_optional, validate_dispatch};
use super::{
    DispatchJob, PublicReportState, RedteamJobListFilter, RedteamJobStoreError, RedteamState,
};

/// Default and maximum lifetime of a shareable report link.
const DEFAULT_REPORT_TTL_DAYS: u32 = 30;
const MAX_REPORT_TTL_DAYS: u32 = 90;

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

/// `GET /v1/redteam/jobs/{id}/report` — presentation-ready vulnerability
/// report for a completed job, optionally compared against a second run of the
/// same agent (`?compare={job_id}`).
#[utoipa::path(
    get,
    path = "/v1/redteam/jobs/{id}/report",
    tag = "redteam",
    params(
        ("id" = String, Path, description = "Job id"),
        ("compare" = Option<String>, Query, description = "Second same-agent job id to compare against"),
    ),
    responses(
        (status = 200, description = "Vulnerability report", body = RedteamReportPayload),
        (status = 400, description = "Compare job targets a different agent", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Job not found", body = ApiError),
    ),
)]
pub async fn get_report(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let job = match state.store.get(&workspace_id, &id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    let results = match state.store.list_results(&workspace_id, &id).await {
        Ok(results) => results,
        Err(e) => return job_error_response(e),
    };

    let compare = match read_compare(uri.query()) {
        Some(compare_id) => {
            let compare_job = match state.store.get(&workspace_id, &compare_id).await {
                Ok(job) => job,
                Err(e) => return job_error_response(e),
            };
            if !same_agent(&job, &compare_job) {
                return job_error_response(RedteamJobStoreError::Validation(
                    "compare job must target the same agent".into(),
                ));
            }
            let compare_results = match state.store.list_results(&workspace_id, &compare_id).await {
                Ok(results) => results,
                Err(e) => return job_error_response(e),
            };
            Some((compare_job, compare_results))
        }
        None => None,
    };

    let generated_at = chrono::Utc::now().to_rfc3339();
    let payload = build_report(
        &job,
        &results,
        compare
            .as_ref()
            .map(|(job, results)| (job, results.as_slice())),
        &generated_at,
    );
    Json(payload).into_response()
}

/// `POST /v1/redteam/reports` — mint a shareable link for a completed job
/// (optionally a same-agent comparison run).
#[utoipa::path(
    post,
    path = "/v1/redteam/reports",
    tag = "redteam",
    request_body = CreateReportRequest,
    responses(
        (status = 201, description = "Share minted", body = RedteamReportShare),
        (status = 400, description = "Job not complete or compare job differs", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Job not found", body = ApiError),
    ),
)]
pub async fn create_report(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Json(input): Json<CreateReportRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);

    let job = match state.store.get(&workspace_id, &input.job_id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    if job.status != JobStatus::Complete {
        return job_error_response(RedteamJobStoreError::Validation(
            "job must be complete before it can be shared".into(),
        ));
    }

    let compare_job_id = clean_optional(input.compare_job_id.clone());
    if let Some(compare_id) = compare_job_id.as_deref() {
        let compare_job = match state.store.get(&workspace_id, compare_id).await {
            Ok(job) => job,
            Err(e) => return job_error_response(e),
        };
        if compare_job.status != JobStatus::Complete {
            return job_error_response(RedteamJobStoreError::Validation(
                "compare job must be complete".into(),
            ));
        }
        if !same_agent(&job, &compare_job) {
            return job_error_response(RedteamJobStoreError::Validation(
                "compare job must target the same agent".into(),
            ));
        }
    }

    let ttl_days = input
        .ttl_days
        .unwrap_or(DEFAULT_REPORT_TTL_DAYS)
        .clamp(1, MAX_REPORT_TTL_DAYS);
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(ttl_days as i64)).to_rfc3339();
    let token = generate_share_token();

    let share = match state
        .report_share_store
        .create(NewReportShare {
            token: &token,
            workspace_id: &workspace_id,
            job_id: &input.job_id,
            compare_job_id: compare_job_id.as_deref(),
            expires_at: Some(&expires_at),
        })
        .await
    {
        Ok(share) => share,
        Err(e) => return job_error_response(e),
    };

    let body = RedteamReportShare {
        path: format!("/r/{}", share.token),
        token: share.token,
        job_id: share.job_id,
        compare_job_id: share.compare_job_id,
        created_at: share.created_at,
        expires_at: share.expires_at,
    };
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `GET /v1/redteam/reports/{token}` — public, token-authenticated report.
///
/// Unauthenticated by design: the token is the bearer capability. The job
/// lookup is scoped to the token's stored workspace, never the request, so a
/// token cannot reach another workspace's data.
#[utoipa::path(
    get,
    path = "/v1/redteam/reports/{token}",
    tag = "redteam",
    params(("token" = String, Path, description = "Report share token")),
    responses(
        (status = 200, description = "Vulnerability report", body = RedteamReportPayload),
        (status = 404, description = "Unknown, expired, or revoked token", body = ApiError),
    ),
)]
pub async fn get_public_report(
    State(state): State<PublicReportState>,
    Path(token): Path<String>,
) -> Response {
    let share = match state.report_share_store.get_by_token(&token).await {
        Ok(share) => share,
        Err(e) => return job_error_response(e),
    };
    let workspace_id = share.workspace_id;

    let job = match state.store.get(&workspace_id, &share.job_id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    let results = match state.store.list_results(&workspace_id, &share.job_id).await {
        Ok(results) => results,
        Err(e) => return job_error_response(e),
    };

    let compare = match share.compare_job_id {
        Some(compare_id) => {
            let compare_job = match state.store.get(&workspace_id, &compare_id).await {
                Ok(job) => job,
                Err(e) => return job_error_response(e),
            };
            let compare_results = match state.store.list_results(&workspace_id, &compare_id).await {
                Ok(results) => results,
                Err(e) => return job_error_response(e),
            };
            Some((compare_job, compare_results))
        }
        None => None,
    };

    let generated_at = chrono::Utc::now().to_rfc3339();
    let payload = build_report(
        &job,
        &results,
        compare
            .as_ref()
            .map(|(job, results)| (job, results.as_slice())),
        &generated_at,
    );
    Json(payload).into_response()
}

/// `POST /v1/redteam/reports/{token}/revoke` — revoke a shareable link
/// (matches the `/cancel` action convention; the public read then 404s).
#[utoipa::path(
    post,
    path = "/v1/redteam/reports/{token}/revoke",
    tag = "redteam",
    params(("token" = String, Path, description = "Report share token")),
    responses(
        (status = 204, description = "Share revoked"),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Unknown token", body = ApiError),
    ),
)]
pub async fn revoke_report(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.report_share_store.revoke(&workspace_id, &token).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => job_error_response(e),
    }
}

/// Two jobs target the same agent when they share a non-empty `agent_id`;
/// otherwise fall back to an identical target URL (jobs run before agents were
/// registered carry no `agent_id`).
fn same_agent(a: &RedteamJobSummary, b: &RedteamJobSummary) -> bool {
    match (a.agent_id.as_deref(), b.agent_id.as_deref()) {
        (Some(x), Some(y)) => x == y,
        _ => a.target == b.target,
    }
}

/// Parse the optional `compare` job id from the query string.
fn read_compare(query: Option<&str>) -> Option<String> {
    query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned())
        .find(|(key, _)| key == "compare")
        .and_then(|(_, value)| clean_optional(Some(value)))
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
