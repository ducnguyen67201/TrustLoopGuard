//! Policy DSL for TrustLoopGuard. YAML in, compiled `Policy` out.
//!
//! Also parses `AgentProfile` YAML — see `agent_parse::load_agent_str` —
//! and non-content policy families — see `family_parse::load_any_str`.

pub mod agent_parse;
pub mod family_ast;
pub mod family_parse;
pub mod policy_ast;
pub mod policy_parse;

pub use agent_parse::load_agent_str;
pub use family_ast::{
    AnyPolicy, ApprovalPolicy, ApprovalWhen, FamilyPolicy, FlowPolicy, FlowRule, MemoryPolicy,
    ParameterSourcePolicy,
};
pub use family_parse::{load_any_str, validate_family_policy, KNOWN_FAMILIES};
pub use policy_ast::{Action, MatchClause, Matcher, Policy, PolicyId};
pub use policy_parse::{load_str, validate_policy, PolicyError, ValidationIssue};
