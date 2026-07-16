//! Deterministic evaluation of typed tool policies into authorization evidence.

use sha2::{Digest, Sha256};
use tl_core::{
    ActionGrantScope, AuthorityRequirement, AuthorizationCapabilityId, AuthorizationEffect,
    AuthorizationFinding, AuthorizationGrantScope, AuthorizationSubject, ShellActionParameters,
};
use tl_policy::{
    FamilyPolicy, ToolMatchClause, ToolMatcher, ToolPolicy, ToolSelector, ToolValueMatcher,
};

use crate::shell_command::{analyze_shell_command, ShellAnalysis, ShellAnalysisStatus};

#[derive(Debug, Clone, Default)]
pub struct ToolPolicyOutcome {
    pub findings: Vec<AuthorizationFinding>,
    pub requirements: Vec<AuthorityRequirement>,
    pub policy_versions: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ToolPolicyError {
    #[error("tool policy evaluation requires a tool subject")]
    DomainMismatch,
    #[error("invalid shell action parameters: {0}")]
    InvalidShellParameters(String),
    #[error("invalid tool policy regex: {0}")]
    InvalidRegex(String),
    #[error("invalid tool authorization capability: {0}")]
    InvalidCapability(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchResult {
    Matched,
    NotMatched,
    Unknown,
}

pub fn evaluate_tool_policies<'a, I>(
    principal_id: &str,
    subject: &AuthorizationSubject,
    families: I,
) -> Result<ToolPolicyOutcome, ToolPolicyError>
where
    I: IntoIterator<Item = &'a FamilyPolicy>,
{
    let AuthorizationSubject::Tool {
        operation,
        tool_identity,
        parameters,
        side_effect,
        ..
    } = subject
    else {
        return Err(ToolPolicyError::DomainMismatch);
    };

    let scoped = families
        .into_iter()
        .filter_map(|family| {
            let FamilyPolicy::Tool(policy) = family else {
                return None;
            };
            scope_matches(policy, principal_id, operation, tool_identity, *side_effect)
                .then_some(policy)
        })
        .collect::<Vec<_>>();
    if scoped.is_empty() {
        return Ok(ToolPolicyOutcome::default());
    }

    let needs_facts = scoped
        .iter()
        .any(|policy| clause_uses_facts(&policy.r#match));
    let shell_parameters = needs_facts
        .then(|| serde_json::from_value::<ShellActionParameters>(parameters.clone()))
        .transpose()
        .map_err(|error| ToolPolicyError::InvalidShellParameters(error.to_string()))?;
    let analysis = shell_parameters.as_ref().map(analyze_shell_command);
    let command_digest = shell_parameters
        .as_ref()
        .map(|parameters| sha256(&parameters.command));
    let capability = AuthorizationCapabilityId::parse(format!(
        "tool:{}/{}",
        tool_identity.server_id.to_ascii_lowercase(),
        tool_identity.tool_name.to_ascii_lowercase()
    ))
    .map_err(|message| ToolPolicyError::InvalidCapability(message.into()))?;

    let mut outcome = ToolPolicyOutcome::default();
    for policy in scoped {
        let result = match_clause(&policy.r#match, parameters, analysis.as_ref())?;
        let effect = match result {
            MatchResult::Matched => policy.action,
            MatchResult::Unknown => AuthorizationEffect::Defer,
            MatchResult::NotMatched => continue,
        };
        let requirement_id = (effect == AuthorizationEffect::RequireApproval)
            .then(|| format!("tool-policy:{}", policy.id));
        let evidence = serde_json::json!({
            "analysis_status": analysis.as_ref().map(|value| match value.status {
                ShellAnalysisStatus::Complete => "complete",
                ShellAnalysisStatus::Partial => "partial",
                ShellAnalysisStatus::Unavailable => "unavailable",
            }),
            "command_sha256": command_digest,
            "match_status": match result {
                MatchResult::Matched => "matched",
                MatchResult::Unknown => "unknown",
                MatchResult::NotMatched => "not_matched",
            },
        });
        outcome.findings.push(AuthorizationFinding {
            id: format!("tool-policy:{}", policy.id),
            source: "tool_policy".into(),
            effect,
            reason: if result == MatchResult::Unknown {
                format!(
                    "tool policy `{}` could not be fully evaluated because shell analysis was incomplete",
                    policy.id
                )
            } else {
                policy.reason.clone()
            },
            severity: policy.severity,
            policy_id: Some(policy.id.clone()),
            requirement_id: requirement_id.clone(),
            remediation: policy.remediation.clone(),
            evidence,
        });
        if let Some(requirement_id) = requirement_id {
            outcome.requirements.push(AuthorityRequirement {
                id: requirement_id,
                capability: capability.clone(),
                required_scope: exact_scope(subject)?,
                approver_roles: if policy.approver_roles.is_empty() {
                    vec!["owner".into(), "admin".into()]
                } else {
                    policy.approver_roles.clone()
                },
                reason: policy.reason.clone(),
                reusable_allowed: false,
                max_grant_ttl_seconds: Some(policy.max_grant_ttl_seconds.unwrap_or(900)),
            });
        }
        outcome.policy_versions.push(policy.id.clone());
    }
    outcome.policy_versions.sort();
    outcome.policy_versions.dedup();
    Ok(outcome)
}

fn scope_matches(
    policy: &ToolPolicy,
    principal_id: &str,
    operation: &str,
    identity: &tl_core::ToolIdentity,
    side_effect: tl_core::SideEffectClass,
) -> bool {
    let when = &policy.when;
    (when.agents.is_empty() || when.agents.iter().any(|value| value == principal_id))
        && (when.operations.is_empty() || when.operations.iter().any(|value| value == operation))
        && (when.side_effects.is_empty() || when.side_effects.contains(&side_effect))
        && (when.tools.is_empty()
            || when
                .tools
                .iter()
                .any(|selector| selector_matches(selector, identity)))
}

fn selector_matches(selector: &ToolSelector, identity: &tl_core::ToolIdentity) -> bool {
    selector
        .server_id
        .as_ref()
        .map_or(true, |value| value == &identity.server_id)
        && selector
            .tool_name
            .as_ref()
            .map_or(true, |value| value == &identity.tool_name)
        && selector
            .schema_hash
            .as_ref()
            .map_or(true, |value| value == &identity.schema_hash)
}

fn clause_uses_facts(clause: &ToolMatchClause) -> bool {
    match clause {
        ToolMatchClause::Single(matcher) => matches!(matcher, ToolMatcher::Fact { .. }),
        ToolMatchClause::Any { any } => any
            .iter()
            .any(|item| matches!(item, ToolMatcher::Fact { .. })),
        ToolMatchClause::All { all } => all
            .iter()
            .any(|item| matches!(item, ToolMatcher::Fact { .. })),
    }
}

fn match_clause(
    clause: &ToolMatchClause,
    parameters: &serde_json::Value,
    analysis: Option<&ShellAnalysis>,
) -> Result<MatchResult, ToolPolicyError> {
    match clause {
        ToolMatchClause::Single(matcher) => match_one(matcher, parameters, analysis),
        ToolMatchClause::Any { any } => {
            let mut unknown = false;
            for matcher in any {
                match match_one(matcher, parameters, analysis)? {
                    MatchResult::Matched => return Ok(MatchResult::Matched),
                    MatchResult::Unknown => unknown = true,
                    MatchResult::NotMatched => {}
                }
            }
            Ok(if unknown {
                MatchResult::Unknown
            } else {
                MatchResult::NotMatched
            })
        }
        ToolMatchClause::All { all } => {
            let mut unknown = false;
            for matcher in all {
                match match_one(matcher, parameters, analysis)? {
                    MatchResult::NotMatched => return Ok(MatchResult::NotMatched),
                    MatchResult::Unknown => unknown = true,
                    MatchResult::Matched => {}
                }
            }
            Ok(if unknown {
                MatchResult::Unknown
            } else {
                MatchResult::Matched
            })
        }
    }
}

fn match_one(
    matcher: &ToolMatcher,
    parameters: &serde_json::Value,
    analysis: Option<&ShellAnalysis>,
) -> Result<MatchResult, ToolPolicyError> {
    match matcher {
        ToolMatcher::Fact { fact } => {
            let Some(analysis) = analysis else {
                return Ok(MatchResult::Unknown);
            };
            if let Some(values) = analysis.facts.get(&fact.key) {
                for value in values {
                    if value_matches(&fact.value, value)? == MatchResult::Matched {
                        return Ok(MatchResult::Matched);
                    }
                }
            }
            if analysis.status == ShellAnalysisStatus::Complete {
                Ok(MatchResult::NotMatched)
            } else {
                Ok(MatchResult::Unknown)
            }
        }
        ToolMatcher::Parameter { parameter } => {
            let Some(value) = parameters.pointer(&parameter.path) else {
                return Ok(MatchResult::NotMatched);
            };
            let value = scalar_string(value);
            value_matches(&parameter.value, &value)
        }
    }
}

fn value_matches(
    value_matcher: &ToolValueMatcher,
    value: &str,
) -> Result<MatchResult, ToolPolicyError> {
    let matched = if let Some(expected) = value_matcher.equals.as_deref() {
        value == expected
    } else if !value_matcher.one_of.is_empty() {
        value_matcher
            .one_of
            .iter()
            .any(|expected| expected == value)
    } else if let Some(pattern) = value_matcher.regex.as_deref() {
        regex::Regex::new(pattern)
            .map_err(|error| ToolPolicyError::InvalidRegex(error.to_string()))?
            .is_match(value)
    } else {
        false
    };
    Ok(if matched {
        MatchResult::Matched
    } else {
        MatchResult::NotMatched
    })
}

fn scalar_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn exact_scope(subject: &AuthorizationSubject) -> Result<AuthorizationGrantScope, ToolPolicyError> {
    let AuthorizationSubject::Tool {
        operation,
        tool_identity,
        parameters,
        side_effect,
        ..
    } = subject
    else {
        return Err(ToolPolicyError::DomainMismatch);
    };
    Ok(AuthorizationGrantScope::Action(ActionGrantScope {
        operations: vec![operation.clone()],
        side_effects: vec![*side_effect],
        server_id: Some(tool_identity.server_id.clone()),
        tool_name: Some(tool_identity.tool_name.clone()),
        schema_hash: Some(tool_identity.schema_hash.clone()),
        parameters: Some(parameters.clone()),
        allowed_destinations: Vec::new(),
        maximum_data_confidentiality: None,
        minimum_source_trust: None,
    }))
}

fn sha256(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}
