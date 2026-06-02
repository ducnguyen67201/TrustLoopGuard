//! YAML loader for `AgentProfile`. Mirrors `policy_parse::load_str` for the
//! agent profile concept introduced in `v0-design-decisions.md §5`.
//!
//! Customers author one profile per agent (see `policies/agents/*.yaml`),
//! `tl-server` parses them at boot or via `POST /v1/agents`, and tier 3 LLM
//! judges consult the parsed profile for ground truth on what the agent is
//! permitted to claim.

mod validation;

use crate::policy_parse::PolicyError;
use tl_core::AgentProfile;

/// Parse one agent profile from YAML. Performs minimal validation:
/// - `agent_id` must be non-empty
/// - `display_name` must be non-empty
/// - `scope.in_scope` must contain at least one entry (an agent with no
///   declared scope cannot be checked for out-of-scope answers).
pub fn load_agent_str(src: &str) -> Result<AgentProfile, PolicyError> {
    let profile: AgentProfile = serde_yaml::from_str(src)?;
    validation::validate(&profile)?;
    Ok(profile)
}

#[cfg(test)]
mod tests;
