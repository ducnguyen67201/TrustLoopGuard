//! Human review event and analytics endpoints.

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
    ApiError, ApiErrorCode, CreateHumanReviewEventRequest, HumanReviewAnalyticsResponse,
    HumanReviewAnalyticsSummary, HumanReviewEvent, HumanReviewEventListResponse,
    HumanReviewOutcomeCounts,
};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum HumanReviewStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Default)]
pub struct HumanReviewAnalyticsFilter {
    pub agent_id: Option<String>,
    pub policy_id: Option<String>,
    pub run_kind: Option<String>,
    pub workflow_step: Option<String>,
}

#[async_trait]
pub trait HumanReviewStore: Send + Sync {
    async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<HumanReviewEvent, HumanReviewStoreError>;

    async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: usize,
    ) -> Result<Vec<HumanReviewEvent>, HumanReviewStoreError>;

    async fn analytics(
        &self,
        workspace_id: &str,
        filter: HumanReviewAnalyticsFilter,
    ) -> Result<HumanReviewAnalyticsResponse, HumanReviewStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryHumanReviewStore {
    events: RwLock<HashMap<String, Vec<HumanReviewEvent>>>,
}

impl MemoryHumanReviewStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl HumanReviewStore for MemoryHumanReviewStore {
    async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<HumanReviewEvent, HumanReviewStoreError> {
        validate_create_event(&input)?;
        let now = chrono::Utc::now().to_rfc3339();
        let event = HumanReviewEvent {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            trace_id: trace_id.to_string(),
            run_id: None,
            run_event_id: None,
            outcome: input.outcome,
            reason_codes: input
                .reason_codes
                .into_iter()
                .filter_map(|value| clean_string(value.trim()))
                .collect(),
            note: input.note.and_then(|value| clean_string(value.trim())),
            reviewer_id: reviewer_id.and_then(|value| clean_string(value.trim())),
            metadata: normalize_metadata(input.metadata),
            created_at: now,
        };
        self.events
            .write()
            .await
            .entry(key(workspace_id, trace_id))
            .or_default()
            .push(event.clone());
        Ok(event)
    }

    async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: usize,
    ) -> Result<Vec<HumanReviewEvent>, HumanReviewStoreError> {
        let events = self.events.read().await;
        let mut rows = events
            .get(&key(workspace_id, trace_id))
            .cloned()
            .unwrap_or_default();
        rows.truncate(limit.clamp(1, 100));
        Ok(rows)
    }

    async fn analytics(
        &self,
        _workspace_id: &str,
        _filter: HumanReviewAnalyticsFilter,
    ) -> Result<HumanReviewAnalyticsResponse, HumanReviewStoreError> {
        Ok(empty_analytics())
    }
}

#[derive(Clone)]
pub struct HumanReviewState {
    pub store: Arc<dyn HumanReviewStore>,
}

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

fn validate_create_event(
    input: &CreateHumanReviewEventRequest,
) -> Result<(), HumanReviewStoreError> {
    if !(input.metadata.is_null() || input.metadata.is_object()) {
        return Err(HumanReviewStoreError::Validation(
            "metadata must be a JSON object".into(),
        ));
    }
    if input
        .reason_codes
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(HumanReviewStoreError::Validation(
            "reason_codes must not contain empty values".into(),
        ));
    }
    Ok(())
}

fn read_filter(query: Option<&str>) -> HumanReviewAnalyticsFilter {
    let mut filter = HumanReviewAnalyticsFilter::default();
    for (key, value) in query_parts(query) {
        match key.as_str() {
            "agent_id" => filter.agent_id = clean_string(&value),
            "policy_id" => filter.policy_id = clean_string(&value),
            "run_kind" => filter.run_kind = clean_string(&value),
            "workflow_step" => filter.workflow_step = clean_string(&value),
            _ => {}
        }
    }
    filter
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
    query.into_iter().flat_map(|query| {
        query.split('&').filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            let key = url::form_urlencoded::parse(key.as_bytes()).next()?.0;
            let value = url::form_urlencoded::parse(value.as_bytes()).next()?.0;
            Some((key.into_owned(), value.into_owned()))
        })
    })
}

fn clean_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        json!({})
    } else {
        value
    }
}

fn key(workspace_id: &str, trace_id: &str) -> String {
    format!("{workspace_id}:{trace_id}")
}

fn empty_analytics() -> HumanReviewAnalyticsResponse {
    HumanReviewAnalyticsResponse {
        summary: HumanReviewAnalyticsSummary::default(),
        outcomes: HumanReviewOutcomeCounts::default(),
        by_workflow_step: vec![],
        by_policy: vec![],
        by_agent: vec![],
        by_run_kind: vec![],
        top_reasons: vec![],
    }
}

pub fn review_error_response(error: HumanReviewStoreError) -> Response {
    let (status, code) = match error {
        HumanReviewStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        HumanReviewStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        HumanReviewStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    crate::log_api_error(status, code, &error.to_string());
    let retriable = matches!(code, ApiErrorCode::Internal | ApiErrorCode::Unavailable);
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
