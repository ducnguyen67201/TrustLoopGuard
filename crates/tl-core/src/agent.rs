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
    /// Raw system prompt the customer ships to their LLM. Source of truth
    /// for auto-generating guardrails: `POST /v1/agents/{id}/guardrails:generate`
    /// reads this and asks an LLM to derive a policy set tailored to it.
    /// Optional at the type level so existing profiles keep deserializing;
    /// the generate endpoint enforces presence at call time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub system_prompt: Option<String>,
    /// Optional machine-readable agent definition (e.g. an n8n workflow export)
    /// imported alongside or instead of the chat `system_prompt`. The
    /// hardening loop's attack-vector planner analyses this to find injectable
    /// source→sink paths and tailor attacks to the agent. Absent ⇒ a plain
    /// chat agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub workflow_definition: Option<WorkflowDefinition>,
    /// Loopback endpoint this agent is reachable at (the arena adapter contract),
    /// captured at import so the Attacks page can target it without re-typing.
    /// Loopback-only; validated at the web edge and by the dispatch SSRF guard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub target_url: Option<String>,
}

/// A machine-readable agent definition imported for hardening. `source` names
/// the format (`n8n` today; the analyser keys off it), `definition` is the raw
/// exported JSON kept verbatim so we never lose fidelity round-tripping.
///
/// ponytail: a struct, not a tagged `Chat | WorkflowJson | ToolSchema` enum —
/// "chat" is just the absence of this field, and tool-schema import has no
/// caller yet. Add a `source` value (and an analyser arm) when a second format
/// actually arrives.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct WorkflowDefinition {
    /// Format discriminator, e.g. `n8n`.
    pub source: String,
    /// Raw exported workflow JSON, kept verbatim.
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub definition: serde_json::Value,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum KnowledgeSourceKind {
    #[default]
    Local,
    Web,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct KnowledgeSource {
    pub kb_id: String,
    #[serde(default)]
    #[cfg_attr(
        feature = "ts-export",
        ts(as = "Option<KnowledgeSourceKind>", optional)
    )]
    pub kind: KnowledgeSourceKind,
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
                kind: KnowledgeSourceKind::Local,
                url: None,
                description: None,
            }],
            escalation_triggers: vec!["self-harm".into()],
            system_prompt: Some("You are Acme support…".into()),
            workflow_definition: None,
            target_url: Some("http://127.0.0.1:9112".into()),
        };
        let json = serde_json::to_string(&profile).unwrap();
        let parsed: AgentProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.agent_id, "acme-support-v3");
        assert_eq!(parsed.target_url.as_deref(), Some("http://127.0.0.1:9112"));
        assert_eq!(parsed.scope.in_scope, vec!["billing".to_string()]);
        assert_eq!(parsed.authority.cannot_promise, vec!["refunds".to_string()]);
        assert_eq!(
            parsed.system_prompt.as_deref(),
            Some("You are Acme support…")
        );
    }

    #[test]
    fn workflow_definition_round_trips() {
        let json = r#"{
            "agent_id": "n8n-invoice",
            "display_name": "Invoice Flow",
            "scope": { "in_scope": ["invoices"] },
            "authority": {},
            "tone": { "target": "neutral" },
            "workflow_definition": {
                "source": "n8n",
                "definition": { "nodes": [{ "type": "n8n-nodes-base.httpRequest" }] }
            }
        }"#;
        let parsed: AgentProfile = serde_json::from_str(json).unwrap();
        let wf = parsed
            .workflow_definition
            .as_ref()
            .expect("workflow present");
        assert_eq!(wf.source, "n8n");
        assert_eq!(
            wf.definition["nodes"][0]["type"],
            "n8n-nodes-base.httpRequest"
        );
        // Survives a full serialize → deserialize cycle (storage stores JSONB).
        let round = serde_json::to_string(&parsed).unwrap();
        let again: AgentProfile = serde_json::from_str(&round).unwrap();
        assert_eq!(again.workflow_definition.unwrap().source, "n8n");
        // Absent field stays None.
        let chat = r#"{"agent_id":"c","display_name":"C","scope":{},"authority":{},"tone":{"target":"x"}}"#;
        let chat: AgentProfile = serde_json::from_str(chat).unwrap();
        assert!(chat.workflow_definition.is_none());
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
