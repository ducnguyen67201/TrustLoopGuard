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
use tl_core::{BudgetAlertConfig, BudgetAlertFiring, BudgetAlertThresholdType, BudgetAlertWindow};
use tl_policy::FinancialPolicy;
use tokio::sync::mpsc;

use crate::dashboard_admin::SettingsStore;
use crate::escalation::WebhookDelivery;

pub mod evaluator;
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

/// Create-time config shape (the store assigns id + timestamps).
#[derive(Debug, Clone, PartialEq)]
pub struct NewBudgetAlertConfig {
    pub name: String,
    pub window: BudgetAlertWindow,
    pub principal_id: Option<String>,
    pub threshold_type: BudgetAlertThresholdType,
    pub threshold_value: i64,
    pub webhook_url: Option<String>,
    pub enabled: bool,
}

/// Partial update: `None` fields are left unchanged.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpdateBudgetAlertConfig {
    pub name: Option<String>,
    pub window: Option<BudgetAlertWindow>,
    pub principal_id: Option<String>,
    pub threshold_type: Option<BudgetAlertThresholdType>,
    pub threshold_value: Option<i64>,
    pub webhook_url: Option<String>,
    pub enabled: Option<bool>,
}

/// One threshold crossing to record. The store assigns id + fired_at.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordBudgetAlertFiring {
    pub config_id: String,
    pub principal_id: String,
    pub window_start: DateTime<Utc>,
    pub cap_minor: i64,
    pub spent_minor: i64,
    pub currency: String,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait BudgetAlertStore: Send + Sync {
    async fn create_config(
        &self,
        workspace_id: &str,
        input: NewBudgetAlertConfig,
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
        update: UpdateBudgetAlertConfig,
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

    /// Firing history, newest first. `config_id = None` lists the
    /// whole workspace.
    async fn list_firings(
        &self,
        workspace_id: &str,
        config_id: Option<&str>,
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
        if !evaluator::crossed(
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
        caps.0 = min_cap(caps.0, policy.daily_minor);
        caps.1 = min_cap(caps.1, policy.weekly_minor);
        caps.2 = min_cap(caps.2, policy.monthly_minor);
    }
    caps
}

fn min_cap(current: Option<i64>, candidate: Option<i64>) -> Option<i64> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (value, None) | (None, value) => value,
    }
}

pub(crate) fn window_label(window: BudgetAlertWindow) -> &'static str {
    match window {
        BudgetAlertWindow::Day => "day",
        BudgetAlertWindow::Week => "week",
        BudgetAlertWindow::Month => "month",
    }
}

/// Adjective form used in validation messages ("no weekly cap
/// configured for this scope").
pub(crate) fn window_adjective(window: BudgetAlertWindow) -> &'static str {
    match window {
        BudgetAlertWindow::Day => "daily",
        BudgetAlertWindow::Week => "weekly",
        BudgetAlertWindow::Month => "monthly",
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
