use serde::{Deserialize, Serialize};
use tl_core::{Channel, Severity};

pub type PolicyId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    #[serde(default)]
    pub when: WhenClause,
    pub r#match: MatchClause,
    pub action: Action,
    #[serde(default)]
    pub rewrite: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: Severity,
}

fn default_severity() -> Severity {
    Severity::Medium
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WhenClause {
    #[serde(default)]
    pub channel: Vec<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatchClause {
    Any { any: Vec<Matcher> },
    All { all: Vec<Matcher> },
    Single(Matcher),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Matcher {
    Regex(String),
    Literal(String),
    Semantic(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Block,
    Rewrite,
    Escalate,
}
