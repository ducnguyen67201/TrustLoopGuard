//! LLM budget gate + token metering for the gateway proxy.
//!
//! Check-before / record-after around the forward step: a request is
//! admitted iff the principal's spend-so-far is under every matching
//! cap at admission time; its own cost lands after the response.
//! `// ponytail: admission check races under concurrency; per-principal serialization if a customer needs exact caps`
//!
//! Budgets are financial family policies whose `when.operations`
//! contains [`tl_core::LLM_CHAT_OPERATION`] — same registry, same
//! window math, same pure verdict fn as `/v1/financial/actions`; only
//! the spend-sum source differs (`llm_usage_events`, not the ledger).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use tl_core::LLM_CHAT_OPERATION;
use tl_engine::financial_windowed_verdict;
use tl_policy::{FamilyPolicy, FinancialPolicy};

use crate::auth::WorkspaceKeyContext;
use crate::budget_alerts::{window_starts, BudgetAlertRuntime};
use crate::llm_usage::RecordLlmUsageEvent;
use crate::AppState;

use super::errors::api_error_response;

/// Budgets and metering are USD-denominated for v1; the price table is
/// USD minor units, so policies in other currencies never match.
const LLM_USAGE_CURRENCY: &str = "USD";

/// Pre-flight budget gate. `Some(response)` = deny (429, or 500 when
/// policy/spend state is unreadable — fail closed, mirroring content
/// enforcement). `None` = admitted.
///
/// Requests without a workspace-key principal (internal key, JWT) skip
/// the budget entirely: there is nobody to bill.
pub(super) async fn admit_llm_budget(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    key: Option<&WorkspaceKeyContext>,
) -> Option<Response> {
    let Some(key) = key else {
        tracing::debug!(
            workspace_id,
            "gateway budget skipped: no workspace key context (internal key or JWT)"
        );
        return None;
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
            return Some(api_error_response(
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
        return None;
    }

    let now = Utc::now();
    let (day_start, week_start, month_start) = window_starts(now)?;
    let mut sums = [0_i64; 3];
    for (slot, start) in [day_start, week_start, month_start].into_iter().enumerate() {
        match app
            .llm_usage_store
            .net_llm_spend_minor(workspace_id, &principal, LLM_USAGE_CURRENCY, start, now)
            .await
        {
            Ok(spent) => sums[slot] = spent,
            Err(error) => {
                tracing::error!(workspace_id, principal, error = %error, "gateway budget spend sum failed");
                return Some(api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to compute budget spend".into(),
                ));
            }
        }
    }
    let [spent_today, spent_week, spent_month] = sums;

    for financial in budgets {
        // Amount 1 = the smallest possible next spend, so the pure
        // verdict (`spent + amount > cap`) reads "admit iff spent <
        // cap": at-cap principals are denied, under-cap admitted. Any
        // breach verdict (block or escalate) denies at the gateway.
        if let Some((_, reason)) =
            financial_windowed_verdict(financial, spent_today, spent_week, spent_month, 1)
        {
            tracing::info!(
                workspace_id,
                principal,
                policy_id = %financial.id,
                spent_today,
                spent_week,
                spent_month,
                "gateway request denied: llm budget exhausted"
            );
            return Some(budget_exceeded_response(&principal, &reason));
        }
    }
    None
}

/// Post-response metering. Never fails the response — the upstream
/// call already happened, so every error path logs and returns.
///
/// The gateway buffers the upstream response (SSE is synthesized), so
/// `usage` is present even for `stream: true` requests. If true
/// passthrough streaming is ever added, this must move to the stream
/// tail.
pub(super) async fn meter_llm_usage(
    app: &AppState,
    workspace_id: &str,
    environment_id: &str,
    key: Option<&WorkspaceKeyContext>,
    route_id: &str,
    gateway_request_id: &str,
    request: &Value,
    provider_response: &Value,
) {
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
            (0, 0)
        }
    };
    let cost_minor = app
        .llm_pricing
        .cost_minor(&model, prompt_tokens, completion_tokens)
        .unwrap_or_else(|| {
            tracing::warn!(
                workspace_id,
                route_id,
                model,
                "no price for model; metering tokens with cost 0"
            );
            0
        });

    let event = RecordLlmUsageEvent {
        principal_id: principal.clone(),
        api_key_id: key.api_key_id.clone(),
        // Raw model string preserved — pricing normalization is
        // lookup-only.
        model,
        prompt_tokens,
        completion_tokens,
        cost_minor,
        currency: LLM_USAGE_CURRENCY.to_string(),
        request_id: gateway_request_id.to_string(),
        metadata: json!({ "route_id": route_id }),
    };
    if let Err(error) = app.llm_usage_store.insert_event(workspace_id, event).await {
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
        LLM_USAGE_CURRENCY,
        |financial| llm_budget_policy_matches(financial, principal),
        |window_start, now| async move {
            app.llm_usage_store
                .net_llm_spend_minor(
                    workspace_id,
                    principal,
                    LLM_USAGE_CURRENCY,
                    window_start,
                    now,
                )
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
/// `when.operations` carries the shared constant. Mirrors
/// `tl_engine::financial_matches` for the selectors that exist here:
/// `agents` matches the principal, `currencies` must admit USD.
/// `action_kinds`/`rails` describe typed payment actions and don't
/// apply to gateway calls, so they are ignored.
fn llm_budget_policy_matches(financial: &FinancialPolicy, principal: &str) -> bool {
    let when = &financial.when;
    if !when.operations.iter().any(|op| op == LLM_CHAT_OPERATION) {
        return false;
    }
    if !when.agents.is_empty() && !when.agents.iter().any(|agent| agent == principal) {
        return false;
    }
    if !when.currencies.is_empty()
        && !when
            .currencies
            .iter()
            .any(|currency| currency.eq_ignore_ascii_case(LLM_USAGE_CURRENCY))
    {
        return false;
    }
    financial.daily_minor.is_some()
        || financial.weekly_minor.is_some()
        || financial.monthly_minor.is_some()
}

/// OpenAI-style error envelope with HTTP 429 so openai-sdk clients
/// raise a typed `insufficient_quota` error instead of a parse failure.
fn budget_exceeded_response(principal: &str, reason: &str) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": {
                "message": format!("budget exhausted for principal `{principal}`: {reason}"),
                "type": "insufficient_quota",
                "code": "budget_exceeded",
            }
        })),
    )
        .into_response()
}
