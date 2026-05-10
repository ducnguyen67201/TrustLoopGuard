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
    fn schemas_have_required_fields() {
        let s = hallucination::schema();
        assert_eq!(s.name, "HallucinationVerdict");
        let req = s.schema["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v == "grounded"));

        let s = tone::schema();
        assert_eq!(s.name, "ToneVerdict");

        let s = authority::schema();
        assert_eq!(s.name, "AuthorityVerdict");
    }
}
