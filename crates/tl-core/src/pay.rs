//! Payment-gate wire contracts.
//!
//! Per-owner spend caps and the decision they produce. Amounts are `i64`
//! **minor units** (e.g. cents), matching the rest of tl-core's money model —
//! the MCP boundary parses any human "100.00" into minor units before it
//! reaches here, so these types never carry floats or currency strings.

use serde::{Deserialize, Serialize};

use crate::Verdict;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Outcome of a gated spend. `Hold` is the borderline band that waits for a
/// human; it maps onto the engine's `Escalate` verdict for audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum PayStatus {
    Allow,
    Block,
    Hold,
}

impl PayStatus {
    /// Engine verdict this status records as. `Hold` escalates to a human.
    pub fn verdict(self) -> Verdict {
        match self {
            PayStatus::Allow => Verdict::Allow,
            PayStatus::Block => Verdict::Block,
            PayStatus::Hold => Verdict::Escalate,
        }
    }
}

/// Per-owner spend caps. Every cap is inclusive and optional — `None` means
/// "no limit on this axis". Keyed by `owner` in storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PayPolicy {
    pub owner: String,
    /// Largest single spend allowed. Over this → `Block`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub per_transaction_minor: Option<i64>,
    /// Cumulative spend allowed within the current UTC day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub daily_minor: Option<i64>,
    /// Cumulative spend allowed within the current UTC month.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub monthly_minor: Option<i64>,
    /// Spend at or above this (but within the caps) → `Hold` for approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub hold_above_minor: Option<i64>,
}

/// A gated spend an agent wants to make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PaySpendRequest {
    pub owner: String,
    pub amount_minor: i64,
    pub merchant: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub memo: Option<String>,
}

/// One row of the audit export: a past decision and, for holds, whether it
/// was later approved. `created_at` is RFC3339.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PayAuditEntry {
    pub decision_id: String,
    pub owner: String,
    pub amount_minor: i64,
    pub merchant: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub category: Option<String>,
    pub status: PayStatus,
    /// `None` unless this was a hold: `Some(true)` approved, `Some(false)` denied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub resolved: Option<bool>,
    pub created_at: String,
}

/// The gate's answer for one spend. `decision_id` is the trace id, the handle
/// `resolve_hold` and `export_audit` refer back to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PayDecision {
    pub status: PayStatus,
    pub reason: String,
    pub decision_id: String,
}
