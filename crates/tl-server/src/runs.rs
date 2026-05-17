//! Dashboard run grouping endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{
    ApiError, ApiErrorCode, CreateRunEventRequest, CreateRunRequest, RunDetail, RunEventKind,
    RunEventListResponse, RunEventSummary, RunKind, RunListResponse, RunStatus, RunSummary,
    TraceListResponse, TraceSummary, UpdateRunRequest,
};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum RunStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Default)]
pub struct RunListFilter {
    pub agent_id: Option<String>,
    pub status: Option<RunStatus>,
    pub kind: Option<RunKind>,
    pub external_id: Option<String>,
    pub limit: usize,
}

#[async_trait]
pub trait RunStore: Send + Sync {
    async fn create(
        &self,
        workspace_id: &str,
        input: CreateRunRequest,
    ) -> Result<RunSummary, RunStoreError>;
    async fn list(
        &self,
        workspace_id: &str,
        filter: RunListFilter,
    ) -> Result<Vec<RunSummary>, RunStoreError>;
    async fn get(&self, workspace_id: &str, run_id: &str) -> Result<RunSummary, RunStoreError>;
    async fn update(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: UpdateRunRequest,
    ) -> Result<RunSummary, RunStoreError>;
    async fn create_event(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: CreateRunEventRequest,
    ) -> Result<RunEventSummary, RunStoreError>;
    async fn events(
        &self,
        workspace_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<RunEventSummary>, RunStoreError>;
    async fn traces(
        &self,
        workspace_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<TraceSummary>, RunStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryRunStore {
    runs: RwLock<HashMap<String, RunSummary>>,
    events: RwLock<HashMap<String, Vec<RunEventSummary>>>,
}

impl MemoryRunStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RunStore for MemoryRunStore {
    async fn create(
        &self,
        workspace_id: &str,
        input: CreateRunRequest,
    ) -> Result<RunSummary, RunStoreError> {
        validate_create_run(&input)?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::now_v7().to_string();
        let run = RunSummary {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            agent_id: input.agent_id.trim().to_string(),
            kind: input.kind,
            status: input.status.unwrap_or(RunStatus::Running),
            external_id: clean_optional(input.external_id),
            metadata: normalize_metadata(input.metadata),
            started_at: now.clone(),
            ended_at: None,
            created_at: now.clone(),
            updated_at: now,
            trace_count: 0,
            blocked_count: 0,
            rewritten_count: 0,
            escalated_count: 0,
            p95_latency_ms: None,
        };
        self.runs.write().await.insert(id, run.clone());
        Ok(run)
    }

    async fn list(
        &self,
        workspace_id: &str,
        filter: RunListFilter,
    ) -> Result<Vec<RunSummary>, RunStoreError> {
        let mut rows: Vec<_> = self
            .runs
            .read()
            .await
            .values()
            .filter(|run| run.workspace_id == workspace_id)
            .filter(|run| {
                filter
                    .agent_id
                    .as_deref()
                    .map_or(true, |id| run.agent_id == id)
            })
            .filter(|run| filter.status.map_or(true, |status| run.status == status))
            .filter(|run| filter.kind.map_or(true, |kind| run.kind == kind))
            .filter(|run| {
                filter.external_id.as_deref().map_or(true, |external_id| {
                    run.external_id.as_deref() == Some(external_id)
                })
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        rows.truncate(filter.limit.clamp(1, 100));
        Ok(rows)
    }

    async fn get(&self, workspace_id: &str, run_id: &str) -> Result<RunSummary, RunStoreError> {
        self.runs
            .read()
            .await
            .get(run_id)
            .filter(|run| run.workspace_id == workspace_id)
            .cloned()
            .ok_or(RunStoreError::NotFound)
    }

    async fn update(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: UpdateRunRequest,
    ) -> Result<RunSummary, RunStoreError> {
        validate_update_run(&input)?;
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .filter(|run| run.workspace_id == workspace_id)
            .ok_or(RunStoreError::NotFound)?;
        if let Some(status) = input.status {
            run.status = status;
            if matches!(
                status,
                RunStatus::Completed | RunStatus::Failed | RunStatus::Canceled
            ) && run.ended_at.is_none()
            {
                run.ended_at = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        if let Some(metadata) = input.metadata {
            run.metadata = normalize_metadata(metadata);
        }
        if let Some(ended_at) = input.ended_at {
            run.ended_at = Some(ended_at);
        }
        run.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(run.clone())
    }

    async fn traces(
        &self,
        workspace_id: &str,
        run_id: &str,
        _limit: usize,
    ) -> Result<Vec<TraceSummary>, RunStoreError> {
        self.get(workspace_id, run_id).await?;
        Ok(vec![])
    }

    async fn create_event(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: CreateRunEventRequest,
    ) -> Result<RunEventSummary, RunStoreError> {
        validate_create_run_event(&input)?;
        self.get(workspace_id, run_id).await?;
        let mut events = self.events.write().await;
        let run_events = events.entry(run_id.to_string()).or_default();
        let sequence = input.sequence.unwrap_or_else(|| {
            run_events
                .last()
                .map(|event| event.sequence + 1)
                .unwrap_or(1)
        });
        let now = chrono::Utc::now().to_rfc3339();
        let event = RunEventSummary {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            run_id: run_id.to_string(),
            sequence,
            kind: input.kind,
            label: clean_optional(input.label),
            input_summary: clean_optional(input.input_summary),
            output_summary: clean_optional(input.output_summary),
            metadata: normalize_metadata(input.metadata),
            occurred_at: input.occurred_at.unwrap_or_else(|| now.clone()),
            created_at: now,
        };
        run_events.push(event.clone());
        run_events.sort_by_key(|event| event.sequence);
        Ok(event)
    }

    async fn events(
        &self,
        workspace_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<RunEventSummary>, RunStoreError> {
        self.get(workspace_id, run_id).await?;
        let events = self.events.read().await;
        let mut rows = events.get(run_id).cloned().unwrap_or_default();
        rows.retain(|event| event.workspace_id == workspace_id);
        rows.truncate(limit.clamp(1, 200));
        Ok(rows)
    }
}

#[derive(Clone)]
pub struct RunState {
    pub store: Arc<dyn RunStore>,
}

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
    match state.store.create(&workspace_id, input).await {
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
    match state.store.list(&workspace_id, filter).await {
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
    let run = match state.store.get(&workspace_id, &id).await {
        Ok(run) => run,
        Err(e) => return run_error_response(e),
    };
    match state.store.traces(&workspace_id, &id, 100).await {
        Ok(traces) => match state.store.events(&workspace_id, &id, 200).await {
            Ok(events) => Json(RunDetail {
                run,
                events,
                traces,
            })
            .into_response(),
            Err(e) => run_error_response(e),
        },
        Err(e) => run_error_response(e),
    }
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
    match state.store.update(&workspace_id, &id, input).await {
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
    match state.store.create_event(&workspace_id, &id, input).await {
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
    let limit = read_limit(uri.query()).unwrap_or(100).clamp(1, 200);
    match state.store.events(&workspace_id, &id, limit).await {
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
    let limit = read_limit(uri.query()).unwrap_or(50).clamp(1, 100);
    match state.store.traces(&workspace_id, &id, limit).await {
        Ok(traces) => Json(TraceListResponse { traces }).into_response(),
        Err(e) => run_error_response(e),
    }
}

fn validate_create_run(input: &CreateRunRequest) -> Result<(), RunStoreError> {
    if input.agent_id.trim().is_empty() {
        return Err(RunStoreError::Validation("agent_id is required".into()));
    }
    validate_metadata(&input.metadata)
}

fn validate_update_run(input: &UpdateRunRequest) -> Result<(), RunStoreError> {
    if let Some(metadata) = input.metadata.as_ref() {
        validate_metadata(metadata)?;
    }
    if let Some(ended_at) = input.ended_at.as_ref() {
        chrono::DateTime::parse_from_rfc3339(ended_at)
            .map_err(|_| RunStoreError::Validation("ended_at must be RFC 3339".into()))?;
    }
    Ok(())
}

pub(crate) fn validate_create_run_event(
    input: &CreateRunEventRequest,
) -> Result<(), RunStoreError> {
    if input.sequence.is_some_and(|sequence| sequence < 1) {
        return Err(RunStoreError::Validation(
            "sequence must be greater than 0".into(),
        ));
    }
    if let Some(occurred_at) = input.occurred_at.as_ref() {
        chrono::DateTime::parse_from_rfc3339(occurred_at)
            .map_err(|_| RunStoreError::Validation("occurred_at must be RFC 3339".into()))?;
    }
    validate_metadata(&input.metadata)
}

fn validate_metadata(value: &serde_json::Value) -> Result<(), RunStoreError> {
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(RunStoreError::Validation(
            "metadata must be a JSON object".into(),
        ))
    }
}

fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        json!({})
    } else {
        value
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn read_filter(query: Option<&str>) -> Result<RunListFilter, RunStoreError> {
    let mut filter = RunListFilter {
        limit: 20,
        ..RunListFilter::default()
    };
    for (key, value) in query_parts(query) {
        match key.as_str() {
            "agent_id" => filter.agent_id = clean_optional(Some(value)),
            "external_id" => filter.external_id = clean_optional(Some(value)),
            "status" => filter.status = Some(parse_status(&value)?),
            "kind" => filter.kind = Some(parse_kind(&value)?),
            "limit" => {
                filter.limit = value.parse().unwrap_or(20);
            }
            _ => {}
        }
    }
    Ok(filter)
}

fn read_limit(query: Option<&str>) -> Option<usize> {
    query_parts(query).find_map(|(key, value)| {
        if key == "limit" {
            value.parse().ok()
        } else {
            None
        }
    })
}

fn query_parts(query: Option<&str>) -> impl Iterator<Item = (String, String)> + '_ {
    query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned())
}

fn parse_kind(value: &str) -> Result<RunKind, RunStoreError> {
    match value {
        "chat_session" => Ok(RunKind::ChatSession),
        "live_call" => Ok(RunKind::LiveCall),
        "workflow" => Ok(RunKind::Workflow),
        "job" => Ok(RunKind::Job),
        "other" => Ok(RunKind::Other),
        other => Err(RunStoreError::Validation(format!(
            "unknown run kind: {other}"
        ))),
    }
}

fn parse_status(value: &str) -> Result<RunStatus, RunStoreError> {
    match value {
        "warming" => Ok(RunStatus::Warming),
        "running" => Ok(RunStatus::Running),
        "completed" => Ok(RunStatus::Completed),
        "failed" => Ok(RunStatus::Failed),
        "canceled" => Ok(RunStatus::Canceled),
        other => Err(RunStoreError::Validation(format!(
            "unknown run status: {other}"
        ))),
    }
}

#[allow(dead_code)]
fn parse_event_kind(value: &str) -> Result<RunEventKind, RunStoreError> {
    match value {
        "user_turn" => Ok(RunEventKind::UserTurn),
        "assistant_turn" => Ok(RunEventKind::AssistantTurn),
        "tool_call" => Ok(RunEventKind::ToolCall),
        "workflow_step" => Ok(RunEventKind::WorkflowStep),
        "interruption" => Ok(RunEventKind::Interruption),
        "retry" => Ok(RunEventKind::Retry),
        "system_event" => Ok(RunEventKind::SystemEvent),
        "other" => Ok(RunEventKind::Other),
        other => Err(RunStoreError::Validation(format!(
            "unknown run event kind: {other}"
        ))),
    }
}

fn run_error_response(error: RunStoreError) -> Response {
    let (status, code) = match error {
        RunStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        RunStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        RunStoreError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal),
    };
    crate::log_api_error(status, code, &error.to_string());
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable: matches!(code, ApiErrorCode::RateLimited | ApiErrorCode::Unavailable),
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
