//! AST for non-content policy families (`flow`, `parameter_source`,
//! `approval`, `memory`). Content policies keep their existing `Policy`
//! shape in `policy_ast`; a YAML document selects a family with a
//! top-level `family:` tag, and documents without one stay content.

use serde::{Deserialize, Serialize};
use tl_core::{
    AllowedSource, FinancialActionKind, FinancialActionPrecondition, FinancialRail, Severity,
    SideEffectClass,
};

use crate::policy_ast::{Action, Policy, PolicyId};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// A parsed policy document of any family. `Content` wraps the legacy
/// `Policy` shape. Not a serde type: family discrimination happens in
/// `family_parse::load_any_str`, which probes the `family:` field so an
/// unknown family fails loudly instead of falling through to content.
#[derive(Debug, Clone)]
pub enum AnyPolicy {
    Family(FamilyPolicy),
    Content(Policy),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum FamilyPolicy {
    Flow(FlowPolicy),
    ParameterSource(ParameterSourcePolicy),
    Approval(ApprovalPolicy),
    Memory(MemoryPolicy),
    Payment(PaymentPolicy),
    Financial(FinancialPolicy),
}

impl FamilyPolicy {
    pub fn id(&self) -> &str {
        match self {
            FamilyPolicy::Flow(p) => &p.id,
            FamilyPolicy::ParameterSource(p) => &p.id,
            FamilyPolicy::Approval(p) => &p.id,
            FamilyPolicy::Memory(p) => &p.id,
            FamilyPolicy::Payment(p) => &p.id,
            FamilyPolicy::Financial(p) => &p.id,
        }
    }
}

/// Scope for a payment policy: which owners (agents) and operations it caps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct PaymentWhen {
    /// Owners (principal agent ids) the caps apply to. Empty = all owners.
    #[serde(default)]
    pub agents: Vec<String>,
    /// Operations treated as payments, e.g. `["pay"]`. Empty = match none
    /// (fail closed: a payment policy with no operation caps nothing).
    #[serde(default)]
    pub operations: Vec<String>,
}

/// Per-owner spend caps. Amounts are `i64` minor units (cents); caps are
/// inclusive. `per_transaction`/`daily`/`monthly` over-cap → `on_breach`
/// (default Block); `hold_above` → Escalate (a human-approved hold).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct PaymentPolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub when: PaymentWhen,
    #[serde(default)]
    pub per_transaction_minor: Option<i64>,
    #[serde(default)]
    pub hold_above_minor: Option<i64>,
    #[serde(default)]
    pub daily_minor: Option<i64>,
    #[serde(default)]
    pub monthly_minor: Option<i64>,
    /// Verdict when a hard cap (per_transaction/daily/monthly) is exceeded.
    #[serde(default = "default_block_action")]
    pub on_breach: Action,
}

fn default_block_action() -> Action {
    Action::Block
}

fn default_escalate_action() -> Action {
    Action::Escalate
}

/// Scope for typed financial actions. Empty selectors are invalid for
/// financial policies; unlike legacy payment policies, they are meant for the
/// `/v1/financial/actions` contract rather than generic event parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FinancialWhen {
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub action_kinds: Vec<FinancialActionKind>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub currencies: Vec<String>,
    #[serde(default)]
    pub rails: Vec<FinancialRail>,
}

/// First-class financial authorization controls for typed financial actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FinancialPolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    #[serde(default)]
    pub when: FinancialWhen,
    #[serde(default)]
    pub per_transaction_minor: Option<i64>,
    #[serde(default)]
    pub hold_above_minor: Option<i64>,
    #[serde(default)]
    pub daily_minor: Option<i64>,
    #[serde(default)]
    pub monthly_minor: Option<i64>,
    #[serde(default)]
    pub allowed_counterparty_ids: Vec<String>,
    #[serde(default)]
    pub denied_counterparty_ids: Vec<String>,
    #[serde(default)]
    pub hold_new_counterparty: bool,
    #[serde(default)]
    pub mandate_required: bool,
    #[serde(default)]
    pub approval_threshold_minor: Option<i64>,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    #[serde(default)]
    pub refund_original_method_only: bool,
    #[serde(default)]
    pub required_preconditions: Vec<FinancialActionPrecondition>,
    #[serde(default = "default_escalate_action")]
    pub missing_evidence_action: Action,
    #[serde(default = "default_block_action")]
    pub failed_precondition_action: Action,
    #[serde(default = "default_block_action")]
    pub on_breach: Action,
}

/// Source-to-sink and action-integrity rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FlowPolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    #[serde(flatten)]
    pub rule: FlowRule,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum FlowRule {
    /// Sensitive data may flow only to allowed sinks.
    DestinationPermission { sinks: Vec<SideEffectClass> },
    /// High-impact actions must be authorized by trusted context.
    ActionIntegrity,
}

/// Allowed-source rules for one authority-bearing parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ParameterSourcePolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub tool: String,
    pub param: String,
    pub allowed_sources: Vec<AllowedSource>,
    pub action: Action,
}

/// Human/admin approval requirements for matching actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ApprovalPolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub when: ApprovalWhen,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
    pub action: Action,
}

/// Conditions selecting which actions require approval. At least one of
/// `tools`/`side_effects` must be set — a parser-level constraint
/// enforced by `family_parse::validate_family_policy`, not by the type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ApprovalWhen {
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub side_effects: Vec<SideEffectClass>,
}

/// Write-time memory policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MemoryPolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub deny_untrusted_authority_writes: bool,
    pub action: Action,
}

fn default_severity() -> Severity {
    Severity::Medium
}
