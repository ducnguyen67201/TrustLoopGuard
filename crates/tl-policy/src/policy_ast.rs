use serde::{Deserialize, Serialize};
use tl_core::{Channel, Severity};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

pub type PolicyId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Policy {
    pub id: PolicyId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub when: WhenClause,
    pub r#match: MatchClause,
    pub action: Action,
    #[serde(default)]
    pub rewrite: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    /// Agent that owns this policy. Set by
    /// `POST /v1/agents/{id}/guardrails:generate` so deleting the agent
    /// can cascade-delete its generated policies. `None` for global
    /// policies authored directly via `POST /v1/policies`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_agent_id: Option<String>,
}

fn default_severity() -> Severity {
    Severity::Medium
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct WhenClause {
    #[serde(default, alias = "channel")]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum MatchClause {
    Any { any: Vec<Matcher> },
    All { all: Vec<Matcher> },
    Single(Matcher),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum Matcher {
    Regex(String),
    Literal(String),
    Semantic(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum Action {
    Allow,
    Block,
    Rewrite,
    Escalate,
}
