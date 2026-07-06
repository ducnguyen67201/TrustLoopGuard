//! AST for non-content policy families (`flow`, `parameter_source`,
//! `approval`, `memory`). Content policies keep their existing `Policy`
//! shape in `policy_ast`; a YAML document selects a family with a
//! top-level `family:` tag, and documents without one stay content.

use serde::{Deserialize, Serialize};
use tl_core::{
    AllowedSource, Confidentiality, FinancialActionKind, FinancialActionPrecondition,
    FinancialRail, Integrity, Origin, PolicyFamily, Severity, SideEffectClass, Trust,
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

impl AnyPolicy {
    pub fn id(&self) -> &str {
        match self {
            AnyPolicy::Family(policy) => policy.id(),
            AnyPolicy::Content(policy) => &policy.id,
        }
    }

    pub fn family(&self) -> PolicyFamily {
        match self {
            AnyPolicy::Family(policy) => policy.family(),
            AnyPolicy::Content(_) => PolicyFamily::Content,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            AnyPolicy::Family(policy) => policy.description(),
            AnyPolicy::Content(policy) => policy.description.as_deref(),
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            AnyPolicy::Family(policy) => policy.severity(),
            AnyPolicy::Content(policy) => policy.severity,
        }
    }

    pub fn action(&self) -> Option<Action> {
        match self {
            AnyPolicy::Family(policy) => policy.action(),
            AnyPolicy::Content(policy) => Some(policy.action),
        }
    }

    pub fn owner_agent_id(&self) -> Option<&str> {
        match self {
            AnyPolicy::Family(_) => None,
            AnyPolicy::Content(policy) => policy.owner_agent_id.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum FamilyPolicy {
    Flow(FlowPolicy),
    ParameterSource(ParameterSourcePolicy),
    Approval(ApprovalPolicy),
    Memory(MemoryPolicy),
    Financial(FinancialPolicy),
    SourceLabel(SourceLabelFamilyPolicy),
}

impl FamilyPolicy {
    pub fn id(&self) -> &str {
        match self {
            FamilyPolicy::Flow(p) => &p.id,
            FamilyPolicy::ParameterSource(p) => &p.id,
            FamilyPolicy::Approval(p) => &p.id,
            FamilyPolicy::Memory(p) => &p.id,
            FamilyPolicy::Financial(p) => &p.id,
            FamilyPolicy::SourceLabel(p) => &p.id,
        }
    }

    pub fn family(&self) -> PolicyFamily {
        match self {
            FamilyPolicy::Flow(_) => PolicyFamily::Flow,
            FamilyPolicy::ParameterSource(_) => PolicyFamily::ParameterSource,
            FamilyPolicy::Approval(_) => PolicyFamily::Approval,
            FamilyPolicy::Memory(_) => PolicyFamily::Memory,
            FamilyPolicy::Financial(_) => PolicyFamily::Financial,
            FamilyPolicy::SourceLabel(_) => PolicyFamily::SourceLabel,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            FamilyPolicy::Flow(p) => p.description.as_deref(),
            FamilyPolicy::ParameterSource(p) => p.description.as_deref(),
            FamilyPolicy::Approval(p) => p.description.as_deref(),
            FamilyPolicy::Memory(p) => p.description.as_deref(),
            FamilyPolicy::Financial(p) => p.description.as_deref(),
            FamilyPolicy::SourceLabel(p) => p.description.as_deref(),
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            FamilyPolicy::Flow(p) => p.severity,
            FamilyPolicy::ParameterSource(p) => p.severity,
            FamilyPolicy::Approval(p) => p.severity,
            FamilyPolicy::Memory(p) => p.severity,
            FamilyPolicy::Financial(p) => p.severity,
            FamilyPolicy::SourceLabel(p) => p.severity,
        }
    }

    pub fn action(&self) -> Option<Action> {
        match self {
            FamilyPolicy::Flow(p) => Some(p.action),
            FamilyPolicy::ParameterSource(p) => Some(p.action),
            FamilyPolicy::Approval(p) => Some(p.action),
            FamilyPolicy::Memory(p) => Some(p.action),
            FamilyPolicy::Financial(p) => Some(p.on_breach),
            FamilyPolicy::SourceLabel(_) => None,
        }
    }
}

fn default_block_action() -> Action {
    Action::Block
}

fn default_escalate_action() -> Action {
    Action::Escalate
}

/// Scope for typed financial actions. Financial policies are meant for the
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

/// Workspace source-label override, stored in the unified policy registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SourceLabelFamilyPolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub origin: Origin,
    #[serde(default)]
    pub trust: Option<Trust>,
    #[serde(default)]
    pub confidentiality: Option<Confidentiality>,
    #[serde(default)]
    pub integrity: Option<Integrity>,
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
