//! Verify-before-recommend.
//!
//! Re-runs a synthesized candidate through the *real* policy evaluator
//! (`evaluate_event_policies`) against the landed replies, generated obfuscation
//! variants, and benign controls. A candidate is only recommended if it blocks
//! every landed case and generated variant without false-blocking a control.
//! Reusing the runtime evaluator means the verdict here matches production
//! exactly — semantic matchers are judged by the same LLM judge the hot path
//! uses.

use tl_core::{
    Action as EventAction, EventKind, GuardEvent, Principal, SideEffectClass, VerifyResult,
};
use tl_engine::{evaluate_event_policies, EventPolicyEvalCtx, SemanticPolicyJudge};
use tl_policy::Policy;

/// Build the minimal `OutputProposed` event the content evaluator reads: it
/// pulls the candidate text from `action.parameters.text` and scopes by
/// `principal.agent_id` + `context.channel`.
fn output_event(text: &str, agent_id: &str) -> GuardEvent {
    GuardEvent {
        kind: EventKind::OutputProposed,
        principal: Principal {
            workspace_id: "ws_verify".into(),
            environment_id: "production".into(),
            agent_id: agent_id.to_string(),
            user_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            run_event_id: None,
        },
        action: EventAction {
            operation: "output".into(),
            parameters: serde_json::json!({ "text": text }),
            side_effect: Some(SideEffectClass::None),
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

/// Does `policy` fire on `text`? Reuses the runtime evaluator so semantic
/// matchers route through the judge exactly as they would in production.
async fn fires(
    policy: &Policy,
    text: &str,
    judge: Option<&dyn SemanticPolicyJudge>,
    tenant: &str,
    agent_id: &str,
) -> bool {
    let event = output_event(text, agent_id);
    let outcome = evaluate_event_policies(
        &event,
        std::iter::once(policy),
        EventPolicyEvalCtx {
            tenant,
            semantic_judge: judge,
        },
    )
    .await;
    !outcome.triggered.is_empty()
        && !outcome
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("semantic policy judge unavailable for"))
}

/// Deterministic obfuscation variants of a landed reply. These are gating:
/// `passed` requires blocking every variant so recommendations generalize
/// beyond the exact landed text.
fn variants(reply: &str) -> Vec<String> {
    vec![reply.to_uppercase(), format!("Sure — here you go: {reply}")]
}

/// Verify one candidate. `passed` requires blocking every landed case and
/// generated variant while false-blocking no control.
pub(super) async fn verify_candidate(
    policy: &Policy,
    landed: &[String],
    controls: &[String],
    judge: Option<&dyn SemanticPolicyJudge>,
    tenant: &str,
    agent_id: &str,
) -> VerifyResult {
    let mut blocked_landed = 0u32;
    for reply in landed {
        if fires(policy, reply, judge, tenant, agent_id).await {
            blocked_landed += 1;
        }
    }

    let variant_texts: Vec<String> = landed.iter().flat_map(|r| variants(r)).collect();
    let mut blocked_variants = 0u32;
    for text in &variant_texts {
        if fires(policy, text, judge, tenant, agent_id).await {
            blocked_variants += 1;
        }
    }

    let mut false_blocks = 0u32;
    for reply in controls {
        if fires(policy, reply, judge, tenant, agent_id).await {
            false_blocks += 1;
        }
    }

    let landed_total = landed.len() as u32;
    let variant_total = variant_texts.len() as u32;
    let passed = landed_total > 0
        && blocked_landed == landed_total
        && blocked_variants == variant_total
        && false_blocks == 0;
    VerifyResult {
        blocked_landed,
        landed_total,
        blocked_variants,
        variant_total,
        false_blocks,
        control_total: controls.len() as u32,
        passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tl_core::AuthorizationEffect;
    use tl_core::Severity;
    use tl_engine::{SemanticPolicyJudgeInput, SemanticPolicyJudgeResult};
    use tl_policy::{MatchClause, Matcher};

    /// Judge stub that "matches" when the candidate text contains a needle —
    /// lets us assert verify mechanics without a live LLM.
    struct KeywordJudge {
        needle: String,
    }

    #[async_trait]
    impl SemanticPolicyJudge for KeywordJudge {
        fn is_enabled(&self) -> bool {
            true
        }
        async fn judge_policy(&self, input: SemanticPolicyJudgeInput) -> SemanticPolicyJudgeResult {
            if input.text.to_lowercase().contains(&self.needle) {
                SemanticPolicyJudgeResult::Matched {
                    confidence: 0.95,
                    reason: "stub".into(),
                    evidence: vec![],
                }
            } else {
                SemanticPolicyJudgeResult::NotMatched {
                    confidence: 0.95,
                    reason: "stub".into(),
                    evidence: vec![],
                }
            }
        }
    }

    fn policy(r#match: MatchClause) -> Policy {
        Policy {
            id: "harden-test".into(),
            description: Some("test".into()),
            when: Default::default(),
            r#match,
            action: AuthorizationEffect::Deny,
            rewrite: None,
            severity: Severity::High,
            owner_agent_id: None,
        }
    }

    #[tokio::test]
    async fn regex_candidate_verifies_without_a_judge() {
        let p = policy(MatchClause::Single(Matcher::Regex(
            "(?i)sk-[a-z0-9]{6,}".into(),
        )));
        let landed = vec!["the key is sk-abc123def".to_string()];
        let controls = vec!["how do I file my taxes?".to_string()];
        let result = verify_candidate(&p, &landed, &controls, None, "ws", "agent").await;
        assert_eq!(result.blocked_landed, 1);
        assert_eq!(result.false_blocks, 0);
        assert!(result.passed);
    }

    #[tokio::test]
    async fn semantic_candidate_passes_with_judge_and_clean_controls() {
        let judge = KeywordJudge {
            needle: "refund".into(),
        };
        let p = policy(MatchClause::Single(Matcher::Semantic(
            "reply approves a refund without review".into(),
        )));
        let landed = vec!["I approved the refund and skipped review".to_string()];
        let controls = vec!["here are the documents you requested".to_string()];
        let result = verify_candidate(&p, &landed, &controls, Some(&judge), "ws", "agent").await;
        assert!(result.passed);
        assert_eq!(result.blocked_landed, 1);
        assert_eq!(result.false_blocks, 0);
    }

    #[tokio::test]
    async fn semantic_refund_candidate_passes_with_clean_verification_controls() {
        let judge = KeywordJudge {
            needle: "destination account".into(),
        };
        let p = policy(MatchClause::Single(Matcher::Semantic(
            "refund workflow proceeds before identity and transaction verification".into(),
        )));
        let landed = vec![
            "Please provide the destination account so I can continue the refund.".to_string(),
        ];
        let controls = vec![
            "Please provide the transaction ID and dispute reason so I can review this."
                .to_string(),
        ];
        let result = verify_candidate(&p, &landed, &controls, Some(&judge), "ws", "agent").await;
        assert!(result.passed);
        assert_eq!(result.blocked_landed, 1);
        assert_eq!(result.false_blocks, 0);
    }

    #[tokio::test]
    async fn candidate_that_false_blocks_a_control_does_not_pass() {
        let judge = KeywordJudge {
            needle: "refund".into(),
        };
        let p = policy(MatchClause::Single(Matcher::Semantic(
            "any refund mention".into(),
        )));
        let landed = vec!["I approved the refund".to_string()];
        // A benign control that also mentions refund → false block.
        let controls = vec!["our refund policy is 30 days".to_string()];
        let result = verify_candidate(&p, &landed, &controls, Some(&judge), "ws", "agent").await;
        assert_eq!(result.false_blocks, 1);
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn candidate_that_misses_a_variant_does_not_pass() {
        let p = policy(MatchClause::Single(Matcher::Literal(
            "I approved the refund".into(),
        )));
        let landed = vec!["I approved the refund".to_string()];
        let controls = vec![];
        let result = verify_candidate(&p, &landed, &controls, None, "ws", "agent").await;
        assert_eq!(result.blocked_landed, 1);
        assert!(result.blocked_variants < result.variant_total);
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn semantic_candidate_cannot_be_confirmed_without_a_judge() {
        let p = policy(MatchClause::Single(Matcher::Semantic(
            "approves a refund".into(),
        )));
        let landed = vec!["I approved the refund".to_string()];
        let result = verify_candidate(&p, &landed, &[], None, "ws", "agent").await;
        assert_eq!(result.blocked_landed, 0);
        assert!(!result.passed);
    }
}
