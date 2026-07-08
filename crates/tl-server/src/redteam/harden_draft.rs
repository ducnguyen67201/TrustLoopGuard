use async_trait::async_trait;
use serde::Deserialize;
use tl_core::{PolicyAction, PolicyDraft, PolicyMatchType, WorkflowRequirement};
use tl_llm::{prompts::harden_draft, JudgeKind, LlmRouter};
use tl_policy::policy_ast::WhenClause;
use tl_policy::synthesis::{Candidate, HarmKind};
use tl_policy::{validate_policy, Action, MatchClause, Matcher, Policy};

pub struct HardenDraftInput<'a> {
    pub tenant: &'a str,
    pub policy_id: &'a str,
    pub harm: HarmKind,
    pub agent_id: Option<&'a str>,
    pub rep_attack: &'a str,
    pub rep_goal: &'a str,
    pub replies: &'a [String],
    pub evidence_seqs: &'a [i32],
    pub controls_count: usize,
    pub workflow_requirements: &'a [WorkflowRequirement],
    pub when: WhenClause,
    pub owner_agent_id: Option<String>,
}

pub struct HardenDraft {
    pub candidate: Candidate,
    #[allow(dead_code)]
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardenDraftError {
    Disabled,
    Provider(String),
    Invalid(String),
}

#[async_trait]
pub trait HardenDrafter: Send + Sync {
    fn is_enabled(&self) -> bool;

    async fn draft(&self, input: HardenDraftInput<'_>) -> Result<HardenDraft, HardenDraftError>;
}

#[async_trait]
impl HardenDrafter for LlmRouter {
    fn is_enabled(&self) -> bool {
        self.has_route(JudgeKind::HardenDraft)
    }

    async fn draft(&self, input: HardenDraftInput<'_>) -> Result<HardenDraft, HardenDraftError> {
        if !self.is_enabled() {
            return Err(HardenDraftError::Disabled);
        }

        let prompt = harden_draft::build(
            input.policy_id,
            harm_name(input.harm),
            input.agent_id.unwrap_or("<none>"),
            &workflow_requirements(input.workflow_requirements),
            &landed_evidence(
                input.rep_attack,
                input.rep_goal,
                input.replies,
                input.evidence_seqs,
            ),
            &format!("{} benign control replies available", input.controls_count),
        );
        let output = self
            .judge(
                JudgeKind::HardenDraft,
                input.tenant,
                &prompt,
                &harden_draft::schema(),
            )
            .await
            .map_err(|error| HardenDraftError::Provider(error.to_string()))?;
        let decoded: HardenDraftOutput = serde_json::from_value(output.json)
            .map_err(|error| HardenDraftError::Invalid(error.to_string()))?;
        let policy = policy_from_draft(
            input.policy_id,
            decoded.draft,
            decoded.regex_backstop,
            input.when,
            input.owner_agent_id,
        )?;
        Ok(HardenDraft {
            candidate: Candidate {
                policy,
                substrate: "semantic_output",
            },
            rationale: decoded.rationale,
        })
    }
}

#[derive(Debug, Deserialize)]
struct HardenDraftOutput {
    draft: PolicyDraft,
    #[serde(default)]
    regex_backstop: Option<String>,
    rationale: String,
}

fn policy_from_draft(
    policy_id: &str,
    draft: PolicyDraft,
    regex_backstop: Option<String>,
    when: WhenClause,
    owner_agent_id: Option<String>,
) -> Result<Policy, HardenDraftError> {
    let matcher = matcher_from_draft(draft.match_type, draft.match_value);
    let r#match = match regex_backstop.and_then(non_empty) {
        Some(backstop) => MatchClause::Any {
            any: vec![matcher, Matcher::Regex(backstop)],
        },
        None => MatchClause::Single(matcher),
    };
    let policy = Policy {
        id: policy_id.to_string(),
        description: Some(draft.description),
        when,
        r#match,
        action: action_from_draft(draft.action),
        rewrite: draft.rewrite,
        severity: draft.severity,
        owner_agent_id,
    };
    validate_policy(&policy).map_err(|issues| {
        let message = issues
            .into_iter()
            .map(|issue| format!("{}: {}", issue.path, issue.message))
            .collect::<Vec<_>>()
            .join("; ");
        HardenDraftError::Invalid(message)
    })?;
    Ok(policy)
}

fn matcher_from_draft(match_type: PolicyMatchType, value: String) -> Matcher {
    match match_type {
        PolicyMatchType::Literal => Matcher::Literal(value),
        PolicyMatchType::Regex => Matcher::Regex(value),
        PolicyMatchType::Semantic => Matcher::Semantic(value),
    }
}

fn action_from_draft(action: PolicyAction) -> Action {
    match action {
        PolicyAction::Block => Action::Block,
        PolicyAction::Rewrite => Action::Rewrite,
        PolicyAction::Escalate => Action::Escalate,
    }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn harm_name(harm: HarmKind) -> &'static str {
    match harm {
        HarmKind::Credential => "credential",
        HarmKind::Pii => "pii",
        HarmKind::SystemPrompt => "system_prompt",
        HarmKind::WorkflowIntegrity => "workflow_integrity",
        HarmKind::ActionClaim => "action_claim",
        HarmKind::ProtectedInfo => "protected_info",
    }
}

fn workflow_requirements(requirements: &[WorkflowRequirement]) -> String {
    if requirements.is_empty() {
        return "(none)".into();
    }
    requirements
        .iter()
        .map(|requirement| {
            format!(
                "- {} | required before: {} | sensitive steps: {}",
                requirement.name,
                list_or_none(&requirement.required_before),
                list_or_none(&requirement.sensitive_steps)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn landed_evidence(
    rep_attack: &str,
    rep_goal: &str,
    replies: &[String],
    evidence_seqs: &[i32],
) -> String {
    replies
        .iter()
        .enumerate()
        .map(|(idx, reply)| {
            let seq = evidence_seqs.get(idx).copied().unwrap_or(idx as i32);
            format!("#{seq}\nattack: {rep_attack}\ngoal: {rep_goal}\nreply: {reply}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".into()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::Severity;

    #[test]
    fn draft_conversion_enforces_stable_policy_id_and_regex_backstop() {
        let policy = policy_from_draft(
            "harden-agent-credential",
            PolicyDraft {
                id: "model-tried-other-id".into(),
                description: "Blocks credential leaks.".into(),
                match_type: PolicyMatchType::Semantic,
                match_value: "reply discloses a credential".into(),
                action: PolicyAction::Block,
                severity: Severity::Critical,
                rewrite: None,
            },
            Some("(?i)sk-[a-z0-9]{6,}".into()),
            WhenClause::default(),
            Some("agent-1".into()),
        )
        .expect("policy validates");

        assert_eq!(policy.id, "harden-agent-credential");
        assert_eq!(policy.owner_agent_id.as_deref(), Some("agent-1"));
        let MatchClause::Any { any } = policy.r#match else {
            panic!("expected semantic matcher plus regex backstop");
        };
        assert!(any
            .iter()
            .any(|matcher| matches!(matcher, Matcher::Semantic(_))));
        assert!(any
            .iter()
            .any(|matcher| matches!(matcher, Matcher::Regex(_))));
    }

    #[test]
    fn draft_conversion_rejects_invalid_regex_backstop() {
        let error = policy_from_draft(
            "harden-agent-invalid",
            PolicyDraft {
                id: "harden-agent-invalid".into(),
                description: "Invalid regex.".into(),
                match_type: PolicyMatchType::Semantic,
                match_value: "reply discloses a credential".into(),
                action: PolicyAction::Block,
                severity: Severity::High,
                rewrite: None,
            },
            Some("(".into()),
            WhenClause::default(),
            None,
        )
        .expect_err("invalid regex should fail validation");

        assert!(matches!(error, HardenDraftError::Invalid(_)));
    }
}
