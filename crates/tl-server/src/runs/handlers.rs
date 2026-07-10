use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::RunEventSummary;
#[allow(unused_imports)]
use tl_core::{
    ApiError, CreateRunEventRequest, CreateRunRequest, RunDetail, RunEventListResponse,
    RunGuardrailUsage, RunKind, RunListResponse, RunLlmBudgetDecision, RunProviderUsage, RunStatus,
    RunSummary, TraceListResponse, UpdateRunRequest,
};

use super::context::resolve_environment_id;
use super::query::{read_filter, read_limit};
use super::response::run_error_response;
use super::validation::{validate_create_run, validate_create_run_event, validate_update_run};
use super::RunState;

/// `POST /v1/runs` - create a workspace run.
#[utoipa::path(
    post,
    path = "/v1/runs",
    tag = "runs",
    request_body = CreateRunRequest,
    responses(
        (status = 201, description = "Run created", body = RunSummary),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn create_run(
    State(state): State<RunState>,
    headers: HeaderMap,
    Json(input): Json<CreateRunRequest>,
) -> Response {
    if let Err(e) = validate_create_run(&input) {
        return run_error_response(e);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state
        .store
        .create(&workspace_id, &environment_id, input)
        .await
    {
        Ok(run) => (StatusCode::CREATED, Json(run)).into_response(),
        Err(e) => run_error_response(e),
    }
}

/// `GET /v1/runs` - list workspace runs.
#[utoipa::path(
    get,
    path = "/v1/runs",
    tag = "runs",
    params(
        ("agent_id" = Option<String>, Query, description = "Filter by agent id"),
        ("status" = Option<RunStatus>, Query, description = "Filter by run status"),
        ("kind" = Option<RunKind>, Query, description = "Filter by run kind"),
        ("external_id" = Option<String>, Query, description = "Filter by customer correlation id"),
        ("limit" = Option<usize>, Query, description = "Maximum runs to return, capped at 100"),
    ),
    responses(
        (status = 200, description = "Workspace runs", body = RunListResponse),
        (status = 400, description = "Malformed query", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_runs(State(state): State<RunState>, headers: HeaderMap, uri: Uri) -> Response {
    let filter = match read_filter(uri.query()) {
        Ok(filter) => filter,
        Err(e) => return run_error_response(e),
    };
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state
        .store
        .list(&workspace_id, &environment_id, filter)
        .await
    {
        Ok(runs) => Json(RunListResponse { runs }).into_response(),
        Err(e) => run_error_response(e),
    }
}

/// `GET /v1/runs/:id` - fetch a run and recent traces.
#[utoipa::path(
    get,
    path = "/v1/runs/{id}",
    tag = "runs",
    params(("id" = String, Path, description = "Run id")),
    responses(
        (status = 200, description = "Run detail", body = RunDetail),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Run not found", body = ApiError),
    ),
)]
pub async fn get_run(
    State(state): State<RunState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let run = match state.store.get(&workspace_id, &environment_id, &id).await {
        Ok(run) => run,
        Err(e) => return run_error_response(e),
    };
    match state
        .store
        .traces(&workspace_id, &environment_id, &id, 100)
        .await
    {
        Ok(traces) => match state
            .store
            .events(&workspace_id, &environment_id, &id, 200)
            .await
        {
            Ok(events) => {
                let provider_usage =
                    latest_event_evidence::<RunProviderUsage>(&events, "provider_usage");
                let budget_decision =
                    latest_event_evidence::<RunLlmBudgetDecision>(&events, "budget_decision");
                let guardrail_usage = events
                    .iter()
                    .filter_map(|event| {
                        event
                            .metadata
                            .get("guardrail_usage")
                            .cloned()
                            .and_then(|value| {
                                serde_json::from_value::<RunGuardrailUsage>(value).ok()
                            })
                    })
                    .collect();
                Json(RunDetail {
                    run,
                    events,
                    traces,
                    provider_usage,
                    guardrail_usage,
                    budget_decision,
                })
                .into_response()
            }
            Err(e) => run_error_response(e),
        },
        Err(e) => run_error_response(e),
    }
}

fn latest_event_evidence<T: serde::de::DeserializeOwned>(
    events: &[RunEventSummary],
    key: &str,
) -> Option<T> {
    events.iter().rev().find_map(|event| {
        event
            .metadata
            .get(key)
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
    })
}

/// `PATCH /v1/runs/:id` - update a run.
#[utoipa::path(
    patch,
    path = "/v1/runs/{id}",
    tag = "runs",
    params(("id" = String, Path, description = "Run id")),
    request_body = UpdateRunRequest,
    responses(
        (status = 200, description = "Run updated", body = RunSummary),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Run not found", body = ApiError),
    ),
)]
pub async fn update_run(
    State(state): State<RunState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateRunRequest>,
) -> Response {
    if let Err(e) = validate_update_run(&input) {
        return run_error_response(e);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state
        .store
        .update(&workspace_id, &environment_id, &id, input)
        .await
    {
        Ok(run) => Json(run).into_response(),
        Err(e) => run_error_response(e),
    }
}

/// `POST /v1/runs/:id/events` - append an event to a run timeline.
#[utoipa::path(
    post,
    path = "/v1/runs/{id}/events",
    tag = "runs",
    params(("id" = String, Path, description = "Run id")),
    request_body = CreateRunEventRequest,
    responses(
        (status = 201, description = "Run event created", body = RunEventSummary),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Run not found", body = ApiError),
    ),
)]
pub async fn create_run_event(
    State(state): State<RunState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<CreateRunEventRequest>,
) -> Response {
    if let Err(e) = validate_create_run_event(&input) {
        return run_error_response(e);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state
        .store
        .create_event(&workspace_id, &environment_id, &id, input)
        .await
    {
        Ok(event) => (StatusCode::CREATED, Json(event)).into_response(),
        Err(e) => run_error_response(e),
    }
}

/// `GET /v1/runs/:id/events` - list events for a run timeline.
#[utoipa::path(
    get,
    path = "/v1/runs/{id}/events",
    tag = "runs",
    params(
        ("id" = String, Path, description = "Run id"),
        ("limit" = Option<usize>, Query, description = "Maximum events to return, capped at 200"),
    ),
    responses(
        (status = 200, description = "Run events", body = RunEventListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Run not found", body = ApiError),
    ),
)]
pub async fn list_run_events(
    State(state): State<RunState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let limit = read_limit(uri.query()).unwrap_or(100).clamp(1, 200);
    match state
        .store
        .events(&workspace_id, &environment_id, &id, limit)
        .await
    {
        Ok(events) => Json(RunEventListResponse { events }).into_response(),
        Err(e) => run_error_response(e),
    }
}

/// `GET /v1/runs/:id/traces` - list traces for a run.
#[utoipa::path(
    get,
    path = "/v1/runs/{id}/traces",
    tag = "runs",
    params(
        ("id" = String, Path, description = "Run id"),
        ("limit" = Option<usize>, Query, description = "Maximum traces to return, capped at 100"),
    ),
    responses(
        (status = 200, description = "Run traces", body = TraceListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Run not found", body = ApiError),
    ),
)]
pub async fn list_run_traces(
    State(state): State<RunState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let limit = read_limit(uri.query()).unwrap_or(50).clamp(1, 100);
    match state
        .store
        .traces(&workspace_id, &environment_id, &id, limit)
        .await
    {
        Ok(traces) => Json(TraceListResponse { traces }).into_response(),
        Err(e) => run_error_response(e),
    }
}
