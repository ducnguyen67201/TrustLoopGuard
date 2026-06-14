use tl_core::{GuardEvent, TriggeredPolicy, Verdict};
use tl_policy::{Action, MatchClause, Matcher, Policy};

#[derive(Debug, Clone)]
pub struct EventPolicyOutcome {
    pub triggered: Vec<TriggeredPolicy>,
    pub verdict: Option<Verdict>,
    pub reason: Option<String>,
    pub safe_output: Option<String>,
}

impl EventPolicyOutcome {
    pub fn empty() -> Self {
        Self {
            triggered: Vec::new(),
            verdict: None,
            reason: None,
            safe_output: None,
        }
    }
}

pub fn evaluate_content_policies<'a, I>(event: &GuardEvent, policies: I) -> EventPolicyOutcome
where
    I: IntoIterator<Item = &'a Policy>,
{
    let Some(text) = output_text(event) else {
        return EventPolicyOutcome::empty();
    };

    let mut outcome = EventPolicyOutcome::empty();
    for policy in policies {
        if !policy_scope_matches(policy, event) || !match_clause_matches(&policy.r#match, text) {
            continue;
        }

        outcome.triggered.push(TriggeredPolicy {
            id: policy.id.clone(),
            severity: policy.severity,
            reason: format!("policy `{}` matched", policy.id),
        });

        if outcome.verdict.is_none() {
            if let Some(verdict) = verdict_from_action(policy.action) {
                outcome.verdict = Some(verdict);
                outcome.reason = Some(format!("policy `{}` triggered", policy.id));
                outcome.safe_output = policy.rewrite.clone();
            }
        }
    }

    outcome
}

fn output_text(event: &GuardEvent) -> Option<&str> {
    if event.kind != tl_core::EventKind::OutputProposed {
        return None;
    }
    event.action.parameters.get("text")?.as_str()
}

fn policy_scope_matches(policy: &Policy, event: &GuardEvent) -> bool {
    if !policy.when.agents.is_empty()
        && !policy
            .when
            .agents
            .iter()
            .any(|agent| agent == &event.principal.agent_id)
    {
        return false;
    }

    if !policy.when.domains.is_empty() {
        let domain = event
            .context
            .get("domain")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("customer_support");
        if !policy
            .when
            .domains
            .iter()
            .any(|candidate| candidate == domain)
        {
            return false;
        }
    }

    if !policy.when.channels.is_empty() {
        let Some(channel) = event
            .context
            .get("channel")
            .and_then(serde_json::Value::as_str)
        else {
            return false;
        };
        if !policy
            .when
            .channels
            .iter()
            .any(|candidate| channel == channel_name(candidate))
        {
            return false;
        }
    }

    true
}

fn match_clause_matches(clause: &MatchClause, text: &str) -> bool {
    match clause {
        MatchClause::Any { any } => any.iter().any(|matcher| matcher_matches(matcher, text)),
        MatchClause::All { all } => all.iter().all(|matcher| matcher_matches(matcher, text)),
        MatchClause::Single(matcher) => matcher_matches(matcher, text),
    }
}

fn matcher_matches(matcher: &Matcher, text: &str) -> bool {
    match matcher {
        Matcher::Literal(value) => text.contains(value),
        Matcher::Regex(pattern) => regex::Regex::new(pattern)
            .map(|regex| regex.is_match(text))
            .unwrap_or(false),
        // Semantic matching is handled by the optional policy judge layer.
        Matcher::Semantic(_) => false,
    }
}

fn verdict_from_action(action: Action) -> Option<Verdict> {
    match action {
        Action::Allow => None,
        Action::Block => Some(Verdict::Block),
        Action::Rewrite => Some(Verdict::Rewrite),
        Action::Escalate => Some(Verdict::Escalate),
    }
}

fn channel_name(channel: &tl_core::Channel) -> &'static str {
    match channel {
        tl_core::Channel::Voice => "voice",
        tl_core::Channel::Chat => "chat",
        tl_core::Channel::Email => "email",
    }
}

#[cfg(test)]
mod tests {
    use tl_core::{Action as EventAction, EventKind, Principal};
    use tl_policy::load_str;

    use super::*;

    fn output_event(text: &str) -> GuardEvent {
        GuardEvent {
            kind: EventKind::OutputProposed,
            principal: Principal {
                workspace_id: "ws_1".into(),
                environment_id: "production".into(),
                agent_id: "agent-1".into(),
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: EventAction {
                operation: "output".into(),
                parameters: serde_json::json!({ "text": text }),
                side_effect: Some(tl_core::SideEffectClass::None),
            },
            sources: vec![],
            provenance: Default::default(),
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::json!({ "channel": "chat" }),
        }
    }

    #[test]
    fn literal_content_policy_blocks_output_event() {
        let policy = load_str(
            r#"
id: refund-guarantee
match:
  literal: guaranteed refund
action: block
severity: high
"#,
        )
        .unwrap();

        let outcome =
            evaluate_content_policies(&output_event("we offer a guaranteed refund"), &[policy]);

        assert_eq!(outcome.verdict, Some(Verdict::Block));
        assert_eq!(outcome.triggered[0].id, "refund-guarantee");
    }

    #[test]
    fn scoped_channel_must_match_event_context() {
        let policy = load_str(
            r#"
id: chat-only
when:
  channels: [email]
match:
  literal: guaranteed refund
action: block
"#,
        )
        .unwrap();

        let outcome =
            evaluate_content_policies(&output_event("we offer a guaranteed refund"), &[policy]);

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.verdict, None);
    }
}
