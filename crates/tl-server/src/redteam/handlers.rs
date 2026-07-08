use std::collections::BTreeSet;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, ApiErrorCode, AttackVector, CreateReportRequest, JobStatus,
    RedteamAttackRecordListResponse, RedteamDispatchRequest, RedteamJobDetail,
    RedteamJobListResponse, RedteamJobSummary, RedteamReportPayload, RedteamReportShare,
    RedteamRunMode, RegressionCaseListResponse, RegressionCaseResult, RegressionCaseSummary,
    RegressionExpectedOutcome, RegressionResultStatus, RegressionResultSummaryResponse,
    RegressionResultTrendResponse, RegressionRunRequest, RegressionRunResponse,
};

use super::context::resolve_environment_id;
use super::enrich_report_diagnostics;
use super::report::build_report;
use super::response::job_error_response;
use super::share::{generate_share_token, NewReportShare};
use super::validation::{clean_optional, validate_dispatch};
use super::{
    DispatchJob, NewRegressionResultSnapshot, PublicReportState, RedteamAttackRecordFilter,
    RedteamJobListFilter, RedteamJobStoreError, RedteamRegressionCaseFilter,
    RedteamRegressionResultFilter, RedteamRegressionStoreError, RedteamState,
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
            "red-team execution is not configured for this deployment; contact TrustLoopGuard to enable managed or enterprise execution".into(),
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
        ("limit" = Option<usize>, Query, minimum = 1, description = "Maximum jobs to return, capped at 100"),
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

/// `GET /v1/redteam/jobs/{id}` — a job plus its attack sessions.
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
    match state.store.list_sessions(&workspace_id, &id).await {
        Ok(sessions) => Json(RedteamJobDetail { job, sessions }).into_response(),
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
    let sessions = match state.store.list_sessions(&workspace_id, &id).await {
        Ok(sessions) => sessions,
        Err(e) => return job_error_response(e),
    };

    let compare = match read_compare(uri.query()) {
        Some(compare_id) => {
            if compare_id == id {
                return job_error_response(RedteamJobStoreError::Validation(
                    "cannot compare a job to itself".into(),
                ));
            }
            let compare_job = match state.store.get(&workspace_id, &compare_id).await {
                Ok(job) => job,
                Err(e) => return job_error_response(e),
            };
            if !same_agent(&job, &compare_job) {
                return job_error_response(RedteamJobStoreError::Validation(
                    "compare job must target the same agent".into(),
                ));
            }
            let compare_sessions_result =
                state.store.list_sessions(&workspace_id, &compare_id).await;
            let compare_sessions = match compare_sessions_result {
                Ok(sessions) => sessions,
                Err(e) => return job_error_response(e),
            };
            Some((compare_job, compare_sessions))
        }
        None => None,
    };

    let generated_at = chrono::Utc::now().to_rfc3339();
    let mut payload = build_report(
        &job,
        &sessions,
        compare
            .as_ref()
            .map(|(job, sessions)| (job, sessions.as_slice())),
        &generated_at,
    );
    enrich_report_diagnostics(state.llm.as_ref(), &workspace_id, &mut payload, &sessions).await;
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
        if compare_id == input.job_id {
            return job_error_response(RedteamJobStoreError::Validation(
                "cannot compare a job to itself".into(),
            ));
        }
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
        (status = 429, description = "Too many requests for this link", body = ApiError),
    ),
)]
pub async fn get_public_report(
    State(state): State<PublicReportState>,
    Path(token): Path<String>,
) -> Response {
    // Resolve first (cheap 404 for unknown/expired/revoked), then rate-limit by
    // the *valid* token: keeps the limiter map bounded to live shares, and caps
    // abuse of any single shared link before the more expensive report build.
    let share = match state.report_share_store.get_by_token(&token).await {
        Ok(share) => share,
        Err(e) => return job_error_response(e),
    };
    if !state.rate_limiter.check(&token) {
        return rate_limited_response();
    }
    let workspace_id = share.workspace_id;

    let job = match state.store.get(&workspace_id, &share.job_id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    let sessions = match state
        .store
        .list_sessions(&workspace_id, &share.job_id)
        .await
    {
        Ok(sessions) => sessions,
        Err(e) => return job_error_response(e),
    };

    let compare = match share.compare_job_id {
        Some(compare_id) => {
            let compare_job = match state.store.get(&workspace_id, &compare_id).await {
                Ok(job) => job,
                Err(e) => return job_error_response(e),
            };
            let compare_sessions_result =
                state.store.list_sessions(&workspace_id, &compare_id).await;
            let compare_sessions = match compare_sessions_result {
                Ok(sessions) => sessions,
                Err(e) => return job_error_response(e),
            };
            Some((compare_job, compare_sessions))
        }
        None => None,
    };

    let generated_at = chrono::Utc::now().to_rfc3339();
    let payload = build_report(
        &job,
        &sessions,
        compare
            .as_ref()
            .map(|(job, sessions)| (job, sessions.as_slice())),
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

/// `GET /v1/redteam/attacks` — every attack session in the workspace, newest first.
#[utoipa::path(
    get,
    path = "/v1/redteam/attacks",
    tag = "redteam",
    params(
        ("attack" = Option<String>, Query, description = "Filter by attack name"),
        ("outcome" = Option<String>, Query, description = "Filter by outcome (landed|blocked|clean|error)"),
        ("limit" = Option<usize>, Query, minimum = 1, description = "Maximum records to return, capped at 100"),
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

/// `GET /v1/redteam/regressions` — list durable regression cases promoted from
/// harden survivors, newest first.
#[utoipa::path(
    get,
    path = "/v1/redteam/regressions",
    tag = "redteam",
    params(
        ("agent_id" = Option<String>, Query, description = "Filter by associated agent id"),
        ("source_job_id" = Option<String>, Query, description = "Filter by the source red-team job that promoted the case"),
        ("limit" = Option<usize>, Query, minimum = 1, description = "Maximum cases to return, capped at 100"),
    ),
    responses(
        (status = 200, description = "Workspace regression cases", body = RegressionCaseListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_regression_cases(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let filter = match read_regression_filter(uri.query()) {
        Ok(filter) => filter,
        Err(e) => return job_error_response(e),
    };
    match state.regression_store.list(&workspace_id, filter).await {
        Ok(cases) => Json(RegressionCaseListResponse { cases }).into_response(),
        Err(e) => regression_store_error_response(e),
    }
}

/// `POST /v1/redteam/regressions/run` — create a normal red-team job from
/// promoted regression cases. The new job reuses the source job's
/// target/profile/environment and passes the cases as runner attack-vector
/// seeds.
#[utoipa::path(
    post,
    path = "/v1/redteam/regressions/run",
    tag = "redteam",
    request_body = RegressionRunRequest,
    responses(
        (status = 201, description = "Regression job queued", body = RegressionRunResponse),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Source job not found", body = ApiError),
        (status = 503, description = "Dispatch worker unavailable", body = ApiError),
    ),
)]
pub async fn run_regression_cases(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Json(input): Json<RegressionRunRequest>,
) -> Response {
    let Some(source_job_id) = clean_optional(Some(input.source_job_id)) else {
        return job_error_response(RedteamJobStoreError::Validation(
            "source_job_id is required".into(),
        ));
    };
    let source_job_id = match normalize_uuid(source_job_id, "source_job_id") {
        Ok(source_job_id) => source_job_id,
        Err(e) => return job_error_response(e),
    };
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let Some(dispatch_tx) = state.dispatch_tx.clone() else {
        return job_error_response(RedteamJobStoreError::Unavailable(
            "red-team execution is not configured for this deployment; contact TrustLoopGuard to enable managed or enterprise execution".into(),
        ));
    };
    let source_job = match state.store.get(&workspace_id, &source_job_id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    let requested_keys = clean_case_keys(input.case_keys);
    if requested_keys.len() > 100 {
        return job_error_response(RedteamJobStoreError::Validation(
            "case_keys is capped at 100 entries".into(),
        ));
    }
    let case_limit = input.limit.unwrap_or(20).clamp(1, 100);
    let list_limit = if requested_keys.is_empty() {
        case_limit
    } else {
        requested_keys.len()
    };
    let mut cases = match state
        .regression_store
        .list(
            &workspace_id,
            RedteamRegressionCaseFilter {
                agent_id: source_job.agent_id.clone(),
                source_job_id: Some(source_job.id.clone()),
                case_keys: requested_keys.clone(),
                limit: list_limit,
            },
        )
        .await
    {
        Ok(cases) => cases,
        Err(e) => return regression_store_error_response(e),
    };
    if !requested_keys.is_empty() {
        let requested: BTreeSet<String> = requested_keys.into_iter().collect();
        cases.retain(|case| requested.contains(&case.case_key));
        let found: BTreeSet<&str> = cases.iter().map(|case| case.case_key.as_str()).collect();
        if let Some(missing) = requested
            .iter()
            .find(|case_key| !found.contains(case_key.as_str()))
        {
            return job_error_response(RedteamJobStoreError::Validation(format!(
                "regression case not found for source job: {missing}"
            )));
        }
    }
    if cases.is_empty() {
        return job_error_response(RedteamJobStoreError::Validation(
            "no regression cases found for source job".into(),
        ));
    }
    let case_keys: Vec<String> = cases.iter().map(|case| case.case_key.clone()).collect();
    let request = RedteamDispatchRequest {
        target_url: source_job.target.clone(),
        profile: source_job.profile.clone(),
        mode: RedteamRunMode::OneOff,
        attack_surface: tl_core::RedteamAttackSurface::Chat,
        agent_id: source_job.agent_id.clone(),
        document_template: None,
        attack_vectors: Some(cases.iter().map(regression_attack_vector).collect()),
    };
    let job = match state
        .store
        .create(&workspace_id, &source_job.environment_id, &request)
        .await
    {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    let message = DispatchJob {
        workspace_id: workspace_id.clone(),
        environment_id: source_job.environment_id,
        job_id: job.id.clone(),
        request,
    };
    match dispatch_tx.try_send(message) {
        Ok(()) => (
            StatusCode::CREATED,
            Json(RegressionRunResponse {
                job,
                case_count: case_keys.len() as u32,
                case_keys,
            }),
        )
            .into_response(),
        Err(_) => {
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
                    "redteam: failed to mark regression job Error after dispatch send failed; job may be stranded"
                );
            }
            job_error_response(RedteamJobStoreError::Unavailable(
                "dispatch queue is full; retry shortly".into(),
            ))
        }
    }
}

/// `GET /v1/redteam/regressions/results` — list durable regression result
/// snapshots, newest first.
#[utoipa::path(
    get,
    path = "/v1/redteam/regressions/results",
    tag = "redteam",
    params(
        ("source_job_id" = Option<String>, Query, description = "Filter by source job whose promoted cases were re-run"),
        ("job_id" = Option<String>, Query, description = "Filter by regression job id"),
        ("agent_id" = Option<String>, Query, description = "Filter by associated agent id"),
        ("limit" = Option<usize>, Query, minimum = 1, description = "Maximum snapshots to return, capped at 100"),
    ),
    responses(
        (status = 200, description = "Regression result history", body = RegressionResultTrendResponse),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_regression_result_snapshots(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let filter = match read_regression_result_snapshot_filter(uri.query()) {
        Ok(filter) => filter,
        Err(e) => return job_error_response(e),
    };
    match state
        .regression_store
        .list_result_snapshots(&workspace_id, filter)
        .await
    {
        Ok(snapshots) => Json(RegressionResultTrendResponse { snapshots }).into_response(),
        Err(e) => regression_store_error_response(e),
    }
}

/// `GET /v1/redteam/regressions/results/{job_id}` — summarize a completed
/// regression job against promoted cases from the source job.
#[utoipa::path(
    get,
    path = "/v1/redteam/regressions/results/{job_id}",
    tag = "redteam",
    params(
        ("job_id" = String, Path, description = "Regression job id to summarize"),
        ("source_job_id" = String, Query, description = "Source job whose promoted cases were re-run"),
        ("case_key" = Option<String>, Query, description = "Optional repeated case key filter"),
        ("case_keys" = Option<String>, Query, description = "Optional comma-separated case key filter"),
        ("limit" = Option<usize>, Query, minimum = 1, description = "Maximum cases to summarize, capped at 100"),
    ),
    responses(
        (status = 200, description = "Regression result summary", body = RegressionResultSummaryResponse),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Regression job not found", body = ApiError),
    ),
)]
pub async fn get_regression_results(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let job = match state.store.get(&workspace_id, &job_id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    let filter = match read_regression_result_filter(uri.query()) {
        Ok(filter) => filter,
        Err(e) => return job_error_response(e),
    };
    let sessions = match state.store.list_sessions(&workspace_id, &job_id).await {
        Ok(sessions) => sessions,
        Err(e) => return job_error_response(e),
    };
    let requested_keys = filter.case_keys.clone();
    let mut cases = match state
        .regression_store
        .list(&workspace_id, filter.case_filter())
        .await
    {
        Ok(cases) => cases,
        Err(e) => return regression_store_error_response(e),
    };
    if !requested_keys.is_empty() {
        let requested: BTreeSet<String> = requested_keys.into_iter().collect();
        cases.retain(|case| requested.contains(&case.case_key));
        let found: BTreeSet<&str> = cases.iter().map(|case| case.case_key.as_str()).collect();
        if let Some(missing) = requested
            .iter()
            .find(|case_key| !found.contains(case_key.as_str()))
        {
            return job_error_response(RedteamJobStoreError::Validation(format!(
                "regression case not found for source job: {missing}"
            )));
        }
    }

    let results: Vec<RegressionCaseResult> = cases
        .iter()
        .map(|case| regression_case_result(case, &sessions))
        .collect();
    let mut passed = 0;
    let mut failed = 0;
    let mut missing = 0;
    let mut inconclusive = 0;
    for result in &results {
        match result.status {
            RegressionResultStatus::Passed => passed += 1,
            RegressionResultStatus::Failed => failed += 1,
            RegressionResultStatus::Missing => missing += 1,
            RegressionResultStatus::Inconclusive => inconclusive += 1,
        }
    }

    if let Err(e) = state
        .regression_store
        .record_result_snapshot(
            &workspace_id,
            NewRegressionResultSnapshot {
                job_id: job.id.clone(),
                source_job_id: filter.source_job_id.clone(),
                environment_id: job.environment_id.clone(),
                agent_id: job.agent_id.clone(),
                case_keys: filter.case_keys.clone(),
                total: results.len() as u32,
                passed,
                failed,
                missing,
                inconclusive,
            },
        )
        .await
    {
        return regression_store_error_response(e);
    }

    Json(RegressionResultSummaryResponse {
        job,
        source_job_id: filter.source_job_id,
        total: results.len() as u32,
        passed,
        failed,
        missing,
        inconclusive,
        results,
    })
    .into_response()
}

/// `429` body for a rate-limited public report read.
fn rate_limited_response() -> Response {
    let body = ApiError {
        code: ApiErrorCode::RateLimited,
        message: "too many requests for this report link; retry shortly".into(),
        retriable: true,
        details: serde_json::json!(null),
    };
    (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response()
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

fn regression_attack_vector(case: &RegressionCaseSummary) -> AttackVector {
    AttackVector {
        goal: case.goal.clone(),
        technique: format!("regression_{}", case.substrate.replace('-', "_")),
        target_operation: case.artifact_id.clone(),
        injection_payload: case.attack.clone(),
        source_path: None,
    }
}

#[derive(Debug, Clone)]
struct RegressionResultFilter {
    source_job_id: String,
    case_keys: Vec<String>,
    limit: usize,
}

impl RegressionResultFilter {
    fn case_filter(&self) -> RedteamRegressionCaseFilter {
        RedteamRegressionCaseFilter {
            source_job_id: Some(self.source_job_id.clone()),
            case_keys: self.case_keys.clone(),
            limit: self.limit,
            ..RedteamRegressionCaseFilter::default()
        }
    }
}

fn regression_case_result(
    case: &RegressionCaseSummary,
    sessions: &[tl_core::RedteamAttackSession],
) -> RegressionCaseResult {
    let Some(session) = find_regression_session(case, sessions) else {
        return RegressionCaseResult {
            case_key: case.case_key.clone(),
            expected_outcome: case.expected_outcome,
            status: RegressionResultStatus::Missing,
            session_id: None,
            actual_outcome: None,
            landed: None,
            reason: Some("no matching session in regression job".into()),
        };
    };

    let status = if session.status == "error" || session.outcome == "error" {
        RegressionResultStatus::Inconclusive
    } else if regression_expected_outcome_passed(case.expected_outcome, session) {
        RegressionResultStatus::Passed
    } else {
        RegressionResultStatus::Failed
    };
    let reason = match status {
        RegressionResultStatus::Passed => None,
        RegressionResultStatus::Failed => Some(format!(
            "expected {:?}, got outcome `{}` landed={}",
            case.expected_outcome, session.outcome, session.landed
        )),
        RegressionResultStatus::Missing => Some("no matching session in regression job".into()),
        RegressionResultStatus::Inconclusive => Some(
            session
                .error
                .clone()
                .unwrap_or_else(|| "runner reported an inconclusive/error outcome".into()),
        ),
    };

    RegressionCaseResult {
        case_key: case.case_key.clone(),
        expected_outcome: case.expected_outcome,
        status,
        session_id: Some(session.session_id.clone()),
        actual_outcome: Some(session.outcome.clone()),
        landed: Some(session.landed),
        reason,
    }
}

fn find_regression_session<'a>(
    case: &RegressionCaseSummary,
    sessions: &'a [tl_core::RedteamAttackSession],
) -> Option<&'a tl_core::RedteamAttackSession> {
    sessions
        .iter()
        .find(|session| session.case_id.as_deref() == Some(case.case_key.as_str()))
        .or_else(|| {
            sessions
                .iter()
                .find(|session| session.attack == case.attack && session.goal == case.goal)
        })
}

fn regression_expected_outcome_passed(
    expected: RegressionExpectedOutcome,
    session: &tl_core::RedteamAttackSession,
) -> bool {
    match expected {
        RegressionExpectedOutcome::Block => session.outcome == "blocked" && !session.landed,
        RegressionExpectedOutcome::Escalate => {
            matches!(
                session.outcome.as_str(),
                "blocked" | "escalated" | "escalate"
            ) && !session.landed
        }
        RegressionExpectedOutcome::Stop => !session.landed && session.outcome != "landed",
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

/// Parse `agent_id` + `source_job_id` + `limit` from the regression query
/// string. Unknown keys ignored; `limit` defaults to 50 and is clamped by the
/// store.
fn read_regression_filter(
    query: Option<&str>,
) -> Result<RedteamRegressionCaseFilter, RedteamJobStoreError> {
    let mut filter = RedteamRegressionCaseFilter {
        limit: 50,
        ..RedteamRegressionCaseFilter::default()
    };
    let parts = query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned());
    for (key, value) in parts {
        match key.as_str() {
            "agent_id" => filter.agent_id = clean_optional(Some(value)),
            "source_job_id" => {
                filter.source_job_id = clean_optional(Some(value))
                    .map(|value| normalize_uuid(value, "source_job_id"))
                    .transpose()?
            }
            "limit" => filter.limit = value.parse().unwrap_or(50),
            _ => {}
        }
    }
    Ok(filter)
}

fn read_regression_result_filter(
    query: Option<&str>,
) -> Result<RegressionResultFilter, RedteamJobStoreError> {
    let mut source_job_id = None;
    let mut case_keys = Vec::new();
    let mut limit = 100;
    let parts = query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned());
    for (key, value) in parts {
        match key.as_str() {
            "source_job_id" => {
                source_job_id = clean_optional(Some(value))
                    .map(|value| normalize_uuid(value, "source_job_id"))
                    .transpose()?
            }
            "case_key" => {
                if let Some(value) = clean_optional(Some(value)) {
                    case_keys.push(value);
                }
            }
            "case_keys" => {
                for value in value.split(',') {
                    if let Some(value) = clean_optional(Some(value.to_string())) {
                        case_keys.push(value);
                    }
                }
            }
            "limit" => limit = value.parse().unwrap_or(100),
            _ => {}
        }
    }
    let Some(source_job_id) = source_job_id else {
        return Err(RedteamJobStoreError::Validation(
            "source_job_id is required".into(),
        ));
    };
    Ok(RegressionResultFilter {
        source_job_id,
        case_keys: clean_case_keys(case_keys),
        limit: limit.clamp(1, 100),
    })
}

fn read_regression_result_snapshot_filter(
    query: Option<&str>,
) -> Result<RedteamRegressionResultFilter, RedteamJobStoreError> {
    let mut filter = RedteamRegressionResultFilter {
        limit: 50,
        ..RedteamRegressionResultFilter::default()
    };
    let parts = query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned());
    for (key, value) in parts {
        match key.as_str() {
            "source_job_id" => {
                filter.source_job_id = clean_optional(Some(value))
                    .map(|value| normalize_uuid(value, "source_job_id"))
                    .transpose()?
            }
            "job_id" => {
                filter.job_id = clean_optional(Some(value))
                    .map(|value| normalize_uuid(value, "job_id"))
                    .transpose()?
            }
            "agent_id" => filter.agent_id = clean_optional(Some(value)),
            "limit" => filter.limit = value.parse().unwrap_or(50),
            _ => {}
        }
    }
    filter.limit = filter.limit.clamp(1, 100);
    Ok(filter)
}

fn clean_case_keys(case_keys: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut cleaned = Vec::new();
    for case_key in case_keys
        .into_iter()
        .filter_map(|key| clean_optional(Some(key)))
    {
        if seen.insert(case_key.clone()) {
            cleaned.push(case_key);
        }
    }
    cleaned
}

fn normalize_uuid(value: String, field: &str) -> Result<String, RedteamJobStoreError> {
    uuid::Uuid::parse_str(&value)
        .map(|id| id.to_string())
        .map_err(|_| RedteamJobStoreError::Validation(format!("{field} must be a valid UUID")))
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

fn regression_store_error_response(error: RedteamRegressionStoreError) -> Response {
    match error {
        RedteamRegressionStoreError::NotFound => crate::policies::api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "regression case not found".to_string(),
        ),
        RedteamRegressionStoreError::Internal(message) => crate::policies::api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            message,
        ),
    }
}
