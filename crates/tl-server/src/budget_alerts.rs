//! Budget alert thresholds: warn before the hard cap blocks.
//!
//! Configs ("alert at 80% of the weekly cap") are evaluated
//! synchronously right after each spend is recorded — the financial
//! ledger path and the LLM metering path both call into
//! [`process_spend`]. Dedup is the firings table's UNIQUE
//! `(config_id, principal_id, window_start)` insert; delivery rides
//! the generalized escalation webhook worker.
//!
//! Every store ships as trait + memory + postgres (see the financial
//! store trio).
//! `// ponytail: sync eval at write; poll worker if eval ever needs to get heavy`

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde_json::json;
use tl_core::{
    BudgetAlertConfig, BudgetAlertFiring, BudgetAlertThresholdType, BudgetAlertWindow,
    CreateBudgetAlertConfigRequest, SpendMeter, UpdateBudgetAlertConfigRequest,
};
use tl_policy::{FamilyPolicy, FinancialPolicy};
use tokio::sync::mpsc;

use crate::dashboard_admin::SettingsStore;
use crate::escalation::WebhookDelivery;
use crate::policies::PolicyStore;

mod handlers;
mod memory_store;

pub use handlers::{
    __path_create_budget_alert, __path_delete_budget_alert, __path_list_budget_alert_firings,
    __path_list_budget_alerts, __path_update_budget_alert, create_budget_alert,
    delete_budget_alert, list_budget_alert_firings, list_budget_alerts, update_budget_alert,
    BudgetAlertApiState,
};
pub use memory_store::MemoryBudgetAlertStore;

#[derive(Debug, thiserror::Error)]
pub enum BudgetAlertStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// One threshold crossing to record. The store assigns id + fired_at.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordBudgetAlertFiring {
    pub config_id: String,
    pub meter: SpendMeter,
    pub principal_id: String,
    pub window_start: DateTime<Utc>,
    pub cap_minor: i64,
    pub spent_minor: i64,
    pub currency: String,
    pub payload: serde_json::Value,
}

/// Stores take the tl-core request types directly (like the financial
/// store); the handlers normalize inputs first, and stores default a
/// missing `enabled` to `true`.
#[async_trait]
pub trait BudgetAlertStore: Send + Sync {
    async fn create_config(
        &self,
        workspace_id: &str,
        input: CreateBudgetAlertConfigRequest,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError>;

    async fn get_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError>;

    async fn list_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<BudgetAlertConfig>, BudgetAlertStoreError>;

    /// The spend-time hook's single indexed lookup; usually zero rows.
    async fn list_enabled_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<BudgetAlertConfig>, BudgetAlertStoreError>;

    async fn update_config(
        &self,
        workspace_id: &str,
        config_id: &str,
        update: UpdateBudgetAlertConfigRequest,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError>;

    async fn delete_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<(), BudgetAlertStoreError>;

    /// Insert-first dedup gate: `true` when this call recorded the
    /// firing (the caller should deliver), `false` when another spend
    /// in the same window already had.
    async fn try_record_firing(
        &self,
        workspace_id: &str,
        firing: RecordBudgetAlertFiring,
    ) -> Result<bool, BudgetAlertStoreError>;

    /// Firing history for one config, newest first.
    async fn list_firings(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<Vec<BudgetAlertFiring>, BudgetAlertStoreError>;
}

/// Everything the spend-time hooks need, bundled so the financial
/// service and the gateway metering path wire identically. The
/// delivery tx is `Option` (mirroring `escalation_tx`): `None` still
/// records firings, it just skips webhook sends.
#[derive(Clone)]
pub struct BudgetAlertRuntime {
    pub store: Arc<dyn BudgetAlertStore>,
    pub settings: Arc<dyn SettingsStore>,
    pub delivery_tx: Option<mpsc::Sender<WebhookDelivery>>,
}

/// One capped window's state at spend time. Callers only build entries
/// for windows that actually have a cap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowSpend {
    pub window: BudgetAlertWindow,
    pub window_start: DateTime<Utc>,
    pub cap_minor: i64,
    pub spent_minor: i64,
}

/// Shared spend-time alert hook for both spend sources (financial
/// ledger and LLM metering): load enabled configs, resolve the
/// tightest caps from the financial policies admitted by
/// `policy_matches`, sum spend per capped-and-watched window via
/// `spend_minor`, then record + deliver crossings through
/// [`process_spend`]. Infallible by design: alerting must never fail a
/// spend, so every error path is `tracing::error!` + return.
pub async fn evaluate_spend_alerts<M, S, Fut>(
    runtime: &BudgetAlertRuntime,
    policy_store: &dyn PolicyStore,
    workspace_id: &str,
    environment_id: &str,
    principal_id: &str,
    currency: &str,
    meter: SpendMeter,
    policy_matches: M,
    spend_minor: S,
) where
    M: Fn(&FinancialPolicy) -> bool,
    S: Fn(DateTime<Utc>, DateTime<Utc>) -> Fut,
    Fut: std::future::Future<Output = Result<i64, String>>,
{
    // One indexed lookup; almost always zero rows → early return.
    let configs = match runtime.store.list_enabled_configs(workspace_id).await {
        Ok(configs) => configs
            .into_iter()
            .filter(|config| config.meter == meter)
            .collect::<Vec<_>>(),
        Err(error) => {
            tracing::error!(workspace_id, error = %error, "budget alert config lookup failed");
            return;
        }
    };
    if configs.is_empty() {
        return;
    }
    // Caps come from the same policies the hard limits enforce.
    let families = match policy_store
        .list_enabled_families(workspace_id, environment_id)
        .await
    {
        Ok(families) => families,
        Err(error) => {
            tracing::error!(workspace_id, error = %error, "budget alert policy lookup failed");
            return;
        }
    };
    let (daily_cap, weekly_cap, monthly_cap) =
        min_window_caps(families.iter().filter_map(|family| match family.as_ref() {
            FamilyPolicy::Financial(financial) if policy_matches(financial) => Some(financial),
            _ => None,
        }));
    let now = Utc::now();
    let Some((day_start, week_start, month_start)) = window_starts(now) else {
        return;
    };
    let mut windows = Vec::new();
    for (window, window_start, cap) in [
        (BudgetAlertWindow::Day, day_start, daily_cap),
        (BudgetAlertWindow::Week, week_start, weekly_cap),
        (BudgetAlertWindow::Month, month_start, monthly_cap),
    ] {
        // Only sum windows that have both a cap and a config watching
        // them.
        let Some(cap_minor) = cap else { continue };
        if !configs.iter().any(|config| config.window == window) {
            continue;
        }
        match spend_minor(window_start, now).await {
            Ok(spent_minor) => windows.push(WindowSpend {
                window,
                window_start,
                cap_minor,
                spent_minor,
            }),
            Err(error) => {
                tracing::error!(workspace_id, error = %error, "budget alert spend sum failed");
            }
        }
    }
    if windows.is_empty() {
        return;
    }
    process_spend(
        runtime,
        workspace_id,
        principal_id,
        currency,
        &configs,
        &windows,
    )
    .await;
}

/// Evaluate `configs` against the caller-computed window sums, record
/// crossings (deduped), and enqueue webhook deliveries. Never fails:
/// every error is logged and skipped — alerting must not break spends.
pub async fn process_spend(
    runtime: &BudgetAlertRuntime,
    workspace_id: &str,
    principal_id: &str,
    currency: &str,
    configs: &[BudgetAlertConfig],
    windows: &[WindowSpend],
) {
    for config in configs {
        if !config.enabled {
            continue;
        }
        if let Some(scoped) = &config.principal_id {
            if scoped != principal_id {
                continue;
            }
        }
        // No entry = no cap on that window for this scope; a threshold
        // without a cap can never cross.
        let Some(window) = windows.iter().find(|w| w.window == config.window) else {
            continue;
        };
        if !crossed(
            config.threshold_type,
            config.threshold_value,
            window.cap_minor,
            window.spent_minor,
        ) {
            continue;
        }

        let payload = firing_payload(config, workspace_id, principal_id, currency, window);
        let recorded = runtime
            .store
            .try_record_firing(
                workspace_id,
                RecordBudgetAlertFiring {
                    config_id: config.id.clone(),
                    meter: config.meter,
                    principal_id: principal_id.to_string(),
                    window_start: window.window_start,
                    cap_minor: window.cap_minor,
                    spent_minor: window.spent_minor,
                    currency: currency.to_string(),
                    payload: payload.clone(),
                },
            )
            .await;
        match recorded {
            Ok(true) => {}
            // Another spend in this window already fired this config.
            Ok(false) => continue,
            Err(error) => {
                tracing::error!(
                    workspace_id,
                    config_id = %config.id,
                    error = %error,
                    "budget alert firing record failed; alert skipped"
                );
                continue;
            }
        }

        deliver_firing(runtime, workspace_id, config, payload).await;
    }
}

/// Resolve the delivery URL (per-config first, workspace
/// `escalation_webhook_url` fallback) and enqueue the webhook job.
/// A firing with no URL anywhere is recorded but not sent — that is
/// the dashboard-only configuration, not an error.
async fn deliver_firing(
    runtime: &BudgetAlertRuntime,
    workspace_id: &str,
    config: &BudgetAlertConfig,
    payload: serde_json::Value,
) {
    let Some(tx) = &runtime.delivery_tx else {
        tracing::debug!(
            workspace_id,
            config_id = %config.id,
            "budget alert fired; delivery worker not running, firing recorded only"
        );
        return;
    };
    let webhook_url = match &config.webhook_url {
        Some(url) => Some(url.clone()),
        None => match runtime.settings.get(workspace_id).await {
            Ok(settings) => settings.escalation_webhook_url,
            Err(error) => {
                tracing::error!(
                    workspace_id,
                    config_id = %config.id,
                    error = %error,
                    "budget alert webhook fallback lookup failed; firing recorded only"
                );
                None
            }
        },
    };
    let Some(webhook_url) = webhook_url else {
        tracing::debug!(
            workspace_id,
            config_id = %config.id,
            "budget alert fired; no webhook configured, firing recorded only"
        );
        return;
    };
    if let Err(error) = tx
        .send(WebhookDelivery {
            // Config id doubles as the persistence/correlation key —
            // it is a UUID, and the escalations table allows repeats.
            trace_id: config.id.clone(),
            webhook_url,
            body: payload,
        })
        .await
    {
        tracing::error!(
            workspace_id,
            config_id = %config.id,
            error = %error,
            "budget alert delivery enqueue failed; firing recorded only"
        );
    }
}

/// The webhook body. Also persisted verbatim on the firing row so the
/// dashboard shows exactly what was sent.
fn firing_payload(
    config: &BudgetAlertConfig,
    workspace_id: &str,
    principal_id: &str,
    currency: &str,
    window: &WindowSpend,
) -> serde_json::Value {
    let remaining = window.cap_minor.saturating_sub(window.spent_minor);
    // Integer percent; window.cap_minor > 0 is guaranteed by the
    // evaluator's cap guard upstream of every caller.
    let percent_used = if window.cap_minor > 0 {
        ((window.spent_minor as i128 * 100) / window.cap_minor as i128) as i64
    } else {
        0
    };
    json!({
        "type": "budget_alert",
        "workspace_id": workspace_id,
        "config_id": config.id,
        "config_name": config.name,
        "meter": meter_label(config.meter),
        "principal_id": principal_id,
        "window": window_label(config.window),
        "window_start": window.window_start.to_rfc3339(),
        "threshold_type": threshold_type_label(config.threshold_type),
        "threshold_value": config.threshold_value,
        "cap_minor": window.cap_minor,
        "spent_minor": window.spent_minor,
        "remaining_minor": remaining,
        "percent_used": percent_used,
        "currency": currency,
        "fired_at": Utc::now().to_rfc3339(),
    })
}

pub(crate) fn meter_label(meter: SpendMeter) -> &'static str {
    match meter {
        SpendMeter::Actions => "actions",
        SpendMeter::LlmUsage => "llm_usage",
    }
}

pub(crate) fn meter_from_str(value: &str) -> Option<SpendMeter> {
    match value {
        "actions" => Some(SpendMeter::Actions),
        "llm_usage" => Some(SpendMeter::LlmUsage),
        _ => None,
    }
}

/// Window boundaries shared with the hard caps: day at 00:00 UTC, week
/// from Monday 00:00 UTC, month from the 1st. `None` only on calendar
/// math failure, which callers treat as "skip alerting".
// ponytail: week starts Monday UTC; make configurable if a customer asks
pub fn window_starts(now: DateTime<Utc>) -> Option<(DateTime<Utc>, DateTime<Utc>, DateTime<Utc>)> {
    let day_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()?;
    let days_from_monday = i64::from(now.weekday().num_days_from_monday());
    let week_start = day_start - Duration::days(days_from_monday);
    let month_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()?;
    Some((day_start, week_start, month_start))
}

/// Tightest cap per window across the matching financial policies —
/// the same cap the hard limit enforces first.
pub fn min_window_caps<'a>(
    policies: impl Iterator<Item = &'a FinancialPolicy>,
) -> (Option<i64>, Option<i64>, Option<i64>) {
    let mut caps: (Option<i64>, Option<i64>, Option<i64>) = (None, None, None);
    for policy in policies {
        caps.0 = caps.0.into_iter().chain(policy.daily_minor).min();
        caps.1 = caps.1.into_iter().chain(policy.weekly_minor).min();
        caps.2 = caps.2.into_iter().chain(policy.monthly_minor).min();
    }
    caps
}

pub(crate) fn window_label(window: BudgetAlertWindow) -> &'static str {
    match window {
        BudgetAlertWindow::Day => "day",
        BudgetAlertWindow::Week => "week",
        BudgetAlertWindow::Month => "month",
    }
}

pub(crate) fn window_from_str(value: &str) -> Option<BudgetAlertWindow> {
    match value {
        "day" => Some(BudgetAlertWindow::Day),
        "week" => Some(BudgetAlertWindow::Week),
        "month" => Some(BudgetAlertWindow::Month),
        _ => None,
    }
}

pub(crate) fn threshold_type_label(threshold_type: BudgetAlertThresholdType) -> &'static str {
    match threshold_type {
        BudgetAlertThresholdType::Percent => "percent",
        BudgetAlertThresholdType::Absolute => "absolute",
    }
}

pub(crate) fn threshold_type_from_str(value: &str) -> Option<BudgetAlertThresholdType> {
    match value {
        "percent" => Some(BudgetAlertThresholdType::Percent),
        "absolute" => Some(BudgetAlertThresholdType::Absolute),
        _ => None,
    }
}

/// Has spend crossed the configured threshold of the window's cap?
/// Pure threshold math, integer-only — no floats anywhere near money.
///
/// - `percent`: fires when `spent * 100 >= cap * threshold`. The
///   integer cross-multiplication means fractional boundaries round
///   toward firing on the next whole unit (cap 3 at 80% ⇒ threshold
///   is 2.4 ⇒ fires at spent 3).
/// - `absolute`: fires when the remaining budget (`cap - spent`)
///   drops to `threshold_value` or below.
///
/// A cap of zero (or less) never fires: the hard limit already blocks
/// everything, so there is nothing to warn about.
fn crossed(
    threshold_type: BudgetAlertThresholdType,
    threshold_value: i64,
    cap_minor: i64,
    spent_minor: i64,
) -> bool {
    if cap_minor <= 0 {
        return false;
    }
    match threshold_type {
        // i128 so `spent * 100` cannot overflow near i64::MAX.
        BudgetAlertThresholdType::Percent => {
            i128::from(spent_minor) * 100 >= i128::from(cap_minor) * i128::from(threshold_value)
        }
        BudgetAlertThresholdType::Absolute => {
            i128::from(cap_minor) - i128::from(spent_minor) <= i128::from(threshold_value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BudgetAlertThresholdType::{Absolute, Percent};

    #[test]
    fn percent_fires_exactly_at_the_boundary() {
        // cap 5000, threshold 80% → boundary at 4000.
        assert!(crossed(Percent, 80, 5000, 4000));
        assert!(crossed(Percent, 80, 5000, 4001));
        assert!(crossed(Percent, 80, 5000, 5000));
    }

    #[test]
    fn percent_below_the_boundary_does_not_fire() {
        assert!(!crossed(Percent, 80, 5000, 3999));
        assert!(!crossed(Percent, 80, 5000, 0));
    }

    #[test]
    fn cap_zero_never_fires() {
        assert!(!crossed(Percent, 80, 0, 0));
        assert!(!crossed(Percent, 80, 0, 100));
        assert!(!crossed(Absolute, 1000, 0, 0));
        assert!(!crossed(Absolute, 1000, -1, 0));
    }

    #[test]
    fn percent_integer_math_rounds_toward_the_next_whole_unit() {
        // cap 3 at 80%: the fractional boundary is 2.4, so the alert
        // fires at spent 3, not spent 2.
        assert!(!crossed(Percent, 80, 3, 2));
        assert!(crossed(Percent, 80, 3, 3));
    }

    #[test]
    fn percent_does_not_overflow_near_i64_max() {
        assert!(crossed(Percent, 80, i64::MAX, i64::MAX));
        assert!(!crossed(Percent, 100, i64::MAX, i64::MAX - 1));
    }

    #[test]
    fn absolute_fires_when_remaining_reaches_the_threshold() {
        // cap 5000, threshold 1000: fires once remaining <= 1000.
        assert!(crossed(Absolute, 1000, 5000, 4000));
        assert!(crossed(Absolute, 1000, 5000, 4200));
        assert!(!crossed(Absolute, 1000, 5000, 3999));
    }

    #[test]
    fn absolute_fires_past_the_cap() {
        assert!(crossed(Absolute, 0, 5000, 5000));
        assert!(crossed(Absolute, 0, 5000, 6000));
        assert!(!crossed(Absolute, 0, 5000, 4999));
    }
}
