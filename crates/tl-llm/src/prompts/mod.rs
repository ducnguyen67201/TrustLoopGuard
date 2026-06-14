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
    }
}
