use async_trait::async_trait;
use tl_core::{GuardEvent, Severity, TriggeredPolicy, Verdict};
use tl_llm::{prompts::semantic_policy, JudgeKind, LlmRouter};
use tl_policy::{Action, MatchClause, Matcher, Policy};

const SEMANTIC_MATCH_CONFIDENCE: f64 = 0.85;
const SEMANTIC_AMBIGUOUS_CONFIDENCE: f64 = 0.55;

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

pub struct EventPolicyEvalCtx<'a> {
    pub tenant: &'a str,
    pub semantic_judge: Option<&'a dyn SemanticPolicyJudge>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticPolicyJudgeInput {
    pub tenant: String,
    pub policy_id: String,
    pub policy_description: String,
    pub match_clause: String,
    pub policy_action: String,
    pub policy_severity: String,
    pub event_summary: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticPolicyJudgeResult {
    Matched {
        confidence: f64,
        reason: String,
        evidence: Vec<String>,
    },
    NotMatched {
        confidence: f64,
        reason: String,
        evidence: Vec<String>,
    },
    Error(String),
}

#[async_trait]
pub trait SemanticPolicyJudge: Send + Sync {
    fn is_enabled(&self) -> bool;

    async fn judge_policy(&self, input: SemanticPolicyJudgeInput) -> SemanticPolicyJudgeResult;
}

#[async_trait]
impl SemanticPolicyJudge for LlmRouter {
    fn is_enabled(&self) -> bool {
        self.has_route(JudgeKind::SemanticPolicy)
    }

    async fn judge_policy(&self, input: SemanticPolicyJudgeInput) -> SemanticPolicyJudgeResult {
        let prompt = semantic_policy::build(
            &input.policy_id,
            &input.policy_description,
            &input.match_clause,
            &input.policy_action,
            &input.policy_severity,
            &input.event_summary,
            &input.text,
        );

        match self
            .judge(
                JudgeKind::SemanticPolicy,
                &input.tenant,
                &prompt,
                &semantic_policy::schema(),
            )
            .await
        {
            Ok(output) => semantic_result_from_json(output.json),
            Err(error) => SemanticPolicyJudgeResult::Error(error.to_string()),
        }
    }
}

pub async fn evaluate_event_policies<'a, I>(
    event: &GuardEvent,
    policies: I,
    ctx: EventPolicyEvalCtx<'_>,
) -> EventPolicyOutcome
where
    I: IntoIterator<Item = &'a Policy>,
{
    let Some(text) = output_text(event) else {
        return EventPolicyOutcome::empty();
    };

    let mut outcome = EventPolicyOutcome::empty();
    for policy in policies {
        if !policy_scope_matches(policy, event) {
            continue;
        }

        match match_clause_decision(&policy.r#match, text) {
            ClauseDecision::Matched => {
                record_trigger(
                    &mut outcome,
                    policy,
                    format!("policy `{}` matched", policy.id),
                );
            }
            ClauseDecision::NotMatched => continue,
            ClauseDecision::NeedsSemantic => {
                evaluate_semantic_policy(event, text, policy, &ctx, &mut outcome).await;
            }
        }
    }

    outcome
}

async fn evaluate_semantic_policy(
    event: &GuardEvent,
    text: &str,
    policy: &Policy,
    ctx: &EventPolicyEvalCtx<'_>,
    outcome: &mut EventPolicyOutcome,
) {
    let Some(judge) = ctx.semantic_judge else {
        tracing::debug!(policy_id = %policy.id, "semantic policy skipped: no judge configured");
        return;
    };
    if !judge.is_enabled() {
        tracing::debug!(policy_id = %policy.id, "semantic policy skipped: no judge route configured");
        return;
    }

    let input = semantic_judge_input(ctx.tenant, event, text, policy);
    match judge.judge_policy(input).await {
        SemanticPolicyJudgeResult::Matched {
            confidence,
            reason,
            evidence,
        } if confidence >= SEMANTIC_MATCH_CONFIDENCE => {
            let evidence = evidence_suffix(&evidence);
            record_trigger(
                outcome,
                policy,
                format!(
                    "semantic policy `{}` matched (confidence={confidence:.2}): {reason}{evidence}",
                    policy.id
                ),
            );
        }
        SemanticPolicyJudgeResult::Matched {
            confidence,
            reason,
            evidence,
        } if confidence >= SEMANTIC_AMBIGUOUS_CONFIDENCE && high_or_critical(policy.severity) => {
            let evidence = evidence_suffix(&evidence);
            record_trigger_with_verdict(
                outcome,
                policy,
                Verdict::Escalate,
                format!(
                    "semantic policy `{}` ambiguous (confidence={confidence:.2}): {reason}{evidence}",
                    policy.id
                ),
                None,
            );
        }
        SemanticPolicyJudgeResult::Matched {
            confidence, reason, ..
        } => {
            tracing::debug!(
                policy_id = %policy.id,
                confidence,
                reason = %reason,
                "semantic policy match below action threshold"
            );
        }
        SemanticPolicyJudgeResult::NotMatched {
            confidence, reason, ..
        } => {
            tracing::debug!(
                policy_id = %policy.id,
                confidence,
                reason = %reason,
                "semantic policy did not match"
            );
        }
        SemanticPolicyJudgeResult::Error(error) if high_or_critical(policy.severity) => {
            record_trigger_with_verdict(
                outcome,
                policy,
                Verdict::Escalate,
                format!(
                    "semantic policy judge unavailable for `{}`: {error}",
                    policy.id
                ),
                None,
            );
        }
        SemanticPolicyJudgeResult::Error(error) => {
            tracing::warn!(
                policy_id = %policy.id,
                severity = ?policy.severity,
                error = %error,
                "semantic policy judge unavailable"
            );
        }
    }
}

fn record_trigger(outcome: &mut EventPolicyOutcome, policy: &Policy, reason: String) {
    if let Some(verdict) = verdict_from_action(policy.action) {
        record_trigger_with_verdict(outcome, policy, verdict, reason, policy.rewrite.clone());
    } else {
        outcome.triggered.push(TriggeredPolicy {
            id: policy.id.clone(),
            severity: policy.severity,
            reason,
        });
    }
}

fn record_trigger_with_verdict(
    outcome: &mut EventPolicyOutcome,
    policy: &Policy,
    verdict: Verdict,
    reason: String,
    safe_output: Option<String>,
) {
    outcome.triggered.push(TriggeredPolicy {
        id: policy.id.clone(),
        severity: policy.severity,
        reason: reason.clone(),
    });

    if outcome
        .verdict
        .map(|current| verdict_rank(verdict) > verdict_rank(current))
        .unwrap_or(true)
    {
        outcome.verdict = Some(verdict);
        outcome.reason = Some(reason);
        outcome.safe_output = match verdict {
            Verdict::Rewrite => safe_output,
            _ => None,
        };
    }
}

fn verdict_rank(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::Allow => 0,
        Verdict::Rewrite => 1,
        Verdict::Escalate => 2,
        Verdict::Block => 3,
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClauseDecision {
    Matched,
    NotMatched,
    NeedsSemantic,
}

fn match_clause_decision(clause: &MatchClause, text: &str) -> ClauseDecision {
    match clause {
        MatchClause::Any { any } => {
            let mut needs_semantic = false;
            for matcher in any {
                match matcher_decision(matcher, text) {
                    ClauseDecision::Matched => return ClauseDecision::Matched,
                    ClauseDecision::NeedsSemantic => needs_semantic = true,
                    ClauseDecision::NotMatched => {}
                }
            }
            if needs_semantic {
                ClauseDecision::NeedsSemantic
            } else {
                ClauseDecision::NotMatched
            }
        }
        MatchClause::All { all } => {
            let mut needs_semantic = false;
            for matcher in all {
                match matcher_decision(matcher, text) {
                    ClauseDecision::Matched => {}
                    ClauseDecision::NeedsSemantic => needs_semantic = true,
                    ClauseDecision::NotMatched => return ClauseDecision::NotMatched,
                }
            }
            if needs_semantic {
                ClauseDecision::NeedsSemantic
            } else {
                ClauseDecision::Matched
            }
        }
        MatchClause::Single(matcher) => matcher_decision(matcher, text),
    }
}

fn matcher_decision(matcher: &Matcher, text: &str) -> ClauseDecision {
    match matcher {
        Matcher::Literal(value) => {
            if text.contains(value) {
                ClauseDecision::Matched
            } else {
                ClauseDecision::NotMatched
            }
        }
        Matcher::Regex(pattern) => {
            if regex::Regex::new(pattern)
                .map(|regex| regex.is_match(text))
                .unwrap_or(false)
            {
                ClauseDecision::Matched
            } else {
                ClauseDecision::NotMatched
            }
        }
        Matcher::Semantic(_) => ClauseDecision::NeedsSemantic,
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

fn semantic_judge_input(
    tenant: &str,
    event: &GuardEvent,
    text: &str,
    policy: &Policy,
) -> SemanticPolicyJudgeInput {
    SemanticPolicyJudgeInput {
        tenant: tenant.to_string(),
        policy_id: policy.id.clone(),
        policy_description: policy.description.clone().unwrap_or_default(),
        match_clause: serde_json::to_string(&policy.r#match).unwrap_or_else(|_| "<invalid>".into()),
        policy_action: format!("{:?}", policy.action).to_ascii_lowercase(),
        policy_severity: format!("{:?}", policy.severity).to_ascii_lowercase(),
        event_summary: event_summary(event),
        text: text.to_string(),
    }
}

fn event_summary(event: &GuardEvent) -> String {
    let channel = event
        .context
        .get("channel")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let domain = event
        .context
        .get("domain")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    format!(
        "kind: {:?}\nagent_id: {}\noperation: {}\nchannel: {channel}\ndomain: {domain}",
        event.kind, event.principal.agent_id, event.action.operation
    )
}

fn semantic_result_from_json(json: serde_json::Value) -> SemanticPolicyJudgeResult {
    let Some(matched) = json.get("matched").and_then(serde_json::Value::as_bool) else {
        return SemanticPolicyJudgeResult::Error(
            "semantic policy judge returned malformed `matched`".into(),
        );
    };
    let Some(confidence) = json
        .get("confidence")
        .and_then(serde_json::Value::as_f64)
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
    else {
        return SemanticPolicyJudgeResult::Error(
            "semantic policy judge returned malformed `confidence`".into(),
        );
    };
    let Some(reason) = json.get("reason").and_then(serde_json::Value::as_str) else {
        return SemanticPolicyJudgeResult::Error(
            "semantic policy judge returned malformed `reason`".into(),
        );
    };
    let Some(evidence) = json_string_array(json.get("evidence")) else {
        return SemanticPolicyJudgeResult::Error(
            "semantic policy judge returned malformed `evidence`".into(),
        );
    };

    if matched {
        SemanticPolicyJudgeResult::Matched {
            confidence,
            reason: reason.to_string(),
            evidence,
        }
    } else {
        SemanticPolicyJudgeResult::NotMatched {
            confidence,
            reason: reason.to_string(),
            evidence,
        }
    }
}

fn json_string_array(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(String::from))
        .collect()
}

fn evidence_suffix(evidence: &[String]) -> String {
    if evidence.is_empty() {
        String::new()
    } else {
        format!("; evidence: {}", evidence.join("; "))
    }
}

fn high_or_critical(severity: Severity) -> bool {
    matches!(severity, Severity::High | Severity::Critical)
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

        async fn judge_policy(&self, input: SemanticPolicyJudgeInput) -> SemanticPolicyJudgeResult {
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
    async fn stronger_policy_verdict_wins_regardless_of_order() {
        let rewrite = load_str(
            r#"
id: rewrite-risky
match:
  literal: risky claim
action: rewrite
rewrite: safer reply
severity: medium
"#,
        )
        .unwrap();
        let block = load_str(
            r#"
id: block-risky
match:
  literal: risky claim
action: block
severity: high
"#,
        )
        .unwrap();

        let outcome = evaluate_event_policies(
            &output_event("this is a risky claim"),
            &[rewrite, block],
            eval_ctx(None),
        )
        .await;

        assert_eq!(outcome.verdict, Some(Verdict::Block));
        assert!(outcome.reason.unwrap().contains("block-risky"));
        assert_eq!(outcome.safe_output, None);
        assert_eq!(outcome.triggered.len(), 2);
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

        let outcome = evaluate_event_policies(
            &output_event("you are dumb"),
            &[policy],
            eval_ctx(Some(&judge)),
        )
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

        let outcome =
            evaluate_event_policies(&output_event("you are dumb"), &[policy], eval_ctx(None)).await;

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

        let outcome = evaluate_event_policies(
            &output_event("maybe curt"),
            &[policy],
            eval_ctx(Some(&judge)),
        )
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

        let outcome = evaluate_event_policies(
            &output_event("you should sue"),
            &[policy],
            eval_ctx(Some(&judge)),
        )
        .await;

        assert_eq!(outcome.verdict, Some(Verdict::Escalate));
        assert_eq!(outcome.triggered[0].id, "legal-advice");
        assert!(outcome.reason.unwrap().contains("judge unavailable"));
    }

    #[test]
    fn malformed_semantic_judge_json_returns_error() {
        let result = semantic_result_from_json(serde_json::json!({
            "matched": true,
            "reason": "missing confidence and evidence"
        }));

        assert!(matches!(result, SemanticPolicyJudgeResult::Error(_)));
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

        let outcome =
            evaluate_event_policies(&tool_event(), &[policy], eval_ctx(Some(&judge))).await;

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.verdict, None);
        assert_eq!(judge.calls(), 0);
    }
}
