//! Prompt templates + JSON schemas for the three Tier 3 judges.
//!
//! Tier 3 (in `tl-engine`) builds the per-request prompt by substituting
//! `{{PLACEHOLDERS}}` and then calls
//! `LlmRouter::judge(kind, tenant, prompt, schema)`. Keeping the prompts
//! and schemas next to the router means a model swap or schema change
//! lands as one PR touching one crate.

use serde_json::json;

use crate::client::JsonSchema;

pub mod hallucination {
    use super::*;

    pub const TEMPLATE: &str = include_str!("hallucination.md");

    pub fn schema() -> JsonSchema {
        JsonSchema {
            name: "HallucinationVerdict".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "grounded":   { "type": "boolean" },
                    "violations": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["grounded", "violations"],
                "additionalProperties": false
            }),
        }
    }

    pub fn build(profile: &str, docs: &str, user_input: &str, draft: &str) -> String {
        TEMPLATE
            .replace("{{PROFILE}}", profile)
            .replace("{{DOCS}}", docs)
            .replace("{{INPUT}}", user_input)
            .replace("{{DRAFT}}", draft)
    }
}

pub mod tone {
    use super::*;

    pub const TEMPLATE: &str = include_str!("tone.md");

    pub fn schema() -> JsonSchema {
        JsonSchema {
            name: "ToneVerdict".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "matches_target": { "type": "boolean" },
                    "detected_tone":  { "type": "string" },
                    "issues": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["matches_target", "detected_tone", "issues"],
                "additionalProperties": false
            }),
        }
    }

    pub fn build(target: &str, forbidden: &str, user_input: &str, draft: &str) -> String {
        TEMPLATE
            .replace("{{TONE_TARGET}}", target)
            .replace("{{TONE_FORBIDDEN}}", forbidden)
            .replace("{{INPUT}}", user_input)
            .replace("{{DRAFT}}", draft)
    }
}

pub mod authority {
    use super::*;

    pub const TEMPLATE: &str = include_str!("authority.md");

    pub fn schema() -> JsonSchema {
        JsonSchema {
            name: "AuthorityVerdict".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "within_authority":    { "type": "boolean" },
                    "forbidden_promises": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["within_authority", "forbidden_promises"],
                "additionalProperties": false
            }),
        }
    }

    pub fn build(can_promise: &str, cannot_promise: &str, user_input: &str, draft: &str) -> String {
        TEMPLATE
            .replace("{{CAN_PROMISE}}", can_promise)
            .replace("{{CANNOT_PROMISE}}", cannot_promise)
            .replace("{{INPUT}}", user_input)
            .replace("{{DRAFT}}", draft)
    }
}

pub mod semantic_policy {
    use super::*;

    pub const TEMPLATE: &str = include_str!("semantic_policy.md");

    pub fn schema() -> JsonSchema {
        JsonSchema {
            name: "SemanticPolicyVerdict".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "matched": { "type": "boolean" },
                    "confidence": {
                        "type": "number",
                        "minimum": 0,
                        "maximum": 1
                    },
                    "reason": { "type": "string" },
                    "evidence": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["matched", "confidence", "reason", "evidence"],
                "additionalProperties": false
            }),
        }
    }

    pub fn build(
        policy_id: &str,
        policy_description: &str,
        match_clause: &str,
        policy_action: &str,
        policy_severity: &str,
        event_summary: &str,
        text: &str,
    ) -> String {
        TEMPLATE
            .replace("{{POLICY_ID}}", policy_id)
            .replace("{{POLICY_DESCRIPTION}}", policy_description)
            .replace("{{MATCH_CLAUSE}}", match_clause)
            .replace("{{POLICY_ACTION}}", policy_action)
            .replace("{{POLICY_SEVERITY}}", policy_severity)
            .replace("{{EVENT_SUMMARY}}", event_summary)
            .replace("{{TEXT}}", text)
    }
}

pub mod harden_draft {
    use super::*;

    pub const TEMPLATE: &str = include_str!("harden_draft.md");

    fn policy_draft_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": [
                "id", "description", "match_type", "match_value",
                "action", "severity", "rewrite",
            ],
            "properties": {
                "id": { "type": "string" },
                "description": { "type": "string" },
                "match_type": { "type": "string", "enum": ["literal", "regex", "semantic"] },
                "match_value": { "type": "string" },
                "action": { "type": "string", "enum": ["block", "rewrite", "escalate"] },
                "severity": {
                    "type": "string",
                    "enum": ["low", "medium", "high", "critical"],
                },
                "rewrite": { "type": ["string", "null"] },
            },
        })
    }

    pub fn schema() -> JsonSchema {
        JsonSchema {
            name: "HardenDraftCandidate".into(),
            schema: json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["draft", "regex_backstop", "rationale"],
                "properties": {
                    "draft": policy_draft_schema(),
                    "regex_backstop": {
                        "type": ["string", "null"],
                        "description": "Optional high-precision regex class backstop."
                    },
                    "rationale": { "type": "string" },
                },
            }),
        }
    }

    pub fn build(
        policy_id: &str,
        harm_class: &str,
        agent_id: &str,
        workflow_requirements: &str,
        landed_evidence: &str,
        control_summary: &str,
    ) -> String {
        TEMPLATE
            .replace("{{POLICY_ID}}", policy_id)
            .replace("{{HARM_CLASS}}", harm_class)
            .replace("{{AGENT_ID}}", agent_id)
            .replace("{{WORKFLOW_REQUIREMENTS}}", workflow_requirements)
            .replace("{{LANDED_EVIDENCE}}", landed_evidence)
            .replace("{{CONTROL_SUMMARY}}", control_summary)
    }
}

pub mod trajectory_diagnostic {
    use super::*;

    pub const TEMPLATE: &str = include_str!("trajectory_diagnostic.md");

    pub fn schema() -> JsonSchema {
        JsonSchema {
            name: "TrajectoryDiagnostic".into(),
            schema: json!({
                "type": "object",
                "additionalProperties": false,
                "required": [
                    "summary",
                    "risk_source",
                    "failure_mode",
                    "harm_class",
                    "suggested_substrate",
                    "source_chain",
                    "confidence",
                ],
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "One concise root-cause explanation for the trajectory finding."
                    },
                    "risk_source": {
                        "type": ["string", "null"],
                        "description": "The unsafe source, actor, tool, memory, or policy gap that introduced the risk."
                    },
                    "failure_mode": {
                        "type": ["string", "null"],
                        "description": "Stable snake_case failure mode label."
                    },
                    "harm_class": {
                        "type": ["string", "null"],
                        "description": "Stable snake_case harm class label."
                    },
                    "suggested_substrate": {
                        "type": ["string", "null"],
                        "description": "Best hardening substrate: semantic_output, approval, param_source, label_policy, memory, provenance, or graph_rule."
                    },
                    "source_chain": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Ordered source-to-sink explanation. Do not invent event ids."
                    },
                    "confidence": {
                        "type": "number",
                        "minimum": 0,
                        "maximum": 1
                    }
                }
            }),
        }
    }

    pub fn build(
        finding_context: &str,
        baseline_diagnostic: &str,
        trajectory_events: &str,
    ) -> String {
        TEMPLATE
            .replace("{{FINDING_CONTEXT}}", finding_context)
            .replace("{{BASELINE_DIAGNOSTIC}}", baseline_diagnostic)
            .replace("{{TRAJECTORY_EVENTS}}", trajectory_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hallucination_template_substitutes_all_placeholders() {
        let out = hallucination::build("PROFILE_X", "DOCS_X", "INPUT_X", "DRAFT_X");
        assert!(out.contains("PROFILE_X"));
        assert!(out.contains("DOCS_X"));
        assert!(out.contains("INPUT_X"));
        assert!(out.contains("DRAFT_X"));
        assert!(!out.contains("{{PROFILE}}"));
        assert!(!out.contains("{{DOCS}}"));
        assert!(!out.contains("{{INPUT}}"));
        assert!(!out.contains("{{DRAFT}}"));
    }

    #[test]
    fn tone_template_substitutes_all_placeholders() {
        let out = tone::build("warm", "curt, sarcastic", "hi", "hello!");
        assert!(out.contains("warm"));
        assert!(out.contains("curt, sarcastic"));
        assert!(!out.contains("{{TONE_TARGET}}"));
        assert!(!out.contains("{{TONE_FORBIDDEN}}"));
    }

    #[test]
    fn authority_template_substitutes_all_placeholders() {
        let out = authority::build("respond in 24h", "refunds", "Q", "A");
        assert!(out.contains("respond in 24h"));
        assert!(out.contains("refunds"));
        assert!(!out.contains("{{CAN_PROMISE}}"));
        assert!(!out.contains("{{CANNOT_PROMISE}}"));
    }

    #[test]
    fn semantic_policy_template_substitutes_all_placeholders() {
        let out = semantic_policy::build(
            "tone-policy",
            "keep replies respectful",
            r#"{"semantic":"the agent insults the user"}"#,
            "block",
            "high",
            "kind: output.proposed\nagent_id: support",
            "you are dumb",
        );
        assert!(out.contains("tone-policy"));
        assert!(out.contains("keep replies respectful"));
        assert!(out.contains("the agent insults the user"));
        assert!(out.contains("block"));
        assert!(out.contains("high"));
        assert!(out.contains("output.proposed"));
        assert!(out.contains("you are dumb"));
        assert!(!out.contains("{{POLICY_ID}}"));
        assert!(!out.contains("{{POLICY_DESCRIPTION}}"));
        assert!(!out.contains("{{MATCH_CLAUSE}}"));
        assert!(!out.contains("{{POLICY_ACTION}}"));
        assert!(!out.contains("{{POLICY_SEVERITY}}"));
        assert!(!out.contains("{{EVENT_SUMMARY}}"));
        assert!(!out.contains("{{TEXT}}"));
    }

    #[test]
    fn harden_draft_template_substitutes_all_placeholders() {
        let out = harden_draft::build(
            "harden-agent-action",
            "workflow_integrity",
            "agent-1",
            "verify identity first",
            "#0 attack -> reply",
            "1 benign control",
        );
        assert!(out.contains("harden-agent-action"));
        assert!(out.contains("workflow_integrity"));
        assert!(out.contains("agent-1"));
        assert!(out.contains("verify identity first"));
        assert!(out.contains("#0 attack -> reply"));
        assert!(out.contains("1 benign control"));
        assert!(!out.contains("{{POLICY_ID}}"));
        assert!(!out.contains("{{HARM_CLASS}}"));
        assert!(!out.contains("{{AGENT_ID}}"));
        assert!(!out.contains("{{WORKFLOW_REQUIREMENTS}}"));
        assert!(!out.contains("{{LANDED_EVIDENCE}}"));
        assert!(!out.contains("{{CONTROL_SUMMARY}}"));
    }

    #[test]
    fn trajectory_diagnostic_template_substitutes_all_placeholders() {
        let out = trajectory_diagnostic::build(
            "finding context",
            r#"{"summary":"deterministic"}"#,
            "event summary",
        );
        assert!(out.contains("finding context"));
        assert!(out.contains(r#"{"summary":"deterministic"}"#));
        assert!(out.contains("event summary"));
        assert!(!out.contains("{{FINDING_CONTEXT}}"));
        assert!(!out.contains("{{BASELINE_DIAGNOSTIC}}"));
        assert!(!out.contains("{{TRAJECTORY_EVENTS}}"));
    }

    #[test]
    fn schemas_have_required_fields() {
        let s = hallucination::schema();
        assert_eq!(s.name, "HallucinationVerdict");
        let req = s.schema["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v == "grounded"));

        let s = tone::schema();
        assert_eq!(s.name, "ToneVerdict");

        let s = authority::schema();
        assert_eq!(s.name, "AuthorityVerdict");

        let s = semantic_policy::schema();
        assert_eq!(s.name, "SemanticPolicyVerdict");
        let req = s.schema["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v == "matched"));
        assert!(req.iter().any(|v| v == "confidence"));
        assert!(req.iter().any(|v| v == "reason"));
        assert!(req.iter().any(|v| v == "evidence"));

        let s = harden_draft::schema();
        assert_eq!(s.name, "HardenDraftCandidate");
        let req = s.schema["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v == "draft"));
        assert!(req.iter().any(|v| v == "regex_backstop"));
        assert!(req.iter().any(|v| v == "rationale"));

        let s = trajectory_diagnostic::schema();
        assert_eq!(s.name, "TrajectoryDiagnostic");
        let req = s.schema["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v == "summary"));
        assert!(req.iter().any(|v| v == "source_chain"));
        assert!(req.iter().any(|v| v == "confidence"));
    }
}
