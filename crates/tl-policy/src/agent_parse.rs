//! YAML loader for `AgentProfile`. Mirrors `policy_parse::load_str` for the
//! agent profile concept introduced in `v0-design-decisions.md §5`.
//!
//! Customers author one profile per agent (see `policies/agents/*.yaml`),
//! `tl-server` parses them at boot or via `POST /v1/agents`, and tier 3 LLM
//! judges consult the parsed profile for ground truth on what the agent is
//! permitted to claim.

use crate::policy_parse::PolicyError;
use tl_core::{AgentProfile, KnowledgeSourceKind};

/// Parse one agent profile from YAML. Performs minimal validation:
/// - `agent_id` must be non-empty
/// - `display_name` must be non-empty
/// - `scope.in_scope` must contain at least one entry (an agent with no
///   declared scope cannot be checked for out-of-scope answers).
pub fn load_agent_str(src: &str) -> Result<AgentProfile, PolicyError> {
    let profile: AgentProfile = serde_yaml::from_str(src)?;
    validate(&profile)?;
    Ok(profile)
}

fn validate(profile: &AgentProfile) -> Result<(), PolicyError> {
    if profile.agent_id.trim().is_empty() {
        return Err(PolicyError::Validation("agent_id is required".into()));
    }
    if profile.display_name.trim().is_empty() {
        return Err(PolicyError::Validation("display_name is required".into()));
    }
    if profile.scope.in_scope.is_empty() {
        return Err(PolicyError::Validation(
            "scope.in_scope must contain at least one entry".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_profile() {
        let yaml = r#"
agent_id: minimal-bot
display_name: Minimal Bot
scope:
  in_scope:
    - billing
authority: {}
tone:
  target: neutral
"#;
        let p = load_agent_str(yaml).expect("parse");
        assert_eq!(p.agent_id, "minimal-bot");
        assert_eq!(p.display_name, "Minimal Bot");
        assert_eq!(p.scope.in_scope, vec!["billing".to_string()]);
        assert!(p.scope.out_of_scope.is_empty());
        assert!(p.knowledge_sources.is_empty());
    }

    #[test]
    fn rejects_missing_agent_id() {
        let yaml = r#"
agent_id: ""
display_name: Bot
scope:
  in_scope:
    - help
authority: {}
tone: { target: neutral }
"#;
        let err = load_agent_str(yaml).unwrap_err();
        assert!(matches!(err, PolicyError::Validation(_)));
        assert!(err.to_string().contains("agent_id"));
    }

    #[test]
    fn rejects_missing_in_scope() {
        let yaml = r#"
agent_id: bot
display_name: Bot
scope: {}
authority: {}
tone: { target: neutral }
"#;
        let err = load_agent_str(yaml).unwrap_err();
        assert!(matches!(err, PolicyError::Validation(_)));
        assert!(err.to_string().contains("in_scope"));
    }

    #[test]
    fn rejects_malformed_yaml() {
        let yaml = "this is: not: valid: yaml: structure: [";
        let err = load_agent_str(yaml).unwrap_err();
        assert!(matches!(err, PolicyError::Yaml(_)));
    }

    #[test]
    fn loads_committed_fixture_acme_support_v3() {
        // Compile-time inclusion of the canonical example fixture.
        // If the path or content changes, this test fails loudly.
        let yaml = include_str!("../../../policies/agents/acme-support-v3.yaml");
        let p = load_agent_str(yaml).expect("fixture parses");
        assert_eq!(p.agent_id, "acme-support-v3");
        assert!(p.scope.in_scope.len() >= 3);
        assert!(!p.authority.cannot_promise.is_empty());
    }

    #[test]
    fn parses_full_featured_profile() {
        let yaml = r#"
agent_id: acme-support-v3
display_name: Acme Support Assistant
scope:
  in_scope:
    - billing questions
    - account access
    - product feature explanations
  out_of_scope:
    - legal advice
    - competitor comparisons
authority:
  can_promise:
    - "we'll respond within 24 hours"
    - "I'll escalate to a specialist"
  cannot_promise:
    - refunds
    - delivery dates
tone:
  target: warm-professional
  forbidden:
    - sarcastic
    - dismissive
knowledge_sources:
  - kb_id: acme-help-center
  - kb_id: acme-billing-policies
escalation_triggers:
  - threats of self-harm
  - mentions of legal action
"#;
        let p = load_agent_str(yaml).expect("parse");
        assert_eq!(p.agent_id, "acme-support-v3");
        assert_eq!(p.scope.in_scope.len(), 3);
        assert_eq!(p.scope.out_of_scope.len(), 2);
        assert_eq!(p.authority.can_promise.len(), 2);
        assert_eq!(p.authority.cannot_promise.len(), 2);
        assert_eq!(p.tone.target, "warm-professional");
        assert_eq!(p.tone.forbidden.len(), 2);
        assert_eq!(p.knowledge_sources.len(), 2);
        assert_eq!(p.knowledge_sources[0].kb_id, "acme-help-center");
        assert_eq!(p.escalation_triggers.len(), 2);
    }

    #[test]
    fn parses_web_knowledge_source_metadata() {
        let yaml = r#"
agent_id: acme-support-v3
display_name: Acme Support Assistant
scope:
  in_scope:
    - billing questions
authority: {}
tone:
  target: warm-professional
knowledge_sources:
  - kb_id: acme-docs
    kind: web
    url: https://docs.acme.test/help
    description: Public support docs
"#;
        let p = load_agent_str(yaml).expect("parse");
        assert_eq!(p.knowledge_sources.len(), 1);
        assert_eq!(p.knowledge_sources[0].kind, KnowledgeSourceKind::Web);
        assert_eq!(
            p.knowledge_sources[0].url.as_deref(),
            Some("https://docs.acme.test/help")
        );
        assert_eq!(
            p.knowledge_sources[0].description.as_deref(),
            Some("Public support docs")
        );
    }

    #[test]
    fn rejects_web_source_without_url() {
        let yaml = r#"
agent_id: acme-support-v3
display_name: Acme Support Assistant
scope:
  in_scope:
    - billing questions
authority: {}
tone:
  target: warm-professional
knowledge_sources:
  - kb_id: acme-docs
    kind: web
"#;
        let err = load_agent_str(yaml).unwrap_err();
        assert!(err.to_string().contains("knowledge_sources[0].url"));
    }

    #[test]
    fn rejects_web_source_private_host() {
        let yaml = r#"
agent_id: acme-support-v3
display_name: Acme Support Assistant
scope:
  in_scope:
    - billing questions
authority: {}
tone:
  target: warm-professional
knowledge_sources:
  - kb_id: local-admin
    kind: web
    url: http://localhost:8080/admin
"#;
        let err = load_agent_str(yaml).unwrap_err();
        assert!(err.to_string().contains("public http(s) URL"));
    }

    #[test]
    fn rejects_duplicate_knowledge_source_ids() {
        let yaml = r#"
agent_id: acme-support-v3
display_name: Acme Support Assistant
scope:
  in_scope:
    - billing questions
authority: {}
tone:
  target: warm-professional
knowledge_sources:
  - kb_id: acme-docs
  - kb_id: acme-docs
"#;
        let err = load_agent_str(yaml).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }
}
