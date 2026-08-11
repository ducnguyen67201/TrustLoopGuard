//! LLM budget gate + token metering for the gateway proxy.
//!
//! Reserve-before / settle-after around the forward step. Bounded
//! requests are admitted only when their maximum possible cost fits
//! under every matching cap. Requests without a caller-supplied output
//! bound are soft-admitted while spend remains below the cap, then
//! settled to actual provider usage; one such request can overshoot.
//!
//! Budgets are financial family policies with
//! [`tl_core::SpendMeter::LlmUsage`] — same registry, same window math,
//! same pure effect composition as `/v1/financial/actions`; only the spend-sum
//! source differs (`llm_usage_events`, not the ledger).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use tl_core::{
    CreateRunEventRequest, LlmUsageKind, RunBudgetWindowSnapshot, RunEventKind,
    RunLlmBudgetDecision, RunProviderUsage, SpendMeter, USD,
};
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
    soft_admission: bool,
    decision: RunLlmBudgetDecision,
}

/// A one-nano marker makes an unbounded request fail once committed
/// spend has reached the cap without fabricating a maximum cost.
const SOFT_ADMISSION_MARKER_NANOS: i64 = 1;

/// Reserve provider cost before any provider traffic. Bounded requests
/// reserve their maximum; unbounded requests use a one-nano admission
/// marker and may overshoot once before future requests are denied.
/// `Ok(None)` means no matching budget (or no runtime principal).
#[allow(clippy::result_large_err)]
pub(super) async fn reserve_llm_budget(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    key: Option<&WorkspaceKeyContext>,
    gateway_request_id: &str,
    request: &Value,
    run_id: Option<&str>,
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
        record_budget_decision(
            app,
            workspace_id,
            environment_id,
            run_id,
            gateway_request_id,
            &RunLlmBudgetDecision {
                principal_id: principal,
                status: "not_configured".to_string(),
                currency: USD.to_string(),
                governing_window: None,
                requested_usd_nanos: None,
                actual_usd_nanos: None,
                windows: vec![],
            },
        )
        .await;
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
        .ok_or_else(|| budget_request_error("model is required when an LLM budget is active"))?;
    let max_tokens = bounded_output_tokens(request);
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
        .map_err(|_| budget_request_error("provider request could not be priced"))?;
    let soft_admission = max_tokens.is_none();
    let reserved_nanos = max_tokens.map_or(SOFT_ADMISSION_MARKER_NANOS, |max_tokens| {
        crate::llm_pricing::cost_nanos(price, input_token_upper_bound, max_tokens)
    });
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
        ReserveLlmBudgetOutcome::Reserved { snapshots } => {
            let decision = RunLlmBudgetDecision {
                principal_id: principal,
                status: if soft_admission {
                    "soft_admitted".to_string()
                } else {
                    "reserved".to_string()
                },
                currency: USD.to_string(),
                governing_window: None,
                requested_usd_nanos: (!soft_admission).then(|| reserved_nanos.to_string()),
                actual_usd_nanos: None,
                windows: run_budget_snapshots(snapshots, soft_admission),
            };
            record_budget_decision(
                app,
                workspace_id,
                environment_id,
                run_id,
                gateway_request_id,
                &decision,
            )
            .await;
            Ok(Some(LlmBudgetReservation {
                request_id: gateway_request_id.to_string(),
                reserved_nanos,
                price,
                soft_admission,
                decision,
            }))
        }
        ReserveLlmBudgetOutcome::Exceeded {
            window,
            cap_nanos,
            committed_nanos,
            requested_nanos,
            snapshots,
        } => {
            let decision = RunLlmBudgetDecision {
                principal_id: principal.clone(),
                status: "denied".to_string(),
                currency: USD.to_string(),
                governing_window: Some(window_label(window).to_string()),
                requested_usd_nanos: (!soft_admission).then(|| requested_nanos.to_string()),
                actual_usd_nanos: None,
                windows: run_budget_snapshots(snapshots, soft_admission),
            };
            record_budget_decision(
                app,
                workspace_id,
                environment_id,
                run_id,
                gateway_request_id,
                &decision,
            )
            .await;
            Err(if soft_admission {
                soft_budget_exceeded_response(&principal, window, cap_nanos, committed_nanos)
            } else {
                budget_exceeded_response(
                    &principal,
                    window,
                    cap_nanos,
                    committed_nanos,
                    requested_nanos,
                )
            })
        }
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
    pub provider_connection_id: &'a str,
    pub attempt: u32,
    pub gateway_request_id: &'a str,
    pub reservation: Option<&'a LlmBudgetReservation>,
    pub request: &'a Value,
    pub provider_response: &'a Value,
    pub provider: &'a str,
    pub latency_ms: u64,
    pub run_id: Option<&'a str>,
}

pub(super) async fn meter_llm_usage(
    app: &AppState,
    metering: MeterLlmUsage<'_>,
) -> RunProviderUsage {
    let MeterLlmUsage {
        workspace_id,
        environment_id,
        key,
        route_id,
        provider_connection_id,
        attempt,
        gateway_request_id,
        reservation,
        request,
        provider_response,
        provider,
        latency_ms,
        run_id,
    } = metering;

    let model = provider_response
        .get("model")
        .and_then(Value::as_str)
        .or_else(|| request.get("model").and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_string();
    let usage = provider_response.get("usage");
    let token_counts = usage.map(|usage| {
        (
            usage
                .get("prompt_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            usage
                .get("completion_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        )
    });
    if usage.is_none() {
        tracing::warn!(
            workspace_id,
            route_id,
            model,
            "provider response has no usage field; metering zeros"
        );
        // Keep a hard-cap reservation active when actual usage is
        // unknowable. Releasing it could admit unrecorded spend.
    }
    let (prompt_tokens, completion_tokens) = token_counts.unwrap_or((0, 0));
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
            0
        }
    };
    if let Some(reservation) = reservation {
        if !reservation.soft_admission && cost_nanos > reservation.reserved_nanos {
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

    let evidence = RunProviderUsage {
        gateway_request_id: gateway_request_id.to_string(),
        route_id: route_id.to_string(),
        attempt,
        provider_connection_id: provider_connection_id.to_string(),
        provider: provider.to_string(),
        model: model.clone(),
        provider_response_id: provider_response
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string),
        status: "succeeded".to_string(),
        failure_code: None,
        prompt_tokens: usage.map(|_| prompt_tokens),
        completion_tokens: usage.map(|_| completion_tokens),
        total_tokens: usage.map(|_| prompt_tokens.saturating_add(completion_tokens)),
        latency_ms,
        estimated_cost_usd_nanos: price.map(|_| cost_nanos.to_string()),
        input_rate_usd_per_million_nanos: price
            .map(|price| price.input_per_million_nanos.to_string()),
        output_rate_usd_per_million_nanos: price
            .map(|price| price.output_per_million_nanos.to_string()),
    };

    let Some(key) = key else {
        tracing::debug!(
            workspace_id,
            "gateway metering skipped: no workspace key context (internal key or JWT)"
        );
        return evidence;
    };
    if usage.is_none() {
        if let Some(reservation) = reservation {
            let mut decision = reservation.decision.clone();
            decision.status = "usage_unknown".to_string();
            record_budget_decision(
                app,
                workspace_id,
                environment_id,
                run_id,
                gateway_request_id,
                &decision,
            )
            .await;
        }
        return evidence;
    }
    let principal = principal_for(key);

    let event = RecordLlmUsageEvent {
        principal_id: principal.clone(),
        api_key_id: key.api_key_id.clone(),
        kind: LlmUsageKind::CustomerInference,
        // Raw model string preserved — pricing normalization is
        // lookup-only.
        model,
        prompt_tokens,
        completion_tokens,
        cost_minor,
        cost_nanos,
        currency: USD.to_string(),
        request_id: gateway_request_id.to_string(),
        metadata: json!({
            "route_id": route_id,
            "provider": provider,
            "provider_response_id": evidence.provider_response_id,
            "latency_ms": latency_ms,
            "input_rate_usd_per_million_nanos": evidence.input_rate_usd_per_million_nanos,
            "output_rate_usd_per_million_nanos": evidence.output_rate_usd_per_million_nanos,
        }),
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
        return evidence;
    }

    if let Some(reservation) = reservation {
        let mut decision = reservation.decision.clone();
        decision.status = if reservation.soft_admission {
            "soft_settled".to_string()
        } else {
            "settled".to_string()
        };
        decision.actual_usd_nanos = Some(cost_nanos.to_string());
        decision.windows.iter_mut().for_each(|window| {
            let cap = window.cap_usd_nanos.parse::<i64>().unwrap_or(0);
            let committed = window
                .committed_before_usd_nanos
                .parse::<i64>()
                .unwrap_or(0)
                .saturating_add(window.reserved_before_usd_nanos.parse::<i64>().unwrap_or(0));
            window.remaining_after_usd_nanos = cap
                .saturating_sub(committed)
                .saturating_sub(cost_nanos)
                .max(0)
                .to_string();
        });
        record_budget_decision(
            app,
            workspace_id,
            environment_id,
            run_id,
            gateway_request_id,
            &decision,
        )
        .await;
    }

    // Budget alert thresholds are checked right after the usage event
    // lands. Same non-failing contract as the metering itself.
    evaluate_llm_budget_alerts(app, workspace_id, environment_id, &principal).await;
    evidence
}

pub(super) async fn release_llm_budget(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
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
    let mut decision = reservation.decision.clone();
    decision.status = "released".to_string();
    record_budget_decision(
        app,
        workspace_id,
        environment_id,
        run_id,
        &reservation.request_id,
        &decision,
    )
    .await;
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
        crate::budget_alerts::SpendAlertEvaluation {
            runtime: &runtime,
            policy_store: app.policy_store.as_ref(),
            workspace_id,
            environment_id,
            principal_id: principal,
            currency: USD,
            meter: SpendMeter::LlmUsage,
        },
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

fn run_budget_snapshots(
    snapshots: Vec<crate::llm_usage::LlmBudgetWindowSnapshot>,
    soft_admission: bool,
) -> Vec<RunBudgetWindowSnapshot> {
    snapshots
        .into_iter()
        .map(|snapshot| RunBudgetWindowSnapshot {
            window: window_label(snapshot.window).to_string(),
            cap_usd_nanos: snapshot.cap_nanos.to_string(),
            committed_before_usd_nanos: snapshot.spent_nanos.to_string(),
            reserved_before_usd_nanos: snapshot.active_reserved_nanos.to_string(),
            requested_usd_nanos: if soft_admission {
                "0".to_string()
            } else {
                snapshot.requested_nanos.to_string()
            },
            remaining_after_usd_nanos: snapshot
                .cap_nanos
                .saturating_sub(snapshot.committed_nanos)
                .saturating_sub(if soft_admission {
                    0
                } else {
                    snapshot.requested_nanos
                })
                .max(0)
                .to_string(),
        })
        .collect()
}

fn window_label(window: LlmBudgetWindow) -> &'static str {
    match window {
        LlmBudgetWindow::Day => "day",
        LlmBudgetWindow::Week => "week",
        LlmBudgetWindow::Month => "month",
    }
}

async fn record_budget_decision(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    run_id: Option<&str>,
    gateway_request_id: &str,
    decision: &RunLlmBudgetDecision,
) {
    let Some(run_id) = run_id else { return };
    let event = CreateRunEventRequest {
        agent_id: None,
        kind: RunEventKind::SystemEvent,
        sequence: None,
        label: Some("LLM spending cap".to_string()),
        input_summary: None,
        output_summary: None,
        metadata: json!({
            "integration_mode": "gateway",
            "gateway_request_id": gateway_request_id,
            "evidence_kind": "llm_budget_decision",
            "budget_decision": decision,
        }),
        occurred_at: None,
    };
    if let Err(error) = app
        .run_store
        .create_event(workspace_id, environment_id, run_id, event)
        .await
    {
        tracing::warn!(workspace_id, environment_id, run_id, error = %error, "could not record gateway budget evidence");
    }
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

fn budget_request_error(message: &str) -> Response {
    openai_error_response(
        StatusCode::BAD_REQUEST,
        message,
        "invalid_request_error",
        "budget_request_invalid",
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

fn soft_budget_exceeded_response(
    principal: &str,
    window: LlmBudgetWindow,
    cap_nanos: i64,
    committed_nanos: i64,
) -> Response {
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
                    "{window} budget already reached for principal `{principal}`; the unbounded request was not sent"
                ),
                "type": "insufficient_quota",
                "code": "budget_exceeded",
                "details": {
                    "window": window,
                    "cap_nanos": cap_nanos,
                    "committed_nanos": committed_nanos,
                    "requested_nanos": Value::Null,
                    "remaining_nanos": cap_nanos.saturating_sub(committed_nanos).max(0),
                    "admission_mode": "soft"
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
