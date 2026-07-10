//! LLM budget gate + token metering for the gateway proxy.
//!
//! Reserve-before / settle-after around the forward step: a request is
//! admitted only when its maximum possible cost fits under every
//! matching cap. Durable per-principal serialization prevents
//! concurrent requests from spending the same remaining budget.
//!
//! Budgets are financial family policies with
//! [`tl_core::SpendMeter::LlmUsage`] — same registry, same window math,
//! same pure verdict fn as `/v1/financial/actions`; only the spend-sum
//! source differs (`llm_usage_events`, not the ledger).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use tl_core::{SpendMeter, USD};
use tl_policy::{FamilyPolicy, FinancialPolicy};

use crate::auth::WorkspaceKeyContext;
use crate::budget_alerts::{window_starts, BudgetAlertRuntime};
use crate::llm_usage::{
    LlmBudgetCapsNanos, LlmBudgetWindow, RecordLlmUsageEvent, ReserveLlmBudget,
    ReserveLlmBudgetOutcome,
};
use crate::AppState;

use super::errors::api_error_response;

#[derive(Debug, Clone)]
pub(super) struct LlmBudgetReservation {
    request_id: String,
    reserved_nanos: i64,
    price: crate::llm_pricing::ModelPrice,
}

/// Reserve the request's maximum possible provider cost before any
/// provider traffic. `Ok(None)` means no matching budget (or no runtime
/// principal); errors fail closed with a provider-compatible envelope.
#[allow(clippy::result_large_err)]
pub(super) async fn reserve_llm_budget(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    key: Option<&WorkspaceKeyContext>,
    gateway_request_id: &str,
    request: &Value,
) -> Result<Option<LlmBudgetReservation>, Response> {
    let Some(key) = key else {
        tracing::debug!(
            workspace_id,
            "gateway budget skipped: no workspace key context (internal key or JWT)"
        );
        return Ok(None);
    };
    let principal = principal_for(key);

    let families = match app
        .policy_store
        .list_enabled_families(workspace_id, environment_id)
        .await
    {
        Ok(families) => families,
        Err(error) => {
            tracing::error!(workspace_id, error = %error, "gateway budget policy load failed");
            return Err(api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load budget policies".into(),
            ));
        }
    };
    let budgets = families
        .iter()
        .filter_map(|family| match family.as_ref() {
            FamilyPolicy::Financial(financial)
                if llm_budget_policy_matches(financial, &principal) =>
            {
                Some(financial)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if budgets.is_empty() {
        return Ok(None);
    }

    let now = Utc::now();
    let Some((day_start, week_start, month_start)) = window_starts(now) else {
        return Err(api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to compute budget windows".into(),
        ));
    };
    let model = request
        .get("model")
        .and_then(Value::as_str)
        .filter(|model| !model.trim().is_empty())
        .ok_or_else(|| bounded_request_error("model is required when an LLM budget is active"))?;
    let max_tokens = bounded_output_tokens(request).ok_or_else(|| {
        bounded_request_error(
            "max_tokens or max_completion_tokens must be a positive integer when an LLM budget is active",
        )
    })?;
    let price =
        crate::llm_pricing::model_price(app.llm_pricing_store.as_ref(), workspace_id, model)
            .await
            .ok_or_else(|| pricing_unavailable_response(model))?;
    let input_token_upper_bound = serde_json::to_vec(request)
        .map(|bytes| {
            i64::try_from(bytes.len())
                .unwrap_or(i64::MAX)
                .saturating_add(64)
        })
        .map_err(|_| bounded_request_error("provider request could not be priced"))?;
    let reserved_nanos = crate::llm_pricing::cost_nanos(price, input_token_upper_bound, max_tokens);
    let caps = tightest_caps_nanos(&budgets);
    let outcome = app
        .llm_usage_store
        .reserve_budget(
            workspace_id,
            ReserveLlmBudget {
                request_id: gateway_request_id.to_string(),
                principal_id: principal.clone(),
                api_key_id: key.api_key_id.clone(),
                currency: USD.to_string(),
                reserved_nanos,
                caps,
                day_start,
                week_start,
                month_start,
                now,
            },
        )
        .await
        .map_err(|error| {
            tracing::error!(workspace_id, principal, error = %error, "gateway budget reservation failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to reserve LLM budget".into(),
            )
        })?;
    match outcome {
        ReserveLlmBudgetOutcome::Reserved => Ok(Some(LlmBudgetReservation {
            request_id: gateway_request_id.to_string(),
            reserved_nanos,
            price,
        })),
        ReserveLlmBudgetOutcome::Exceeded {
            window,
            cap_nanos,
            committed_nanos,
            requested_nanos,
        } => Err(budget_exceeded_response(
            &principal,
            window,
            cap_nanos,
            committed_nanos,
            requested_nanos,
        )),
    }
}

/// Post-response metering and reservation settlement. Never fails the
/// response — the upstream call already happened, so every error path
/// logs and leaves the conservative reservation in place.
///
/// The gateway buffers the upstream response (SSE is synthesized), so
/// `usage` is present even for `stream: true` requests. If true
/// passthrough streaming is ever added, this must move to the stream
/// tail.
pub(super) struct MeterLlmUsage<'a> {
    pub workspace_id: &'a str,
    pub environment_id: &'a str,
    pub key: Option<&'a WorkspaceKeyContext>,
    pub route_id: &'a str,
    pub gateway_request_id: &'a str,
    pub reservation: Option<&'a LlmBudgetReservation>,
    pub request: &'a Value,
    pub provider_response: &'a Value,
}

pub(super) async fn meter_llm_usage(app: &AppState, metering: MeterLlmUsage<'_>) {
    let MeterLlmUsage {
        workspace_id,
        environment_id,
        key,
        route_id,
        gateway_request_id,
        reservation,
        request,
        provider_response,
    } = metering;
    let Some(key) = key else {
        tracing::debug!(
            workspace_id,
            "gateway metering skipped: no workspace key context (internal key or JWT)"
        );
        return;
    };
    let principal = principal_for(key);

    let model = provider_response
        .get("model")
        .and_then(Value::as_str)
        .or_else(|| request.get("model").and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_string();
    let (prompt_tokens, completion_tokens) = match provider_response.get("usage") {
        Some(usage) => (
            usage
                .get("prompt_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            usage
                .get("completion_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        None => {
            tracing::warn!(
                workspace_id,
                route_id,
                model,
                "provider response has no usage field; metering zeros"
            );
            // Keep a hard-cap reservation active when actual usage is
            // unknowable. Releasing it could admit unrecorded spend.
            return;
        }
    };
    // Workspace prices first, built-in defaults as fallback; a store
    // read failure inside falls back to defaults — never fails the
    // response.
    let price = match reservation {
        Some(reservation) => Some(reservation.price),
        None => {
            crate::llm_pricing::model_price(app.llm_pricing_store.as_ref(), workspace_id, &model)
                .await
        }
    };
    let cost_nanos = match price {
        Some(price) => crate::llm_pricing::cost_nanos(price, prompt_tokens, completion_tokens),
        None if reservation.is_none() => {
            tracing::warn!(
                workspace_id,
                route_id,
                model,
                "no price for model; metering tokens with cost 0"
            );
            0
        }
        None => {
            tracing::error!(
                workspace_id,
                route_id,
                model,
                "reserved model price unavailable at settlement"
            );
            return;
        }
    };
    if let Some(reservation) = reservation {
        if cost_nanos > reservation.reserved_nanos {
            tracing::error!(
                workspace_id,
                route_id,
                gateway_request_id,
                reserved_nanos = reservation.reserved_nanos,
                actual_nanos = cost_nanos,
                "provider usage exceeded the preflight maximum reservation"
            );
        }
    }
    let cost_minor = cost_nanos / crate::llm_pricing::NANOS_PER_MINOR;

    let event = RecordLlmUsageEvent {
        principal_id: principal.clone(),
        api_key_id: key.api_key_id.clone(),
        // Raw model string preserved — pricing normalization is
        // lookup-only.
        model,
        prompt_tokens,
        completion_tokens,
        cost_minor,
        cost_nanos,
        currency: USD.to_string(),
        request_id: gateway_request_id.to_string(),
        metadata: json!({ "route_id": route_id }),
    };
    let result = match reservation {
        Some(reservation) => {
            app.llm_usage_store
                .settle_budget(workspace_id, &reservation.request_id, event)
                .await
        }
        None => app.llm_usage_store.insert_event(workspace_id, event).await,
    };
    if let Err(error) = result {
        tracing::error!(
            workspace_id,
            route_id,
            gateway_request_id,
            error = %error,
            "failed to record llm usage event; response returned anyway"
        );
        return;
    }

    // Budget alert thresholds are checked right after the usage event
    // lands. Same non-failing contract as the metering itself.
    evaluate_llm_budget_alerts(app, workspace_id, environment_id, &principal).await;
}

pub(super) async fn release_llm_budget(
    app: &AppState,
    workspace_id: &str,
    reservation: Option<&LlmBudgetReservation>,
) {
    let Some(reservation) = reservation else {
        return;
    };
    if let Err(error) = app
        .llm_usage_store
        .release_budget(workspace_id, &reservation.request_id)
        .await
    {
        tracing::error!(
            workspace_id,
            request_id = %reservation.request_id,
            error = %error,
            "failed to release unused LLM budget reservation"
        );
    }
}

/// Spend-time budget alert hook for the LLM metering path: the shared
/// evaluator with the LLM spend source (`net_llm_spend_minor`) and the
/// LLM budget policy selector. Infallible by design.
async fn evaluate_llm_budget_alerts(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    principal: &str,
) {
    let runtime = BudgetAlertRuntime {
        store: app.budget_alert_store.clone(),
        settings: app.settings_store.clone(),
        delivery_tx: app.budget_alert_tx.clone(),
    };
    crate::budget_alerts::evaluate_spend_alerts(
        &runtime,
        app.policy_store.as_ref(),
        workspace_id,
        environment_id,
        principal,
        USD,
        |financial| llm_budget_policy_matches(financial, principal),
        |window_start, now| async move {
            app.llm_usage_store
                .net_llm_spend_minor(workspace_id, principal, USD, window_start, now)
                .await
                .map_err(|error| error.to_string())
        },
    )
    .await;
}

/// Key without a bound principal → the key id itself is the principal,
/// so per-key budgets still work.
fn principal_for(key: &WorkspaceKeyContext) -> String {
    key.principal_id
        .clone()
        .unwrap_or_else(|| key.api_key_id.clone())
}

/// A financial policy is an LLM budget for this principal when its
/// `meter` is `llm_usage`. Mirrors `tl_engine::financial_matches` for
/// the selectors that exist here: `agents` matches the principal,
/// `currencies` must admit USD. `action_kinds`/`rails` describe typed
/// payment actions and are rejected at creation for this meter;
/// `when.operations` is reserved for scoping operations *within* the
/// meter (e.g. embeddings vs chat) and is not consulted yet.
fn llm_budget_policy_matches(financial: &FinancialPolicy, principal: &str) -> bool {
    if financial.meter != SpendMeter::LlmUsage {
        return false;
    }
    let when = &financial.when;
    if !when.agents.is_empty() && !when.agents.iter().any(|agent| agent == principal) {
        return false;
    }
    if !when.currencies.is_empty()
        && !when
            .currencies
            .iter()
            .any(|currency| currency.eq_ignore_ascii_case(USD))
    {
        return false;
    }
    financial.daily_minor.is_some()
        || financial.weekly_minor.is_some()
        || financial.monthly_minor.is_some()
}

fn tightest_caps_nanos(budgets: &[&FinancialPolicy]) -> LlmBudgetCapsNanos {
    let nanos = |minor: i64| minor.saturating_mul(crate::llm_pricing::NANOS_PER_MINOR);
    let mut caps = LlmBudgetCapsNanos::default();
    for budget in budgets {
        caps.daily = min_cap(caps.daily, budget.daily_minor.map(nanos));
        caps.weekly = min_cap(caps.weekly, budget.weekly_minor.map(nanos));
        caps.monthly = min_cap(caps.monthly, budget.monthly_minor.map(nanos));
    }
    caps
}

fn min_cap(current: Option<i64>, candidate: Option<i64>) -> Option<i64> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.min(candidate)),
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}

fn bounded_output_tokens(request: &Value) -> Option<i64> {
    request
        .get("max_completion_tokens")
        .or_else(|| request.get("max_tokens"))
        .and_then(Value::as_i64)
        .filter(|tokens| *tokens > 0)
}

fn bounded_request_error(message: &str) -> Response {
    openai_error_response(
        StatusCode::BAD_REQUEST,
        message,
        "invalid_request_error",
        "budget_max_tokens_required",
    )
}

fn pricing_unavailable_response(model: &str) -> Response {
    openai_error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        &format!("no trusted price is configured for model `{model}`; the provider was not called"),
        "configuration_error",
        "pricing_unavailable",
    )
}

fn openai_error_response(
    status: StatusCode,
    message: &str,
    error_type: &str,
    code: &str,
) -> Response {
    (
        status,
        Json(json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": code,
            }
        })),
    )
        .into_response()
}

/// OpenAI-style error envelope with HTTP 429 so openai-sdk clients
/// raise a typed `insufficient_quota` error instead of a parse failure.
fn budget_exceeded_response(
    principal: &str,
    window: LlmBudgetWindow,
    cap_nanos: i64,
    committed_nanos: i64,
    requested_nanos: i64,
) -> Response {
    let remaining_nanos = cap_nanos.saturating_sub(committed_nanos).max(0);
    let window = match window {
        LlmBudgetWindow::Day => "daily",
        LlmBudgetWindow::Week => "weekly",
        LlmBudgetWindow::Month => "monthly",
    };
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": {
                "message": format!(
                    "{window} budget would be exceeded for principal `{principal}`: next request maximum ${:.6}, remaining ${:.6}",
                    nanos_to_dollars(requested_nanos),
                    nanos_to_dollars(remaining_nanos),
                ),
                "type": "insufficient_quota",
                "code": "budget_exceeded",
                "details": {
                    "window": window,
                    "cap_nanos": cap_nanos,
                    "committed_nanos": committed_nanos,
                    "requested_nanos": requested_nanos,
                    "remaining_nanos": remaining_nanos,
                }
            }
        })),
    )
        .into_response()
}

fn nanos_to_dollars(nanos: i64) -> f64 {
    nanos as f64 / 1_000_000_000_f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone};

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, s).single().unwrap()
    }

    #[test]
    fn monday_is_its_own_week_start() {
        // 2026-07-06 is a Monday.
        let (day, week, month) = window_starts(utc(2026, 7, 6, 15, 30, 9)).unwrap();
        assert_eq!(day, utc(2026, 7, 6, 0, 0, 0));
        assert_eq!(week, day);
        assert_eq!(month, utc(2026, 7, 1, 0, 0, 0));
    }

    #[test]
    fn sunday_belongs_to_the_previous_monday() {
        // 2026-07-05 is a Sunday; its week began Monday 2026-06-29.
        let (day, week, month) = window_starts(utc(2026, 7, 5, 23, 59, 59)).unwrap();
        assert_eq!(day, utc(2026, 7, 5, 0, 0, 0));
        assert_eq!(week, utc(2026, 6, 29, 0, 0, 0));
        assert_eq!(month, utc(2026, 7, 1, 0, 0, 0));
    }

    #[test]
    fn month_rollover_resets_day_and_month_but_not_week() {
        // 2026-03-01 is a Sunday: the month window starts that day, but
        // the week window still reaches back into February.
        let (day, week, month) = window_starts(utc(2026, 3, 1, 0, 0, 1)).unwrap();
        assert_eq!(day, utc(2026, 3, 1, 0, 0, 0));
        assert_eq!(week, utc(2026, 2, 23, 0, 0, 0));
        assert_eq!(month, day);
    }
}
