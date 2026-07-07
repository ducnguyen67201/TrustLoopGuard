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
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde_json::{json, Value};
use tl_core::LLM_CHAT_OPERATION;
use tl_engine::financial_windowed_verdict;
use tl_policy::{FamilyPolicy, FinancialPolicy};

use crate::auth::WorkspaceKeyContext;
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
    // Three independent reads on the hot path — run them concurrently.
    let spend = |start| {
        app.llm_usage_store.net_llm_spend_minor(
            workspace_id,
            &principal,
            LLM_USAGE_CURRENCY,
            start,
            now,
        )
    };
    let (spent_today, spent_week, spent_month) = match tokio::try_join!(
        spend(day_start),
        spend(week_start),
        spend(month_start)
    ) {
        Ok(sums) => sums,
        Err(error) => {
            tracing::error!(workspace_id, principal, error = %error, "gateway budget spend sum failed");
            return Some(api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to compute budget spend".into(),
            ));
        }
    };

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
        principal_id: principal,
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

/// Window boundaries mirroring `evaluate_ledger_windows`: day at 00:00
/// UTC, week from Monday 00:00 UTC, month from the 1st.
fn window_starts(now: DateTime<Utc>) -> Option<(DateTime<Utc>, DateTime<Utc>, DateTime<Utc>)> {
    let day_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()?;
    // ponytail: week starts Monday UTC; make configurable if a customer asks
    let days_from_monday = i64::from(now.weekday().num_days_from_monday());
    let week_start = day_start - Duration::days(days_from_monday);
    let month_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()?;
    Some((day_start, week_start, month_start))
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

#[cfg(test)]
mod tests {
    use super::*;

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
