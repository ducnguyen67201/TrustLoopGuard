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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
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

    fn tool_event() -> GuardEvent {
        let mut event = output_event("not relevant");
        event.kind = EventKind::ToolCallProposed;
        event.action.operation = "send_email".into();
        event.action.parameters = serde_json::json!({ "recipient": "a@example.com" });
        event
    }

    #[derive(Default)]
    struct RecordingJudge {
        enabled: bool,
        result: Mutex<Option<SemanticPolicyJudgeResult>>,
        calls: AtomicUsize,
        inputs: Mutex<Vec<SemanticPolicyJudgeInput>>,
    }

    impl RecordingJudge {
        fn enabled_with(result: SemanticPolicyJudgeResult) -> Self {
            Self {
                enabled: true,
                result: Mutex::new(Some(result)),
                calls: AtomicUsize::new(0),
                inputs: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl SemanticPolicyJudge for RecordingJudge {
        fn is_enabled(&self) -> bool {
            self.enabled
        }

        async fn judge_policy(
            &self,
            input: SemanticPolicyJudgeInput,
        ) -> SemanticPolicyJudgeResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inputs.lock().unwrap().push(input);
            self.result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| SemanticPolicyJudgeResult::Error("missing canned result".into()))
        }
    }

    fn eval_ctx<'a>(judge: Option<&'a dyn SemanticPolicyJudge>) -> EventPolicyEvalCtx<'a> {
        EventPolicyEvalCtx {
            tenant: "ws_1",
            semantic_judge: judge,
        }
    }

    #[tokio::test]
    async fn literal_content_policy_blocks_output_event() {
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

        let outcome = evaluate_event_policies(
            &output_event("we offer a guaranteed refund"),
            &[policy],
            eval_ctx(None),
        )
        .await;

        assert_eq!(outcome.verdict, Some(Verdict::Block));
        assert_eq!(outcome.triggered[0].id, "refund-guarantee");
    }

    #[tokio::test]
    async fn scoped_channel_must_match_event_context() {
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

        let outcome = evaluate_event_policies(
            &output_event("we offer a guaranteed refund"),
            &[policy],
            eval_ctx(None),
        )
        .await;

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.verdict, None);
    }

    #[tokio::test]
    async fn semantic_policy_match_blocks_output_event() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: block
severity: high
"#,
        )
        .unwrap();
        let judge = RecordingJudge::enabled_with(SemanticPolicyJudgeResult::Matched {
            confidence: 0.94,
            reason: "direct insult".into(),
            evidence: vec!["you are dumb".into()],
        });

        let outcome =
            evaluate_event_policies(&output_event("you are dumb"), &[policy], eval_ctx(Some(&judge)))
                .await;

        assert_eq!(outcome.verdict, Some(Verdict::Block));
        assert_eq!(outcome.triggered[0].id, "respectful-tone");
        assert!(outcome.triggered[0].reason.contains("confidence=0.94"));
        assert_eq!(judge.calls(), 1);
    }

    #[tokio::test]
    async fn semantic_policy_without_judge_route_does_not_trigger() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: block
severity: high
"#,
        )
        .unwrap();

        let outcome = evaluate_event_policies(&output_event("you are dumb"), &[policy], eval_ctx(None))
            .await;

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.verdict, None);
    }

    #[tokio::test]
    async fn semantic_low_confidence_match_does_not_trigger() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: block
severity: high
"#,
        )
        .unwrap();
        let judge = RecordingJudge::enabled_with(SemanticPolicyJudgeResult::Matched {
            confidence: 0.40,
            reason: "weak signal".into(),
            evidence: vec![],
        });

        let outcome =
            evaluate_event_policies(&output_event("maybe curt"), &[policy], eval_ctx(Some(&judge)))
                .await;

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.verdict, None);
        assert_eq!(judge.calls(), 1);
    }

    #[tokio::test]
    async fn semantic_ambiguous_high_severity_escalates() {
        let policy = load_str(
            r#"
id: legal-advice
match:
  semantic: "the agent gives legal advice"
action: block
severity: high
"#,
        )
        .unwrap();
        let judge = RecordingJudge::enabled_with(SemanticPolicyJudgeResult::Matched {
            confidence: 0.70,
            reason: "possibly legal interpretation".into(),
            evidence: vec!["you should sue".into()],
        });

        let outcome = evaluate_event_policies(
            &output_event("you should sue them"),
            &[policy],
            eval_ctx(Some(&judge)),
        )
        .await;

        assert_eq!(outcome.verdict, Some(Verdict::Escalate));
        assert_eq!(outcome.triggered[0].id, "legal-advice");
        assert!(outcome.reason.unwrap().contains("ambiguous"));
    }

    #[tokio::test]
    async fn semantic_judge_error_high_severity_escalates() {
        let policy = load_str(
            r#"
id: legal-advice
match:
  semantic: "the agent gives legal advice"
action: block
severity: critical
"#,
        )
        .unwrap();
        let judge = RecordingJudge::enabled_with(SemanticPolicyJudgeResult::Error(
            "provider timeout".into(),
        ));

        let outcome =
            evaluate_event_policies(&output_event("you should sue"), &[policy], eval_ctx(Some(&judge)))
                .await;

        assert_eq!(outcome.verdict, Some(Verdict::Escalate));
        assert_eq!(outcome.triggered[0].id, "legal-advice");
        assert!(outcome.reason.unwrap().contains("judge unavailable"));
    }

    #[tokio::test]
    async fn any_literal_match_does_not_call_semantic_judge() {
        let policy = load_str(
            r#"
id: refund-guarantee
match:
  any:
    - literal: guaranteed refund
    - semantic: "the agent guarantees an outcome"
action: block
severity: high
"#,
        )
        .unwrap();
        let judge = Arc::new(RecordingJudge::enabled_with(
            SemanticPolicyJudgeResult::Error("should not be called".into()),
        ));

        let outcome = evaluate_event_policies(
            &output_event("we offer a guaranteed refund"),
            &[policy],
            eval_ctx(Some(judge.as_ref())),
        )
        .await;

        assert_eq!(outcome.verdict, Some(Verdict::Block));
        assert_eq!(judge.calls(), 0);
    }

    #[tokio::test]
    async fn all_literal_miss_does_not_call_semantic_judge() {
        let policy = load_str(
            r#"
id: refund-guarantee
match:
  all:
    - literal: refund
    - semantic: "the agent guarantees an outcome"
action: block
severity: high
"#,
        )
        .unwrap();
        let judge = Arc::new(RecordingJudge::enabled_with(
            SemanticPolicyJudgeResult::Error("should not be called".into()),
        ));

        let outcome = evaluate_event_policies(
            &output_event("we can look into this"),
            &[policy],
            eval_ctx(Some(judge.as_ref())),
        )
        .await;

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.verdict, None);
        assert_eq!(judge.calls(), 0);
    }

    #[tokio::test]
    async fn non_output_event_does_not_evaluate_content_policy() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: block
severity: high
"#,
        )
        .unwrap();
        let judge = RecordingJudge::enabled_with(SemanticPolicyJudgeResult::Matched {
            confidence: 0.99,
            reason: "not relevant".into(),
            evidence: vec![],
        });

        let outcome = evaluate_event_policies(&tool_event(), &[policy], eval_ctx(Some(&judge))).await;

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.verdict, None);
        assert_eq!(judge.calls(), 0);
    }
}
