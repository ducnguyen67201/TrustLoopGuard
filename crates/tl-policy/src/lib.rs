//! Policy DSL for TrustLoopGuard. YAML in, compiled `Policy` out.
//!
//! Also parses `AgentProfile` YAML — see `agent_parse::load_agent_str`.

pub mod agent_parse;
pub mod policy_ast;
pub mod policy_parse;

pub use agent_parse::load_agent_str;
pub use policy_ast::{Action, MatchClause, Matcher, Policy, PolicyId};
pub use policy_parse::{load_str, PolicyError};
