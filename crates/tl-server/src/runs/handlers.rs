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
    ApiError, CreateRunEventRequest, CreateRunRequest, FinalizeRunRequest, FinalizeRunResponse,
    RunBoundarySource, RunDetail, RunEventListResponse, RunGuardrailUsage, RunKind,
    RunListResponse, RunLlmBudgetDecision, RunParticipantRole, RunProviderUsage, RunStatus,
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let agent_id = input.agent_id.trim().to_string();
    match state
        .store
        .create(&workspace_id, &environment_id, input)
        .await
    {
        Ok(run) => match state
            .evaluation_store
            .register_participant_and_freeze_manifest(
                &workspace_id,
                &environment_id,
                &run.id,
                &agent_id,
                RunParticipantRole::Primary,
            )
            .await
        {
            Ok(()) => (StatusCode::CREATED, Json(run)).into_response(),
            Err(error) => crate::evaluations::evaluation_error_response(error),
        },
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
                let finalization = match state
                    .store
                    .finalization(&workspace_id, &environment_id, &id)
                    .await
                {
                    Ok(value) => value,
                    Err(error) => return run_error_response(error),
                };
                let participants = match state
                    .evaluation_store
                    .list_participants(&workspace_id, &id)
                    .await
                {
                    Ok(value) => value,
                    Err(error) => return crate::evaluations::evaluation_error_response(error),
                };
                let evaluations = match state
                    .evaluation_store
                    .list_results(&workspace_id, &environment_id, &id)
                    .await
                {
                    Ok(value) => value.into_iter().map(|detail| detail.result).collect(),
                    Err(error) => return crate::evaluations::evaluation_error_response(error),
                };
                let evaluation_jobs = match state
                    .evaluation_store
                    .list_jobs(&workspace_id, &environment_id, &id)
                    .await
                {
                    Ok(value) => value,
                    Err(error) => return crate::evaluations::evaluation_error_response(error),
                };
                Json(RunDetail {
                    run,
                    events,
                    traces,
                    provider_usage,
                    guardrail_usage,
                    budget_decision,
                    finalization,
                    participants,
                    evaluation_jobs,
                    evaluations,
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    if input.status.is_some_and(RunStatus::is_terminal) {
        let status = input.status.expect("terminal status was checked");
        let capture_wait_ms =
            evaluation_capture_wait(&state, &workspace_id, &environment_id, &id).await;
        let finalized = state
            .store
            .finalize(
                &workspace_id,
                &environment_id,
                &id,
                FinalizeRunRequest {
                    status,
                    ended_at: input.ended_at,
                    boundary_source: RunBoundarySource::LegacySdk,
                    expected_flush_id: None,
                    last_event_sequence: None,
                },
                capture_wait_ms,
            )
            .await;
        let mut response = match finalized {
            Ok(response) => response,
            Err(error) => return run_error_response(error),
        };
        if let Some(metadata) = input.metadata {
            match state
                .store
                .update(
                    &workspace_id,
                    &environment_id,
                    &id,
                    UpdateRunRequest {
                        status: None,
                        metadata: Some(metadata),
                        ended_at: None,
                    },
                )
                .await
            {
                Ok(run) => response.run = run,
                Err(error) => return run_error_response(error),
            }
        }
        Json(response.run).into_response()
    } else {
        match state
            .store
            .update(&workspace_id, &environment_id, &id, input)
            .await
        {
            Ok(run) => Json(run).into_response(),
            Err(e) => run_error_response(e),
        }
    }
}

/// `POST /v1/runs/:id/finalize` - authoritatively close a Run and arm capture.
#[utoipa::path(
    post,
    path = "/v1/runs/{id}/finalize",
    tag = "runs",
    params(("id" = String, Path, description = "Run id")),
    request_body = FinalizeRunRequest,
    responses(
        (status = 200, description = "Run finalized", body = FinalizeRunResponse),
        (status = 400, description = "Invalid terminal boundary", body = ApiError),
        (status = 409, description = "Conflicting terminal boundary", body = ApiError),
    ),
)]
pub async fn finalize_run(
    State(state): State<RunState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<FinalizeRunRequest>,
) -> Response {
    if !input.status.is_terminal() {
        return run_error_response(super::RunStoreError::Validation(
            "final run status must be terminal".into(),
        ));
    }
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let capture_wait_ms =
        evaluation_capture_wait(&state, &workspace_id, &environment_id, &id).await;
    match state
        .store
        .finalize(&workspace_id, &environment_id, &id, input, capture_wait_ms)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => run_error_response(error),
    }
}

async fn evaluation_capture_wait(
    state: &RunState,
    workspace_id: &str,
    environment_id: &str,
    run_id: &str,
) -> u64 {
    let Ok(run) = state.store.get(workspace_id, environment_id, run_id).await else {
        return 30_000;
    };
    let participant_ids = state
        .evaluation_store
        .list_participants(workspace_id, run_id)
        .await
        .map(|participants| {
            participants
                .into_iter()
                .map(|participant| participant.agent_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|_| vec![run.agent_id]);
    let mut max_capture_wait_ms = None;
    for agent_id in participant_ids {
        if let Ok(Some(profile)) = state
            .evaluation_store
            .get_profile(workspace_id, environment_id, &agent_id)
            .await
        {
            if profile.enabled {
                max_capture_wait_ms = Some(
                    max_capture_wait_ms
                        .unwrap_or(0)
                        .max(profile.max_capture_wait_ms),
                );
            }
        }
    }
    max_capture_wait_ms.unwrap_or(30_000)
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let run = match state.store.get(&workspace_id, &environment_id, &id).await {
        Ok(run) => run,
        Err(error) => return run_error_response(error),
    };
    let agent_id = input
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&run.agent_id)
        .to_string();
    let role = if agent_id == run.agent_id {
        RunParticipantRole::Primary
    } else {
        RunParticipantRole::Participant
    };
    if let Err(error) = state
        .evaluation_store
        .register_participant_and_freeze_manifest(
            &workspace_id,
            &environment_id,
            &id,
            &agent_id,
            role,
        )
        .await
    {
        return crate::evaluations::evaluation_error_response(error);
    }
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
