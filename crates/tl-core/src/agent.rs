//! Agent profile types — registered up-front and referenced by `agent_id`
//! on every `CheckRequest`. Profiles tell the engine what an agent is
//! permitted to claim, where its scope ends, and what tone it should hold.
//!
//! See `docs/concept/v0-design-decisions.md` §5 for how profiles fit into
//! the four-source layering of guardrail context.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// What an agent is, what it may claim, and how it should sound.
///
/// Authored as YAML by the customer (see `policies/agents/*.yaml`), parsed
/// by `tl-policy::load_agent_str`, persisted in Postgres, cached in process,
/// and consulted by Tier 2 (out-of-scope embedding lookup) and Tier 3
/// (LLM judge ground truth).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentProfile {
    pub agent_id: String,
    pub display_name: String,
    pub scope: AgentScope,
    pub authority: AgentAuthority,
    pub tone: AgentTone,
    #[serde(default)]
    pub knowledge_sources: Vec<KnowledgeSource>,
    #[serde(default)]
    pub escalation_triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentListResponse {
    pub agents: Vec<AgentProfile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentScope {
    #[serde(default)]
    pub in_scope: Vec<String>,
    #[serde(default)]
    pub out_of_scope: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentAuthority {
    #[serde(default)]
    pub can_promise: Vec<String>,
    #[serde(default)]
    pub cannot_promise: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AgentTone {
    pub target: String,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum KnowledgeSourceKind {
    Local,
    Web,
}

impl Default for KnowledgeSourceKind {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct KnowledgeSource {
    pub kb_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub kind: Option<KnowledgeSourceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let profile = AgentProfile {
            agent_id: "acme-support-v3".into(),
            display_name: "Acme Support".into(),
            scope: AgentScope {
                in_scope: vec!["billing".into()],
                out_of_scope: vec!["legal advice".into()],
            },
            authority: AgentAuthority {
                can_promise: vec!["respond within 24h".into()],
                cannot_promise: vec!["refunds".into()],
            },
            tone: AgentTone {
                target: "warm-professional".into(),
                forbidden: vec!["sarcastic".into()],
            },
            knowledge_sources: vec![KnowledgeSource {
                kb_id: "acme-help".into(),
                kind: None,
                url: None,
                description: None,
            }],
            escalation_triggers: vec!["self-harm".into()],
        };
        let json = serde_json::to_string(&profile).unwrap();
        let parsed: AgentProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.agent_id, "acme-support-v3");
        assert_eq!(parsed.scope.in_scope, vec!["billing".to_string()]);
        assert_eq!(parsed.authority.cannot_promise, vec!["refunds".to_string()]);
    }

    #[test]
    fn optional_fields_default_to_empty() {
        let json = r#"{
            "agent_id": "minimal",
            "display_name": "Minimal Agent",
            "scope": { "in_scope": ["help"] },
            "authority": {},
            "tone": { "target": "neutral" }
        }"#;
        let parsed: AgentProfile = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.agent_id, "minimal");
        assert!(parsed.knowledge_sources.is_empty());
        assert!(parsed.escalation_triggers.is_empty());
        assert!(parsed.scope.out_of_scope.is_empty());
    }

    #[test]
    fn list_response_uses_agent_profile_contract() {
        let body = r#"{
            "agents": [{
                "agent_id": "minimal",
                "display_name": "Minimal Agent",
                "scope": { "in_scope": ["help"] },
                "authority": {},
                "tone": { "target": "neutral" }
            }]
        }"#;
        let parsed: AgentListResponse = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.agents.len(), 1);
        assert_eq!(parsed.agents[0].agent_id, "minimal");
    }
}
