//! Budget alert threshold wire types.
//!
//! A budget alert config warns before a budget window's hard cap
//! blocks: "alert at 80% of the weekly cap" (percent) or "alert when
//! 10.00 remains" (absolute). Alerts are evaluated synchronously at
//! spend-record time and fire at most once per (config, principal,
//! window) — each crossing is one `BudgetAlertFiring`.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Which budget window the threshold watches. Windows share the hard
/// caps' boundaries: day at 00:00 UTC, week from Monday 00:00 UTC,
/// month from the 1st.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum BudgetAlertWindow {
    Day,
    Week,
    Month,
}

/// How `threshold_value` is read: `percent` fires when spend reaches
/// that percentage of the window's cap (1-100); `absolute` fires when
/// the remaining budget (cap - spent) drops to the value, in currency
/// minor units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum BudgetAlertThresholdType {
    Percent,
    Absolute,
}

/// One configured alert threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BudgetAlertConfig {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub window: BudgetAlertWindow,
    /// `null` = any principal; the alert is evaluated per acting
    /// principal.
    pub principal_id: Option<String>,
    pub threshold_type: BudgetAlertThresholdType,
    pub threshold_value: i64,
    /// Delivery target. `null` falls back to the workspace
    /// `escalation_webhook_url`; with neither set, firings are still
    /// recorded but nothing is sent.
    pub webhook_url: Option<String>,
    pub enabled: bool,
    /// RFC 3339 timestamps.
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BudgetAlertConfigListResponse {
    pub configs: Vec<BudgetAlertConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateBudgetAlertConfigRequest {
    pub name: String,
    pub window: BudgetAlertWindow,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    pub threshold_type: BudgetAlertThresholdType,
    pub threshold_value: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    /// Defaults to `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Partial update; absent fields are left unchanged. The nullable
/// fields (`principal_id`, `webhook_url`) cannot be cleared through
/// this shape — recreate the config to widen its scope.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateBudgetAlertConfigRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<BudgetAlertWindow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_type: Option<BudgetAlertThresholdType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_value: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// One recorded threshold crossing — at most one per (config,
/// principal, window).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BudgetAlertFiring {
    pub id: String,
    pub workspace_id: String,
    pub config_id: String,
    pub principal_id: String,
    /// RFC 3339 window boundary the dedup key is anchored to.
    pub window_start: String,
    pub cap_minor: i64,
    pub spent_minor: i64,
    pub currency: String,
    /// The exact JSON body delivered to the webhook.
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub payload: serde_json::Value,
    /// RFC 3339 timestamp.
    pub fired_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BudgetAlertFiringListResponse {
    pub firings: Vec<BudgetAlertFiring>,
}
