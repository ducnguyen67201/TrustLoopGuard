//! AST for non-content policy families (`flow`, `parameter_source`,
//! `approval`, `memory`). Content policies keep their existing `Policy`
//! shape in `policy_ast`; a YAML document selects a family with a
//! top-level `family:` tag, and documents without one stay content.

use serde::{Deserialize, Serialize};
use tl_core::{AllowedSource, Severity, SideEffectClass};

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
}

impl FamilyPolicy {
    pub fn id(&self) -> &str {
        match self {
            FamilyPolicy::Flow(p) => &p.id,
            FamilyPolicy::ParameterSource(p) => &p.id,
            FamilyPolicy::Approval(p) => &p.id,
            FamilyPolicy::Memory(p) => &p.id,
        }
    }
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
/// `tools`/`side_effects` must be set (enforced by validation).
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
