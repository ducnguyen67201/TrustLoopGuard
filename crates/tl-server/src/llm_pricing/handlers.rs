//! `/v1/llm-pricing` — workspace-editable model prices.
//!
//! `GET` returns the effective table (workspace rows merged over the
//! built-in defaults, each row flagged with its source). `PUT`/`DELETE`
//! manage workspace rows and are admin-gated like settings writes: a
//! runtime key must never be able to reprice the spend it is billed
//! under.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{
    ApiError, ApiErrorCode, LlmModelPrice, LlmPriceSource, LlmPricingListResponse,
    UpsertLlmModelPriceRequest, USD,
};

use crate::auth::{InternalServiceContext, WorkspaceKeyContext};
use crate::dashboard_admin::authorize_workspace_admin;
use crate::jwt::UserContext;
use crate::team::TeamStore;

use super::NANOS_PER_MINOR;
use super::{normalize_model, LlmPricingStore, DEFAULT_PRICES};

/// Cap on the model key accepted at upsert/delete.
const MAX_MODEL_CHARS: usize = 256;

#[derive(Clone)]
pub struct LlmPricingState {
    pub store: Arc<dyn LlmPricingStore>,
    /// Admin-role source for the write gate.
    pub team_store: Arc<dyn TeamStore>,
}

/// `GET /v1/llm-pricing` - effective model prices: workspace rows
/// merged over the built-in defaults.
#[utoipa::path(
    get,
    path = "/v1/llm-pricing",
    tag = "llm-pricing",
    responses(
        (status = 200, description = "Effective model prices, model ascending", body = LlmPricingListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_llm_pricing(
    State(state): State<LlmPricingState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let rows = match state.store.list_prices(&workspace_id).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!(workspace_id, error = %error, "llm pricing list failed");
            return api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            );
        }
    };
    // BTreeMap keeps the merged table model-ascending; workspace rows
    // overwrite same-key defaults.
    let mut effective: BTreeMap<String, LlmModelPrice> = DEFAULT_PRICES
        .iter()
        .map(|(model, price)| {
            (
                (*model).to_string(),
                price_row((*model).to_string(), price, LlmPriceSource::Default),
            )
        })
        .collect();
    for row in rows {
        effective.insert(
            row.model.clone(),
            price_row(row.model, &row.price, LlmPriceSource::Workspace),
        );
    }
    Json(LlmPricingListResponse {
        prices: effective.into_values().collect(),
    })
    .into_response()
}

/// `PUT /v1/llm-pricing/{model}` - upsert one workspace model price.
#[utoipa::path(
    put,
    path = "/v1/llm-pricing/{model}",
    tag = "llm-pricing",
    params(("model" = String, Path, description = "Model key (normalized to trimmed lowercase)")),
    request_body = UpsertLlmModelPriceRequest,
    responses(
        (status = 200, description = "Persisted workspace model price", body = LlmModelPrice),
        (status = 400, description = "Malformed model or negative price", body = ApiError),
        (status = 401, description = "Missing or invalid credentials", body = ApiError),
        (status = 403, description = "Caller cannot modify pricing for this workspace", body = ApiError),
    ),
)]
pub async fn put_llm_price(
    State(state): State<LlmPricingState>,
    Path(model): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<UpsertLlmModelPriceRequest>,
) -> Response {
    let model = match validate_model(&model) {
        Ok(model) => model,
        Err(message) => {
            return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message)
        }
    };
    // A negative price would subtract from accumulated spend and
    // quietly defeat the budget gate.
    if req.input_per_million_minor < 0 || req.output_per_million_minor < 0 {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "prices must be non-negative".to_string(),
        );
    }
    let input_per_million_nanos = match precise_rate(
        "input_per_million_usd_nanos",
        req.input_per_million_minor,
        req.input_per_million_usd_nanos.as_deref(),
    ) {
        Ok(value) => value,
        Err(message) => {
            return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message)
        }
    };
    let output_per_million_nanos = match precise_rate(
        "output_per_million_usd_nanos",
        req.output_per_million_minor,
        req.output_per_million_usd_nanos.as_deref(),
    ) {
        Ok(value) => value,
        Err(message) => {
            return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message)
        }
    };
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "modify LLM pricing",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state
        .store
        .upsert_price(
            &workspace_id,
            &model,
            req.input_per_million_minor,
            req.output_per_million_minor,
            input_per_million_nanos,
            output_per_million_nanos,
        )
        .await
    {
        Ok(()) => Json(LlmModelPrice {
            model,
            input_per_million_minor: req.input_per_million_minor,
            output_per_million_minor: req.output_per_million_minor,
            input_per_million_usd_nanos: input_per_million_nanos.to_string(),
            output_per_million_usd_nanos: output_per_million_nanos.to_string(),
            currency: USD.to_string(),
            source: LlmPriceSource::Workspace,
        })
        .into_response(),
        Err(error) => {
            tracing::error!(workspace_id, model, error = %error, "llm price upsert failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// `DELETE /v1/llm-pricing/{model}` - remove one workspace model price;
/// the built-in default (if any) applies again.
#[utoipa::path(
    delete,
    path = "/v1/llm-pricing/{model}",
    tag = "llm-pricing",
    params(("model" = String, Path, description = "Model key (normalized to trimmed lowercase)")),
    responses(
        (status = 204, description = "Workspace model price deleted"),
        (status = 400, description = "Malformed model", body = ApiError),
        (status = 401, description = "Missing or invalid credentials", body = ApiError),
        (status = 403, description = "Caller cannot modify pricing for this workspace", body = ApiError),
        (status = 404, description = "No workspace price for this model", body = ApiError),
    ),
)]
pub async fn delete_llm_price(
    State(state): State<LlmPricingState>,
    Path(model): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let model = match validate_model(&model) {
        Ok(model) => model,
        Err(message) => {
            return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message)
        }
    };
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "modify LLM pricing",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.store.delete_price(&workspace_id, &model).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            format!("no workspace price for model `{model}`"),
        ),
        Err(error) => {
            tracing::error!(workspace_id, model, error = %error, "llm price delete failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "internal error".to_string(),
            )
        }
    }
}

/// Trimmed, non-empty, bounded model key, normalized to lowercase — the
/// same key form metering looks up.
fn validate_model(model: &str) -> Result<String, String> {
    let model = model.trim();
    if model.is_empty() {
        return Err("model must not be empty".to_string());
    }
    if model.chars().count() > MAX_MODEL_CHARS {
        return Err(format!(
            "model must be at most {MAX_MODEL_CHARS} characters"
        ));
    }
    Ok(normalize_model(model))
}

fn price_row(model: String, price: &super::ModelPrice, source: LlmPriceSource) -> LlmModelPrice {
    LlmModelPrice {
        model,
        input_per_million_minor: price.input_per_million_minor,
        output_per_million_minor: price.output_per_million_minor,
        input_per_million_usd_nanos: price.input_per_million_nanos.to_string(),
        output_per_million_usd_nanos: price.output_per_million_nanos.to_string(),
        currency: USD.to_string(),
        source,
    }
}

fn precise_rate(name: &str, legacy_minor: i64, exact: Option<&str>) -> Result<i64, String> {
    let Some(exact) = exact else {
        return legacy_minor
            .checked_mul(NANOS_PER_MINOR)
            .ok_or_else(|| format!("{name} is too large"));
    };
    let nanos = exact
        .parse::<i64>()
        .map_err(|_| format!("{name} must be a non-negative decimal integer string"))?;
    if nanos < 0 {
        return Err(format!("{name} must be non-negative"));
    }
    if nanos / NANOS_PER_MINOR != legacy_minor {
        return Err(format!(
            "{name} must project to the supplied legacy minor-unit rate"
        ));
    }
    Ok(nanos)
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    crate::log_api_error(status, code, &message);
    let body = ApiError {
        code,
        message,
        retriable: matches!(code, ApiErrorCode::Internal),
        details: serde_json::Value::Null,
    };
    (status, Json(body)).into_response()
}
