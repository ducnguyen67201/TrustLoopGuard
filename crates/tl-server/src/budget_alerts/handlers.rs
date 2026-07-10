//! CRUD for `/v1/financial/budget-alerts` (+ firing history).
//!
//! Mutations are Owner/Admin-gated (ADMIN_VALIDATE pattern shared with
//! workspace settings); reads are open to workspace keys so agents and
//! dashboards can render alert state.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::SpendMeter;
use tl_core::{
    ApiErrorCode, BudgetAlertConfig, BudgetAlertConfigListResponse, BudgetAlertFiringListResponse,
    BudgetAlertThresholdType, BudgetAlertWindow, CreateBudgetAlertConfigRequest,
    UpdateBudgetAlertConfigRequest, DEFAULT_ENVIRONMENT_ID,
};
use tl_policy::FamilyPolicy;

use crate::auth::{InternalServiceContext, WorkspaceKeyContext};
use crate::dashboard_admin::authorize_workspace_admin;
use crate::jwt::UserContext;
use crate::policies::PolicyStore;
use crate::team::TeamStore;

use super::{window_label, BudgetAlertStore, BudgetAlertStoreError};

const MAX_NAME_CHARS: usize = 120;

#[derive(Clone)]
pub struct BudgetAlertApiState {
    pub store: Arc<dyn BudgetAlertStore>,
    pub policy_store: Arc<dyn PolicyStore>,
    pub team_store: Arc<dyn TeamStore>,
}

/// `POST /v1/financial/budget-alerts` — create an alert threshold.
#[utoipa::path(
    post,
    path = "/v1/financial/budget-alerts",
    tag = "budget-alerts",
    request_body = CreateBudgetAlertConfigRequest,
    responses(
        (status = 201, description = "Budget alert created", body = BudgetAlertConfig),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid credentials", body = ApiError),
        (status = 403, description = "Caller cannot manage budget alerts for this workspace", body = ApiError),
        (status = 409, description = "A budget alert with this name already exists", body = ApiError),
    ),
)]
pub async fn create_budget_alert(
    State(state): State<BudgetAlertApiState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<CreateBudgetAlertConfigRequest>,
) -> Response {
    let input = match validated_new_config(&req) {
        Ok(input) => input,
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
        "manage budget alerts",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    if let Err(response) = require_capped_scope(
        &state,
        &headers,
        &workspace_id,
        input.meter,
        input.window,
        input.principal_id.as_deref(),
    )
    .await
    {
        return response;
    }
    match state.store.create_config(&workspace_id, input).await {
        Ok(config) => (StatusCode::CREATED, Json(config)).into_response(),
        Err(error) => budget_alert_error_response(error),
    }
}

/// `GET /v1/financial/budget-alerts` — list configured alerts.
#[utoipa::path(
    get,
    path = "/v1/financial/budget-alerts",
    tag = "budget-alerts",
    responses(
        (status = 200, description = "Budget alert configs", body = BudgetAlertConfigListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_budget_alerts(
    State(state): State<BudgetAlertApiState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list_configs(&workspace_id).await {
        Ok(configs) => Json(BudgetAlertConfigListResponse { configs }).into_response(),
        Err(error) => budget_alert_error_response(error),
    }
}

/// `PATCH /v1/financial/budget-alerts/{id}` — partial update; absent
/// fields are left unchanged.
#[utoipa::path(
    patch,
    path = "/v1/financial/budget-alerts/{id}",
    tag = "budget-alerts",
    params(("id" = String, Path, description = "Budget alert config id")),
    request_body = UpdateBudgetAlertConfigRequest,
    responses(
        (status = 200, description = "Updated budget alert", body = BudgetAlertConfig),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid credentials", body = ApiError),
        (status = 403, description = "Caller cannot manage budget alerts for this workspace", body = ApiError),
        (status = 404, description = "Budget alert not found", body = ApiError),
        (status = 409, description = "A budget alert with this name already exists", body = ApiError),
    ),
)]
pub async fn update_budget_alert(
    State(state): State<BudgetAlertApiState>,
    Path(config_id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<UpdateBudgetAlertConfigRequest>,
) -> Response {
    let update = match validated_update(&req) {
        Ok(update) => update,
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
        "manage budget alerts",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    // Validate the *merged* threshold semantics and cap scope, so a
    // window/type flip cannot leave a config no cap will ever satisfy.
    let current = match state.store.get_config(&workspace_id, &config_id).await {
        Ok(config) => config,
        Err(error) => return budget_alert_error_response(error),
    };
    let merged_type = update.threshold_type.unwrap_or(current.threshold_type);
    let merged_value = update.threshold_value.unwrap_or(current.threshold_value);
    if let Err(message) = validate_threshold(merged_type, merged_value) {
        return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message);
    }
    let merged_window = update.window.unwrap_or(current.window);
    let merged_meter = update.meter.unwrap_or(current.meter);
    let merged_principal = update.principal_id.clone().or(current.principal_id.clone());
    if let Err(response) = require_capped_scope(
        &state,
        &headers,
        &workspace_id,
        merged_meter,
        merged_window,
        merged_principal.as_deref(),
    )
    .await
    {
        return response;
    }
    match state
        .store
        .update_config(&workspace_id, &config_id, update)
        .await
    {
        Ok(config) => Json(config).into_response(),
        Err(error) => budget_alert_error_response(error),
    }
}

/// `DELETE /v1/financial/budget-alerts/{id}` — remove a config (its
/// firing history goes with it).
#[utoipa::path(
    delete,
    path = "/v1/financial/budget-alerts/{id}",
    tag = "budget-alerts",
    params(("id" = String, Path, description = "Budget alert config id")),
    responses(
        (status = 204, description = "Budget alert deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiError),
        (status = 403, description = "Caller cannot manage budget alerts for this workspace", body = ApiError),
        (status = 404, description = "Budget alert not found", body = ApiError),
    ),
)]
pub async fn delete_budget_alert(
    State(state): State<BudgetAlertApiState>,
    Path(config_id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "manage budget alerts",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.store.delete_config(&workspace_id, &config_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => budget_alert_error_response(error),
    }
}

/// `GET /v1/financial/budget-alerts/{id}/firings` — firing history for
/// one config, newest first.
#[utoipa::path(
    get,
    path = "/v1/financial/budget-alerts/{id}/firings",
    tag = "budget-alerts",
    params(("id" = String, Path, description = "Budget alert config id")),
    responses(
        (status = 200, description = "Budget alert firings", body = BudgetAlertFiringListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Budget alert not found", body = ApiError),
    ),
)]
pub async fn list_budget_alert_firings(
    State(state): State<BudgetAlertApiState>,
    Path(config_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    if let Err(error) = state.store.get_config(&workspace_id, &config_id).await {
        return budget_alert_error_response(error);
    }
    match state.store.list_firings(&workspace_id, &config_id).await {
        Ok(firings) => Json(BudgetAlertFiringListResponse { firings }).into_response(),
        Err(error) => budget_alert_error_response(error),
    }
}

/// Normalize + validate the create request: trimmed name, cleaned
/// optionals, `enabled` defaulted to `true`.
fn validated_new_config(
    req: &CreateBudgetAlertConfigRequest,
) -> Result<CreateBudgetAlertConfigRequest, String> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err("budget alert name is required".into());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "budget alert name must be at most {MAX_NAME_CHARS} characters"
        ));
    }
    validate_threshold(req.threshold_type, req.threshold_value)?;
    let webhook_url = validated_webhook_url(req.webhook_url.as_deref())?;
    Ok(CreateBudgetAlertConfigRequest {
        name: name.to_string(),
        meter: req.meter,
        window: req.window,
        principal_id: clean_optional(req.principal_id.as_deref()),
        threshold_type: req.threshold_type,
        threshold_value: req.threshold_value,
        webhook_url,
        enabled: Some(req.enabled.unwrap_or(true)),
    })
}

fn validated_update(
    req: &UpdateBudgetAlertConfigRequest,
) -> Result<UpdateBudgetAlertConfigRequest, String> {
    let name = match req.name.as_deref().map(str::trim) {
        Some("") => return Err("budget alert name is required".into()),
        Some(name) if name.chars().count() > MAX_NAME_CHARS => {
            return Err(format!(
                "budget alert name must be at most {MAX_NAME_CHARS} characters"
            ))
        }
        Some(name) => Some(name.to_string()),
        None => None,
    };
    let webhook_url = match req.webhook_url.as_deref() {
        Some(url) => validated_webhook_url(Some(url))?,
        None => None,
    };
    Ok(UpdateBudgetAlertConfigRequest {
        name,
        meter: req.meter,
        window: req.window,
        principal_id: clean_optional(req.principal_id.as_deref()),
        threshold_type: req.threshold_type,
        threshold_value: req.threshold_value,
        webhook_url,
        enabled: req.enabled,
    })
}

fn validate_threshold(
    threshold_type: BudgetAlertThresholdType,
    threshold_value: i64,
) -> Result<(), String> {
    match threshold_type {
        BudgetAlertThresholdType::Percent => {
            if !(1..=100).contains(&threshold_value) {
                return Err("percent threshold must be between 1 and 100".into());
            }
        }
        BudgetAlertThresholdType::Absolute => {
            if threshold_value < 0 {
                return Err("absolute threshold must be zero or more".into());
            }
        }
    }
    Ok(())
}

fn validated_webhook_url(value: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let parsed =
        url::Url::parse(raw).map_err(|e| format!("webhook_url must be a valid URL: {e}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("webhook_url must use http or https".into());
    }
    Ok(Some(raw.to_string()))
}

/// Decision 2: a threshold needs a cap to measure against. Reject
/// configs whose (window, principal) scope has no capped financial
/// policy — resolved through the same policy registry the hard limits
/// read.
async fn require_capped_scope(
    state: &BudgetAlertApiState,
    headers: &HeaderMap,
    workspace_id: &str,
    meter: SpendMeter,
    window: BudgetAlertWindow,
    principal_id: Option<&str>,
) -> Result<(), Response> {
    let environment_id = crate::environments::environment_id_from_headers(headers)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string());
    let families = state
        .policy_store
        .list_enabled_families(workspace_id, &environment_id)
        .await
        .map_err(|error| {
            tracing::error!(workspace_id, error = %error, "budget alert cap lookup failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "failed to resolve budget policies".to_string(),
            )
        })?;
    let has_cap = families.iter().any(|family| {
        let FamilyPolicy::Financial(financial) = family.as_ref() else {
            return false;
        };
        if financial.meter != meter {
            return false;
        }
        let cap = match window {
            BudgetAlertWindow::Day => financial.daily_minor,
            BudgetAlertWindow::Week => financial.weekly_minor,
            BudgetAlertWindow::Month => financial.monthly_minor,
        };
        if cap.is_none() {
            return false;
        }
        // Meter-aware reachability, mirroring the runtime matchers: an
        // `llm_usage` cap is only ever evaluated in USD (the metering
        // currency — see `llm_budget_policy_matches`), so a cap scoped
        // to other currencies can never fire this alert. `actions` caps
        // are matched per-action at spend time; the agents check below
        // is the only selector validation can assess up front.
        if financial.meter == SpendMeter::LlmUsage
            && !financial.when.currencies.is_empty()
            && !financial
                .when
                .currencies
                .iter()
                .any(|currency| currency.eq_ignore_ascii_case("USD"))
        {
            return false;
        }
        match principal_id {
            Some(principal) => {
                financial.when.agents.is_empty()
                    || financial.when.agents.iter().any(|agent| agent == principal)
            }
            None => true,
        }
    });
    if has_cap {
        Ok(())
    } else {
        Err(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            format!("no {} cap configured for this scope", window_label(window)),
        ))
    }
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn budget_alert_error_response(error: BudgetAlertStoreError) -> Response {
    let (status, code) = match &error {
        BudgetAlertStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        BudgetAlertStoreError::Conflict(_) => (StatusCode::CONFLICT, ApiErrorCode::Unprocessable),
        BudgetAlertStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        BudgetAlertStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    api_error_response(status, code, error.to_string())
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    crate::log_api_error(status, code, &message);
    let body = ApiError {
        code,
        message,
        retriable: matches!(code, ApiErrorCode::Internal | ApiErrorCode::Unavailable),
        details: serde_json::Value::Null,
    };
    (status, Json(body)).into_response()
}
