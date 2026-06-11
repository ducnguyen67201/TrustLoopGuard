//! Dashboard trace read endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode, TraceListResponse, TraceSummary};

use crate::environments::EnvironmentStore;

#[derive(Debug, thiserror::Error)]
pub enum TraceStoreError {
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait TraceStore: Send + Sync {
    async fn list_recent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TraceSummary>, TraceStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryTraceStore;

#[async_trait]
impl TraceStore for MemoryTraceStore {
    async fn list_recent(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _session_id: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<TraceSummary>, TraceStoreError> {
        Ok(vec![])
    }
}

#[derive(Clone)]
pub struct TraceState {
    pub store: Arc<dyn TraceStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
}

/// `GET /v1/traces` - list recent persisted decision traces for a workspace.
#[utoipa::path(
    get,
    path = "/v1/traces",
    tag = "traces",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum traces to return, capped at 100"),
        ("session_id" = Option<String>, Query, description = "Return only traces tagged with this monitoring session id"),
    ),
    responses(
        (status = 200, description = "Recent traces", body = TraceListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_traces(
    State(state): State<TraceState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(environment_id) => environment_id,
        Err(error) => return crate::environments::environment_error_response(error),
    };
    let limit = read_query_param(uri.query(), "limit")
        .and_then(|value| value.parse().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let session_id = read_query_param(uri.query(), "session_id");
    match state
        .store
        .list_recent(&workspace_id, &environment_id, session_id.as_deref(), limit)
        .await
    {
        Ok(traces) => Json(TraceListResponse { traces }).into_response(),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

/// Read a single query parameter. An empty value is treated as absent.
fn read_query_param(query: Option<&str>, name: &str) -> Option<String> {
    query?.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key == name && !value.is_empty() {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    crate::log_api_error(status, code, &message);
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
