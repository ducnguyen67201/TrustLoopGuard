use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::policy_ast::{Action, MatchClause, Matcher, Policy};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
}

impl ValidationIssue {
    pub(crate) fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("yaml parse: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("policy validation: {0}")]
    Validation(String),
}

pub fn load_str(src: &str) -> Result<Policy, PolicyError> {
    let policy: Policy = serde_yaml::from_str(src)?;
    if let Err(issues) = validate_policy(&policy) {
        return Err(PolicyError::Validation(format_issues(&issues)));
    }
    Ok(policy)
}

pub fn validate_policy(policy: &Policy) -> Result<(), Vec<ValidationIssue>> {
    let mut issues = vec![];

    validate_id(policy, &mut issues);
    validate_optional_text("description", policy.description.as_deref(), &mut issues);
    validate_scope_list("when.domains", &policy.when.domains, &mut issues);
    validate_scope_list("when.agents", &policy.when.agents, &mut issues);
    validate_match_clause("match", &policy.r#match, &mut issues);

    if matches!(policy.action, Action::Rewrite) {
        match policy.rewrite.as_deref().map(str::trim) {
            Some(s) if !s.is_empty() => {}
            _ => issues.push(ValidationIssue::new(
                "rewrite",
                "rewrite is required when action is rewrite",
            )),
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn validate_id(policy: &Policy, issues: &mut Vec<ValidationIssue>) {
    let id = policy.id.trim();
    if id.is_empty() {
        issues.push(ValidationIssue::new("id", "id is required"));
        return;
    }

    let valid = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        issues.push(ValidationIssue::new(
            "id",
            "id must use lowercase letters, numbers, '-' or '_'",
        ));
    }
}

fn validate_optional_text(path: &str, value: Option<&str>, issues: &mut Vec<ValidationIssue>) {
    if matches!(value, Some(s) if s.trim().is_empty()) {
        issues.push(ValidationIssue::new(path, "must not be empty when present"));
    }
}

fn validate_scope_list(path: &str, values: &[String], issues: &mut Vec<ValidationIssue>) {
    for (idx, value) in values.iter().enumerate() {
        if value.trim().is_empty() {
            issues.push(ValidationIssue::new(
                format!("{path}[{idx}]"),
                "must not be empty",
            ));
        }
    }
}

fn validate_match_clause(path: &str, clause: &MatchClause, issues: &mut Vec<ValidationIssue>) {
    match clause {
        MatchClause::Single(matcher) => validate_matcher(path, matcher, issues),
        MatchClause::Any { any } => {
            if any.is_empty() {
                issues.push(ValidationIssue::new(
                    "match.any",
                    "must contain at least one matcher",
                ));
            }
            for (idx, matcher) in any.iter().enumerate() {
                validate_matcher(&format!("{path}.any[{idx}]"), matcher, issues);
            }
        }
        MatchClause::All { all } => {
            if all.is_empty() {
                issues.push(ValidationIssue::new(
                    "match.all",
                    "must contain at least one matcher",
                ));
            }
            for (idx, matcher) in all.iter().enumerate() {
                validate_matcher(&format!("{path}.all[{idx}]"), matcher, issues);
            }
        }
    }
}

fn validate_matcher(path: &str, matcher: &Matcher, issues: &mut Vec<ValidationIssue>) {
    match matcher {
        Matcher::Literal(value) => {
            if value.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("{path}.literal"),
                    "literal matcher must not be empty",
                ));
            }
        }
        Matcher::Regex(pattern) => {
            if pattern.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("{path}.regex"),
                    "regex matcher must not be empty",
                ));
                return;
            }
            if let Err(e) = Regex::new(pattern) {
                issues.push(ValidationIssue::new(
                    format!("{path}.regex"),
                    format!("regex failed to compile: {e}"),
                ));
            }
        }
        Matcher::Semantic(value) => {
            if value.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("{path}.semantic"),
                    "semantic matcher must not be empty",
                ));
            }
        }
    }
}

pub(crate) fn format_issues(issues: &[ValidationIssue]) -> String {
    issues
        .iter()
        .map(|issue| format!("{}: {}", issue.path, issue.message))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_policy() {
        let yaml = r#"
id: refund-promise
description: Prevents unsupported refund promises
match:
  regex: "(?i)refund"
action: rewrite
rewrite: "I'll connect you with a teammate."
"#;
        let p = load_str(yaml).expect("parse");
        assert_eq!(p.id, "refund-promise");
        assert_eq!(
            p.description.as_deref(),
            Some("Prevents unsupported refund promises")
        );
    }

    #[test]
    fn accepts_canonical_scope_fields() {
        let yaml = r#"
id: scoped-policy
when:
  domains: [customer_support]
  agents: [acme-support-v3]
  channels: [chat, email]
match:
  literal: "refund"
action: block
"#;
        let p = load_str(yaml).expect("parse");
        assert_eq!(p.when.domains, vec!["customer_support"]);
        assert_eq!(p.when.agents, vec!["acme-support-v3"]);
        assert_eq!(p.when.channels.len(), 2);
    }

    #[test]
    fn accepts_legacy_channel_scope_field() {
        let yaml = r#"
id: legacy-channel-policy
when:
  channel: [voice, chat]
match:
  literal: "refund"
action: block
"#;
        let p = load_str(yaml).expect("parse");
        assert_eq!(p.when.channels.len(), 2);
    }

    #[test]
    fn rewrite_action_requires_rewrite_text() {
        let yaml = r#"
id: missing-rewrite
match:
  literal: "refund"
action: rewrite
"#;
        let err = load_str(yaml).unwrap_err().to_string();
        assert!(err.contains("rewrite is required"));
    }

    #[test]
    fn regex_matcher_must_compile() {
        let yaml = r#"
id: bad-regex
match:
  regex: "["
action: block
"#;
        let err = load_str(yaml).unwrap_err().to_string();
        assert!(err.contains("regex failed to compile"));
    }

    #[test]
    fn id_is_a_stable_slug() {
        let yaml = r#"
id: "Refund Promise"
match:
  literal: "refund"
action: block
"#;
        let err = load_str(yaml).unwrap_err().to_string();
        assert!(err.contains("lowercase letters"));
    }

    #[test]
    fn documented_examples_parse() {
        for (name, yaml) in [
            (
                "refund-guarantee",
                include_str!("../../../docs/policies/examples/refund-guarantee.yaml"),
            ),
            (
                "pii-block",
                include_str!("../../../docs/policies/examples/pii-block.yaml"),
            ),
            (
                "legal-escalation",
                include_str!("../../../docs/policies/examples/legal-escalation.yaml"),
            ),
            (
                "voice-disclosure",
                include_str!("../../../docs/policies/examples/voice-disclosure.yaml"),
            ),
        ] {
            load_str(yaml).unwrap_or_else(|e| panic!("documented example `{name}` failed: {e}"));
        }
    }
}
