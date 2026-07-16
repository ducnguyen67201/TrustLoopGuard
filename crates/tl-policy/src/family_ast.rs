//! AST for non-content policy families. Content policies keep their existing `Policy`
//! shape in `policy_ast`; a YAML document selects a family with a
//! top-level `family:` tag, and documents without one stay content.

use serde::{Deserialize, Serialize};
use tl_core::{
    AllowedSource, AuthorizationEffect, Confidentiality, FinancialActionKind,
    FinancialActionPrecondition, FinancialRail, Integrity, Origin, PolicyFamily, Severity,
    SideEffectClass, SpendMeter, Trust,
};

use crate::policy_ast::{Policy, PolicyId};

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

    pub fn action(&self) -> Option<AuthorizationEffect> {
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
#[allow(clippy::large_enum_variant)]
pub enum FamilyPolicy {
    Flow(FlowPolicy),
    ParameterSource(ParameterSourcePolicy),
    Approval(ApprovalPolicy),
    Memory(MemoryPolicy),
    Financial(FinancialPolicy),
    SourceLabel(SourceLabelFamilyPolicy),
    Tool(ToolPolicy),
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
            FamilyPolicy::Tool(p) => &p.id,
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
            FamilyPolicy::Tool(_) => PolicyFamily::Tool,
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
            FamilyPolicy::Tool(p) => p.description.as_deref(),
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
            FamilyPolicy::Tool(p) => p.severity,
        }
    }

    pub fn action(&self) -> Option<AuthorizationEffect> {
        match self {
            FamilyPolicy::Flow(p) => Some(p.action),
            FamilyPolicy::ParameterSource(p) => Some(p.action),
            FamilyPolicy::Approval(p) => Some(p.action),
            FamilyPolicy::Memory(p) => Some(p.action),
            FamilyPolicy::Financial(p) => Some(p.on_breach),
            FamilyPolicy::SourceLabel(_) => None,
            FamilyPolicy::Tool(p) => Some(p.action),
        }
    }
}

fn default_deny_effect() -> AuthorizationEffect {
    AuthorizationEffect::Deny
}

fn default_defer_effect() -> AuthorizationEffect {
    AuthorizationEffect::Defer
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
    /// Spend meter this policy governs. Defaults to `actions`, so every
    /// stored policy written before the field existed keeps gating typed
    /// financial actions; `llm_usage` policies are only ever evaluated
    /// by the gateway budget hook.
    #[serde(default)]
    pub meter: SpendMeter,
    #[serde(default)]
    pub per_transaction_minor: Option<i64>,
    #[serde(default)]
    pub daily_minor: Option<i64>,
    #[serde(default)]
    pub weekly_minor: Option<i64>,
    #[serde(default)]
    pub monthly_minor: Option<i64>,
    #[serde(default)]
    pub allowed_counterparty_ids: Vec<String>,
    #[serde(default)]
    pub denied_counterparty_ids: Vec<String>,
    #[serde(default)]
    pub require_approval_for_new_counterparty: bool,
    #[serde(default)]
    pub grant_required: bool,
    #[serde(default)]
    pub approval_threshold_minor: Option<i64>,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    #[serde(default)]
    pub refund_original_method_only: bool,
    #[serde(default)]
    pub required_preconditions: Vec<FinancialActionPrecondition>,
    #[serde(default = "default_defer_effect")]
    pub missing_evidence_effect: AuthorizationEffect,
    #[serde(default = "default_deny_effect")]
    pub failed_precondition_effect: AuthorizationEffect,
    #[serde(default = "default_deny_effect")]
    pub on_breach: AuthorizationEffect,
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
    pub action: AuthorizationEffect,
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
    pub action: AuthorizationEffect,
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
    pub action: AuthorizationEffect,
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
    pub action: AuthorizationEffect,
}

/// Scope for a tool policy. Empty selectors are rejected at authoring time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolWhen {
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub side_effects: Vec<SideEffectClass>,
    #[serde(default)]
    pub tools: Vec<ToolSelector>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolSelector {
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub schema_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ToolMatchClause {
    Any { any: Vec<ToolMatcher> },
    All { all: Vec<ToolMatcher> },
    Single(ToolMatcher),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ToolMatcher {
    Fact { fact: ToolFactMatcher },
    Parameter { parameter: ToolParameterMatcher },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolFactMatcher {
    pub key: String,
    #[serde(flatten)]
    pub value: ToolValueMatcher,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolParameterMatcher {
    pub path: String,
    #[serde(flatten)]
    pub value: ToolValueMatcher,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolValueMatcher {
    #[serde(default)]
    pub equals: Option<String>,
    #[serde(default)]
    pub one_of: Vec<String>,
    #[serde(default)]
    pub regex: Option<String>,
}

/// Deterministic policies for authority-bearing tool invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolPolicy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    #[serde(default)]
    pub when: ToolWhen,
    pub r#match: ToolMatchClause,
    pub action: AuthorizationEffect,
    pub reason: String,
    #[serde(default)]
    pub remediation: Option<String>,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    #[serde(default)]
    pub max_grant_ttl_seconds: Option<u64>,
}

fn default_severity() -> Severity {
    Severity::Medium
}
