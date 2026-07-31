use std::collections::HashSet;

use async_trait::async_trait;
use tl_core::{AuthorizationEffect, GuardEvent, Severity, TriggeredPolicy};
use tl_llm::{prompts::semantic_policy, JudgeKind, LlmCallAudit, LlmRouter};
use tl_policy::{MatchClause, Matcher, Policy};

const SEMANTIC_MATCH_CONFIDENCE: f64 = 0.85;
const SEMANTIC_AMBIGUOUS_CONFIDENCE: f64 = 0.55;

#[derive(Debug, Clone)]
pub struct EventPolicyOutcome {
    pub triggered: Vec<TriggeredPolicy>,
    pub effect: Option<AuthorizationEffect>,
    pub reason: Option<String>,
    pub safe_output: Option<String>,
    pub semantic_invocations: Vec<LlmCallAudit>,
}

impl EventPolicyOutcome {
    pub fn empty() -> Self {
        Self {
            triggered: Vec::new(),
            effect: None,
            reason: None,
            safe_output: None,
            semantic_invocations: Vec::new(),
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
pub struct SemanticPolicyJudgePolicyInput {
    pub policy_id: String,
    pub policy_description: String,
    pub match_clause: String,
    pub policy_action: String,
    pub policy_severity: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticPolicyJudgeBatchInput {
    pub tenant: String,
    pub event_summary: String,
    pub text: String,
    pub policies: Vec<SemanticPolicyJudgePolicyInput>,
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

#[derive(Debug, Clone)]
pub struct SemanticPolicyJudgeBatchOutcome {
    pub results: Vec<(String, SemanticPolicyJudgeResult)>,
    pub invocation: Option<LlmCallAudit>,
}

#[async_trait]
pub trait SemanticPolicyJudge: Send + Sync {
    fn is_enabled(&self) -> bool;

    async fn judge_policy(&self, input: SemanticPolicyJudgeInput) -> SemanticPolicyJudgeResult;

    async fn judge_policies(
        &self,
        input: SemanticPolicyJudgeBatchInput,
    ) -> Vec<(String, SemanticPolicyJudgeResult)> {
        let mut results = Vec::with_capacity(input.policies.len());
        for policy in input.policies {
            let policy_id = policy.policy_id.clone();
            let result = self
                .judge_policy(SemanticPolicyJudgeInput {
                    tenant: input.tenant.clone(),
                    policy_id: policy.policy_id,
                    policy_description: policy.policy_description,
                    match_clause: policy.match_clause,
                    policy_action: policy.policy_action,
                    policy_severity: policy.policy_severity,
                    event_summary: input.event_summary.clone(),
                    text: input.text.clone(),
                })
                .await;
            results.push((policy_id, result));
        }
        results
    }

    async fn judge_policies_with_audit(
        &self,
        input: SemanticPolicyJudgeBatchInput,
    ) -> SemanticPolicyJudgeBatchOutcome {
        SemanticPolicyJudgeBatchOutcome {
            results: self.judge_policies(input).await,
            invocation: None,
        }
    }
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

    async fn judge_policies(
        &self,
        input: SemanticPolicyJudgeBatchInput,
    ) -> Vec<(String, SemanticPolicyJudgeResult)> {
        self.judge_policies_with_audit(input).await.results
    }

    async fn judge_policies_with_audit(
        &self,
        input: SemanticPolicyJudgeBatchInput,
    ) -> SemanticPolicyJudgeBatchOutcome {
        if input.policies.is_empty() {
            return SemanticPolicyJudgeBatchOutcome {
                results: vec![],
                invocation: None,
            };
        }

        let policies_json = serde_json::Value::Array(
            input
                .policies
                .iter()
                .map(|policy| {
                    serde_json::json!({
                        "policy_id": policy.policy_id,
                        "policy_description": policy.policy_description,
                        "match_clause": policy.match_clause,
                        "policy_action": policy.policy_action,
                        "policy_severity": policy.policy_severity,
                    })
                })
                .collect(),
        );
        let prompt = semantic_policy::build_batch(
            &input.event_summary,
            &input.text,
            &serde_json::to_string_pretty(&policies_json).unwrap_or_else(|_| "[]".into()),
        );

        match self
            .judge_with_audit(
                JudgeKind::SemanticPolicy,
                &input.tenant,
                &prompt,
                &semantic_policy::batch_schema(),
            )
            .await
        {
            Ok(output) => SemanticPolicyJudgeBatchOutcome {
                results: semantic_batch_result_from_json(output.output.json, &input.policies),
                invocation: Some(output.audit),
            },
            Err(error) => SemanticPolicyJudgeBatchOutcome {
                results: input
                    .policies
                    .into_iter()
                    .map(|policy| {
                        (
                            policy.policy_id,
                            SemanticPolicyJudgeResult::Error(error.error.to_string()),
                        )
                    })
                    .collect(),
                invocation: Some(error.audit),
            },
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
    let Some(text) = policy_text(event) else {
        return EventPolicyOutcome::empty();
    };

    let mut outcome = EventPolicyOutcome::empty();
    let mut semantic_candidates = Vec::new();
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
                semantic_candidates.push(policy);
            }
        }
    }
    evaluate_semantic_policies(event, text, &semantic_candidates, &ctx, &mut outcome).await;

    outcome
}

async fn evaluate_semantic_policies(
    event: &GuardEvent,
    text: &str,
    policies: &[&Policy],
    ctx: &EventPolicyEvalCtx<'_>,
    outcome: &mut EventPolicyOutcome,
) {
    if policies.is_empty() {
        return;
    }

    let Some(judge) = ctx.semantic_judge else {
        tracing::warn!(
            policy_count = policies.len(),
            "semantic policies unavailable: no judge configured"
        );
        defer_high_severity_semantic_policies(policies, "no semantic judge configured", outcome);
        return;
    };
    if !judge.is_enabled() {
        tracing::warn!(
            policy_count = policies.len(),
            "semantic policies unavailable: no judge route configured"
        );
        defer_high_severity_semantic_policies(
            policies,
            "semantic judge route is disabled",
            outcome,
        );
        return;
    }

    let input = semantic_judge_batch_input(ctx.tenant, event, text, policies);
    let result = judge.judge_policies_with_audit(input).await;
    if let Some(invocation) = result.invocation {
        outcome.semantic_invocations.push(invocation);
    }
    let returned_policy_ids: HashSet<String> = result
        .results
        .iter()
        .map(|(policy_id, _)| policy_id.clone())
        .collect();
    for (policy_id, result) in result.results {
        let Some(policy) = policies
            .iter()
            .copied()
            .find(|policy| policy.id == policy_id)
        else {
            tracing::warn!(
                policy_id = %policy_id,
                "semantic policy judge returned unknown policy id"
            );
            continue;
        };
        apply_semantic_policy_result(policy, result, outcome);
    }
    let omitted_policies: Vec<&Policy> = policies
        .iter()
        .copied()
        .filter(|policy| !returned_policy_ids.contains(&policy.id))
        .collect();
    defer_high_severity_semantic_policies(
        &omitted_policies,
        "semantic judge omitted policy decision",
        outcome,
    );
}

fn defer_high_severity_semantic_policies(
    policies: &[&Policy],
    unavailable_reason: &str,
    outcome: &mut EventPolicyOutcome,
) {
    for policy in policies
        .iter()
        .copied()
        .filter(|policy| high_or_critical(policy.severity))
    {
        record_trigger_with_effect(
            outcome,
            policy,
            AuthorizationEffect::Defer,
            format!(
                "semantic policy judge unavailable for `{}`: {unavailable_reason}",
                policy.id
            ),
            None,
        );
    }
}

fn apply_semantic_policy_result(
    policy: &Policy,
    result: SemanticPolicyJudgeResult,
    outcome: &mut EventPolicyOutcome,
) {
    match result {
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
            record_trigger_with_effect(
                outcome,
                policy,
                AuthorizationEffect::Defer,
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
            record_trigger_with_effect(
                outcome,
                policy,
                AuthorizationEffect::Defer,
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
    if let Some(effect) = effect_from_action(policy.action) {
        record_trigger_with_effect(outcome, policy, effect, reason, policy.rewrite.clone());
    } else {
        outcome.triggered.push(TriggeredPolicy {
            id: policy.id.clone(),
            severity: policy.severity,
            reason,
        });
    }
}

fn record_trigger_with_effect(
    outcome: &mut EventPolicyOutcome,
    policy: &Policy,
    effect: AuthorizationEffect,
    reason: String,
    safe_output: Option<String>,
) {
    outcome.triggered.push(TriggeredPolicy {
        id: policy.id.clone(),
        severity: policy.severity,
        reason: reason.clone(),
    });

    if outcome
        .effect
        .map(|current| effect_rank(effect) > effect_rank(current))
        .unwrap_or(true)
    {
        outcome.effect = Some(effect);
        outcome.reason = Some(reason);
        outcome.safe_output = match effect {
            AuthorizationEffect::Transform => safe_output,
            _ => None,
        };
    }
}

fn effect_rank(effect: AuthorizationEffect) -> u8 {
    match effect {
        AuthorizationEffect::Permit => 0,
        AuthorizationEffect::Transform => 1,
        AuthorizationEffect::RequireApproval => 2,
        AuthorizationEffect::Defer => 3,
        AuthorizationEffect::Deny => 4,
    }
}

fn policy_text(event: &GuardEvent) -> Option<&str> {
    match event.kind {
        tl_core::EventKind::OutputProposed => event.action.parameters.get("text")?.as_str(),
        tl_core::EventKind::ToolCallProposed => event
            .action
            .parameters
            .get("__trustloop")?
            .get("policy_text")?
            .as_str(),
        _ => None,
    }
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

fn effect_from_action(action: AuthorizationEffect) -> Option<AuthorizationEffect> {
    match action {
        AuthorizationEffect::Permit => None,
        AuthorizationEffect::Deny => Some(AuthorizationEffect::Deny),
        AuthorizationEffect::Transform => Some(AuthorizationEffect::Transform),
        AuthorizationEffect::RequireApproval => Some(AuthorizationEffect::RequireApproval),
        AuthorizationEffect::Defer => Some(AuthorizationEffect::Defer),
    }
}

fn channel_name(channel: &tl_core::Channel) -> &'static str {
    match channel {
        tl_core::Channel::Voice => "voice",
        tl_core::Channel::Chat => "chat",
        tl_core::Channel::Email => "email",
    }
}

fn semantic_judge_policy_input(policy: &Policy) -> SemanticPolicyJudgePolicyInput {
    SemanticPolicyJudgePolicyInput {
        policy_id: policy.id.clone(),
        policy_description: policy.description.clone().unwrap_or_default(),
        match_clause: serde_json::to_string(&policy.r#match).unwrap_or_else(|_| "<invalid>".into()),
        policy_action: format!("{:?}", policy.action).to_ascii_lowercase(),
        policy_severity: format!("{:?}", policy.severity).to_ascii_lowercase(),
    }
}

fn semantic_judge_batch_input(
    tenant: &str,
    event: &GuardEvent,
    text: &str,
    policies: &[&Policy],
) -> SemanticPolicyJudgeBatchInput {
    SemanticPolicyJudgeBatchInput {
        tenant: tenant.to_string(),
        event_summary: event_summary(event),
        text: text.to_string(),
        policies: policies
            .iter()
            .map(|policy| semantic_judge_policy_input(policy))
            .collect(),
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

fn semantic_batch_result_from_json(
    json: serde_json::Value,
    policies: &[SemanticPolicyJudgePolicyInput],
) -> Vec<(String, SemanticPolicyJudgeResult)> {
    let Some(decisions) = json.get("decisions").and_then(serde_json::Value::as_array) else {
        return policies
            .iter()
            .map(|policy| {
                (
                    policy.policy_id.clone(),
                    SemanticPolicyJudgeResult::Error(
                        "semantic policy judge returned malformed `decisions`".into(),
                    ),
                )
            })
            .collect();
    };

    let mut results = Vec::with_capacity(policies.len());
    for decision in decisions {
        let Some(policy_id) = decision
            .get("policy_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        results.push((policy_id, semantic_result_from_json(decision.clone())));
    }

    for policy in policies {
        if !results
            .iter()
            .any(|(policy_id, _)| policy_id == &policy.policy_id)
        {
            results.push((
                policy.policy_id.clone(),
                SemanticPolicyJudgeResult::Error(
                    "semantic policy judge omitted policy decision".into(),
                ),
            ));
        }
    }

    results
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
                invocation_id: None,
                tool_identity: None,
                authorization: None,
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

    struct BatchRecordingJudge {
        results: Vec<(String, SemanticPolicyJudgeResult)>,
        calls: AtomicUsize,
        inputs: Mutex<Vec<SemanticPolicyJudgeBatchInput>>,
    }

    impl BatchRecordingJudge {
        fn new(results: Vec<(String, SemanticPolicyJudgeResult)>) -> Self {
            Self {
                results,
                calls: AtomicUsize::new(0),
                inputs: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl SemanticPolicyJudge for BatchRecordingJudge {
        fn is_enabled(&self) -> bool {
            true
        }

        async fn judge_policy(
            &self,
            _input: SemanticPolicyJudgeInput,
        ) -> SemanticPolicyJudgeResult {
            SemanticPolicyJudgeResult::Error("single-policy judge should not be called".into())
        }

        async fn judge_policies(
            &self,
            input: SemanticPolicyJudgeBatchInput,
        ) -> Vec<(String, SemanticPolicyJudgeResult)> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inputs.lock().unwrap().push(input);
            self.results.clone()
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
action: deny
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

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Deny));
        assert_eq!(outcome.triggered[0].id, "refund-guarantee");
    }

    #[tokio::test]
    async fn stronger_policy_verdict_wins_regardless_of_order() {
        let rewrite = load_str(
            r#"
id: rewrite-risky
match:
  literal: risky claim
action: transform
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
action: deny
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

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Deny));
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
action: deny
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
        assert_eq!(outcome.effect, None);
    }

    #[tokio::test]
    async fn semantic_policy_match_blocks_output_event() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: deny
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

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Deny));
        assert_eq!(outcome.triggered[0].id, "respectful-tone");
        assert!(outcome.triggered[0].reason.contains("confidence=0.94"));
        assert_eq!(judge.calls(), 1);
    }

    #[tokio::test]
    async fn semantic_policies_are_batched_into_one_judge_call() {
        let block_policy = load_str(
            r#"
id: no-insults
match:
  semantic: "the agent insults or demeans the user"
action: deny
severity: high
"#,
        )
        .unwrap();
        let escalate_policy = load_str(
            r#"
id: no-legal-advice
match:
  semantic: "the agent gives legal advice"
action: require_approval
severity: high
"#,
        )
        .unwrap();
        let judge = BatchRecordingJudge::new(vec![
            (
                "no-insults".into(),
                SemanticPolicyJudgeResult::Matched {
                    confidence: 0.95,
                    reason: "direct insult".into(),
                    evidence: vec!["you are dumb".into()],
                },
            ),
            (
                "no-legal-advice".into(),
                SemanticPolicyJudgeResult::NotMatched {
                    confidence: 0.10,
                    reason: "no legal advice".into(),
                    evidence: vec![],
                },
            ),
        ]);

        let outcome = evaluate_event_policies(
            &output_event("you are dumb"),
            &[block_policy, escalate_policy],
            eval_ctx(Some(&judge)),
        )
        .await;

        assert_eq!(judge.calls(), 1);
        let inputs = judge.inputs.lock().unwrap();
        assert_eq!(inputs[0].policies.len(), 2);
        assert_eq!(inputs[0].policies[0].policy_id, "no-insults");
        assert_eq!(inputs[0].policies[1].policy_id, "no-legal-advice");
        assert_eq!(outcome.effect, Some(AuthorizationEffect::Deny));
        assert_eq!(outcome.triggered.len(), 1);
        assert_eq!(outcome.triggered[0].id, "no-insults");
    }

    #[tokio::test]
    async fn omitted_high_severity_batch_result_defers() {
        let returned_policy = load_str(
            r#"
id: no-insults
match:
  semantic: "the agent insults or demeans the user"
action: deny
severity: high
"#,
        )
        .unwrap();
        let omitted_policy = load_str(
            r#"
id: no-legal-advice
match:
  semantic: "the agent gives legal advice"
action: deny
severity: critical
"#,
        )
        .unwrap();
        let judge = BatchRecordingJudge::new(vec![(
            "no-insults".into(),
            SemanticPolicyJudgeResult::NotMatched {
                confidence: 0.99,
                reason: "no insult".into(),
                evidence: vec![],
            },
        )]);

        let outcome = evaluate_event_policies(
            &output_event("you should sue them"),
            &[returned_policy, omitted_policy],
            eval_ctx(Some(&judge)),
        )
        .await;

        assert_eq!(judge.calls(), 1);
        assert_eq!(outcome.effect, Some(AuthorizationEffect::Defer));
        assert_eq!(outcome.triggered.len(), 1);
        assert_eq!(outcome.triggered[0].id, "no-legal-advice");
        assert!(outcome.reason.unwrap().contains("omitted policy decision"));
    }

    #[tokio::test]
    async fn omitted_lower_severity_batch_result_remains_advisory() {
        let returned_policy = load_str(
            r#"
id: no-insults
match:
  semantic: "the agent insults or demeans the user"
action: deny
severity: high
"#,
        )
        .unwrap();
        let omitted_policy = load_str(
            r#"
id: no-rudeness
match:
  semantic: "the agent is rude"
action: deny
severity: medium
"#,
        )
        .unwrap();
        let judge = BatchRecordingJudge::new(vec![(
            "no-insults".into(),
            SemanticPolicyJudgeResult::NotMatched {
                confidence: 0.99,
                reason: "no insult".into(),
                evidence: vec![],
            },
        )]);

        let outcome = evaluate_event_policies(
            &output_event("neutral response"),
            &[returned_policy, omitted_policy],
            eval_ctx(Some(&judge)),
        )
        .await;

        assert_eq!(judge.calls(), 1);
        assert_eq!(outcome.effect, None);
        assert!(outcome.triggered.is_empty());
    }

    #[tokio::test]
    async fn high_severity_semantic_policy_without_judge_defers() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: deny
severity: high
"#,
        )
        .unwrap();

        let outcome =
            evaluate_event_policies(&output_event("you are dumb"), &[policy], eval_ctx(None)).await;

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Defer));
        assert_eq!(outcome.triggered[0].id, "respectful-tone");
        assert!(outcome
            .reason
            .unwrap()
            .contains("no semantic judge configured"));
    }

    #[tokio::test]
    async fn high_severity_semantic_policy_with_disabled_judge_defers() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: deny
severity: critical
"#,
        )
        .unwrap();
        let judge = RecordingJudge::default();

        let outcome = evaluate_event_policies(
            &output_event("you are dumb"),
            &[policy],
            eval_ctx(Some(&judge)),
        )
        .await;

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Defer));
        assert_eq!(outcome.triggered[0].id, "respectful-tone");
        assert!(outcome.reason.unwrap().contains("route is disabled"));
        assert_eq!(judge.calls(), 0);
    }

    #[tokio::test]
    async fn low_severity_semantic_policy_without_judge_remains_advisory() {
        let policy = load_str(
            r#"
id: friendly-tone
match:
  semantic: "the agent sounds curt"
action: transform
rewrite: "Please try again."
severity: low
"#,
        )
        .unwrap();

        let outcome = evaluate_event_policies(&output_event("no"), &[policy], eval_ctx(None)).await;

        assert!(outcome.triggered.is_empty());
        assert_eq!(outcome.effect, None);
    }

    #[tokio::test]
    async fn semantic_low_confidence_match_does_not_trigger() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: deny
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
        assert_eq!(outcome.effect, None);
        assert_eq!(judge.calls(), 1);
    }

    #[tokio::test]
    async fn semantic_ambiguous_high_severity_escalates() {
        let policy = load_str(
            r#"
id: legal-advice
match:
  semantic: "the agent gives legal advice"
action: deny
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

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Defer));
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
action: deny
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

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Defer));
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
action: deny
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

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Deny));
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
action: deny
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
        assert_eq!(outcome.effect, None);
        assert_eq!(judge.calls(), 0);
    }

    #[tokio::test]
    async fn non_output_event_does_not_evaluate_content_policy() {
        let policy = load_str(
            r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: deny
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
        assert_eq!(outcome.effect, None);
        assert_eq!(judge.calls(), 0);
    }

    #[tokio::test]
    async fn managed_tool_policy_text_uses_existing_agent_scoped_content_policies() {
        let policy = load_str(
            r#"
id: no-customer-training
when:
  agents: [agent-1]
match:
  literal: train a model
action: deny
severity: critical
"#,
        )
        .unwrap();
        let mut event = tool_event();
        event.action.parameters = serde_json::json!({
            "query": "customers",
            "__trustloop": {
                "policy_text": "Use the customer database to train a model",
                "purpose": "model_training"
            }
        });

        let outcome = evaluate_event_policies(&event, &[policy], eval_ctx(None)).await;

        assert_eq!(outcome.effect, Some(AuthorizationEffect::Deny));
        assert_eq!(outcome.triggered[0].id, "no-customer-training");
    }
}
