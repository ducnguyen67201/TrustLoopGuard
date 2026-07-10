//! LLM gateway usage metering store + `GET /v1/llm-usage`.
//!
//! Every store ships as trait + memory + postgres (see the financial
//! store trio). The gateway budget hook writes events and sums spend
//! windows here; the usage endpoint reads the same rows for dashboards
//! and budget alerts.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Extension, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use tl_core::ApiError;
use tl_core::{ApiErrorCode, LlmUsageBucketsResponse, LlmUsageListResponse};

use crate::auth::WorkspaceKeyContext;

mod memory_store;

pub use memory_store::MemoryLlmUsageStore;

#[derive(Debug, thiserror::Error)]
pub enum LlmUsageStoreError {
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// One metered gateway call, as recorded by the budget hook. The store
/// assigns the row id and `effective_at`.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordLlmUsageEvent {
    pub principal_id: String,
    pub api_key_id: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    /// USD nanos ($1 = 1,000,000,000) used for hard-cap accounting.
    pub cost_nanos: i64,
    pub currency: String,
    /// Gateway request id; unique per workspace so retried writes are
    /// idempotent.
    pub request_id: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LlmUsageFilter {
    pub principal_id: Option<String>,
    pub model: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmUsageGroupBy {
    Day,
    Principal,
    Model,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LlmBudgetCapsNanos {
    pub daily: Option<i64>,
    pub weekly: Option<i64>,
    pub monthly: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReserveLlmBudget {
    pub request_id: String,
    pub principal_id: String,
    pub api_key_id: String,
    pub currency: String,
    pub reserved_nanos: i64,
    pub caps: LlmBudgetCapsNanos,
    pub day_start: DateTime<Utc>,
    pub week_start: DateTime<Utc>,
    pub month_start: DateTime<Utc>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmBudgetWindow {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveLlmBudgetOutcome {
    Reserved,
    Exceeded {
        window: LlmBudgetWindow,
        cap_nanos: i64,
        committed_nanos: i64,
        requested_nanos: i64,
    },
}

#[async_trait]
pub trait LlmUsageStore: Send + Sync {
    /// Record one metered call. Idempotent on `(workspace_id,
    /// request_id)`.
    async fn insert_event(
        &self,
        workspace_id: &str,
        event: RecordLlmUsageEvent,
    ) -> Result<(), LlmUsageStoreError>;

    /// Atomically reserve a request's maximum provider cost against all
    /// active cap windows for its principal.
    async fn reserve_budget(
        &self,
        workspace_id: &str,
        reservation: ReserveLlmBudget,
    ) -> Result<ReserveLlmBudgetOutcome, LlmUsageStoreError>;

    /// Record actual usage and settle its active reservation.
    async fn settle_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
        event: RecordLlmUsageEvent,
    ) -> Result<(), LlmUsageStoreError>;

    /// Release an active reservation when no provider spend occurred.
    async fn release_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
    ) -> Result<(), LlmUsageStoreError>;

    /// Priced spend for one principal in `[start, end)` — the budget
    /// window sum. Plain sum; usage only accrues.
    async fn net_llm_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, LlmUsageStoreError>;

    /// Raw event list, newest first.
    async fn list_events(
        &self,
        workspace_id: &str,
        filter: &LlmUsageFilter,
    ) -> Result<LlmUsageListResponse, LlmUsageStoreError>;

    /// Rollup by day (UTC date key), principal, or model, ordered by
    /// key ascending.
    async fn grouped_usage(
        &self,
        workspace_id: &str,
        group_by: LlmUsageGroupBy,
        filter: &LlmUsageFilter,
    ) -> Result<LlmUsageBucketsResponse, LlmUsageStoreError>;
}

#[derive(Clone)]
pub struct LlmUsageState {
    pub store: Arc<dyn LlmUsageStore>,
    pub pricing_store: Arc<dyn crate::llm_pricing::LlmPricingStore>,
}

#[utoipa::path(
    get,
    path = "/v1/llm-usage",
    tag = "llm-usage",
    params(
        ("principal_id" = Option<String>, Query, description = "Filter by principal"),
        ("model" = Option<String>, Query, description = "Filter by raw model string"),
        ("start" = Option<String>, Query, description = "RFC 3339 window start (inclusive)"),
        ("end" = Option<String>, Query, description = "RFC 3339 window end (exclusive)"),
        ("group_by" = Option<String>, Query, description = "Rollup: `day`, `principal`, or `model`. Omitted = raw event list"),
    ),
    responses(
        (status = 200, description = "Raw usage events (`{\"events\": [...]}`), or `{\"buckets\": [...]}` when `group_by` is set", body = tl_core::LlmUsageResponse),
        (status = 400, description = "Malformed query", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_llm_usage(
    State(state): State<LlmUsageState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let (mut filter, group_by) = match read_query(uri.query()) {
        Ok(parsed) => parsed,
        Err(message) => return llm_usage_error_response(LlmUsageStoreError::Validation(message)),
    };
    // A runtime key only ever sees its own spend: force the principal
    // filter to the key's budget identity (bound principal, else the
    // key id — the same identity the gateway meters under), overriding
    // any caller-supplied principal_id. Workspace-wide reads stay a
    // dashboard/internal-key surface.
    if let Some(Extension(key)) = runtime_key {
        filter.principal_id = Some(key.principal_id.unwrap_or(key.api_key_id));
    }
    let result = match group_by {
        Some(group_by) => state
            .store
            .grouped_usage(&workspace_id, group_by, &filter)
            .await
            .map(|buckets| Json(buckets).into_response()),
        None => state
            .store
            .list_events(&workspace_id, &filter)
            .await
            .map(|events| Json(events).into_response()),
    };
    result.unwrap_or_else(llm_usage_error_response)
}

fn read_query(query: Option<&str>) -> Result<(LlmUsageFilter, Option<LlmUsageGroupBy>), String> {
    let mut filter = LlmUsageFilter::default();
    let mut group_by = None;
    let parts = query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned());
    for (key, value) in parts {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        match key.as_str() {
            "principal_id" => filter.principal_id = Some(trimmed.to_string()),
            "model" => filter.model = Some(trimmed.to_string()),
            "start" => filter.start = Some(parse_rfc3339("start", trimmed)?),
            "end" => filter.end = Some(parse_rfc3339("end", trimmed)?),
            "group_by" => {
                group_by = Some(match trimmed {
                    "day" => LlmUsageGroupBy::Day,
                    "principal" => LlmUsageGroupBy::Principal,
                    "model" => LlmUsageGroupBy::Model,
                    other => {
                        return Err(format!(
                            "unknown group_by `{other}`; expected day, principal, or model"
                        ))
                    }
                });
            }
            _ => {}
        }
    }
    Ok((filter, group_by))
}

fn parse_rfc3339(name: &str, value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("{name} must be RFC 3339: {e}"))
}

fn llm_usage_error_response(error: LlmUsageStoreError) -> Response {
    let (status, code) = match &error {
        LlmUsageStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        LlmUsageStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    crate::log_api_error(status, code, &error.to_string());
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable: matches!(code, ApiErrorCode::Internal),
        details: serde_json::Value::Null,
    };
    (status, Json(body)).into_response()
}
