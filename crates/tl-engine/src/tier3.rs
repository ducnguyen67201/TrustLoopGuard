//! Tier 3 — LLM judges. Real implementation as of PR 9.
//!
//! Three independent judges fan out via `tokio::join!`:
//! - **Hallucination** — every claim in the draft must be grounded in
//!   the agent's knowledge sources or in per-request docs.
//! - **Tone** — draft tone must match the agent's configured target and
//!   avoid the forbidden list.
//! - **Authority** — every commitment in the draft must be on the
//!   agent's `can_promise` list.
//!
//! Each judge is gated on the `LlmRouter` having a route configured for
//! its `JudgeKind`. If no route is configured for any of the three, the
//! tier returns `Skipped`. If the agent profile can't be resolved, the
//! tier also returns `Skipped` — judges need profile context to be
//! grounded.
//!
//! Verdict aggregation:
//! - Hallucination not grounded → Block
//! - Authority not within → Block
//! - Tone mismatch → Revise (or Escalate if no profile-level rewrite)
//! - First non-Allow signal wins.

use std::time::Instant;

use tl_core::{
    AgentProfile, CheckRequest, Severity, Tier, TierResult, TierStatus, TriggeredPolicy, Verdict,
    DEFAULT_WORKSPACE_ID,
};
use tl_llm::prompts::{authority, hallucination, tone};
use tl_llm::{JudgeKind, LlmError, LlmOutput, LlmRouter};
use tokio_util::sync::CancellationToken;

use crate::handler::HandlerCtx;
use crate::orchestrate::{BlockSignal, TierOutput};

pub async fn run(req: &CheckRequest, ctx: &HandlerCtx, cancel: CancellationToken) -> TierOutput {
    let start = Instant::now();
    if cancel.is_cancelled() {
        return cancelled();
    }

    // Resolve agent profile. Without it we have no grounding context to
    // give the judges, so we skip rather than running with empty inputs.
    let workspace_id = req.workspace_id.as_deref().unwrap_or(DEFAULT_WORKSPACE_ID);
    let profile = match ctx
        .profile_resolver
        .resolve(workspace_id, &req.agent_id)
        .await
    {
        Some(p) => p,
        None => return skipped(start),
    };

    // Decide which judges have routes configured. If none → skip.
    let do_hallu = ctx.llm.has_route(JudgeKind::Hallucination);
    let do_tone = ctx.llm.has_route(JudgeKind::Tone);
    let do_auth = ctx.llm.has_route(JudgeKind::Authority);
    if !do_hallu && !do_tone && !do_auth {
        return skipped(start);
    }

    // Gather per-request docs from `req.context.docs`. Missing or wrong
    // shape is fine — judges receive an empty doc set.
    let docs = extract_docs(&req.context);

    // Build prompts up front so the join body is just IO.
    let hallu_prompt = hallucination::build(
        &summarise_profile(&profile),
        &docs.join("\n---\n"),
        &req.input,
        &req.proposed_output,
    );
    let tone_prompt = tone::build(
        &profile.tone.target,
        &profile.tone.forbidden.join(", "),
        &req.input,
        &req.proposed_output,
    );
    let auth_prompt = authority::build(
        &bulleted(&profile.authority.can_promise),
        &bulleted(&profile.authority.cannot_promise),
        &req.input,
        &req.proposed_output,
    );

    // Tenant id lives outside this trait surface in v0; default to the
    // agent_id which is unique per agent and sufficient for budget
    // bucketing until proper multi-tenancy lands.
    let tenant = workspace_id;

    // Cancellation is honored at this select level: if the orchestrator
    // fires the cancel token, abort immediately rather than waiting for
    // the LLM round-trips.
    let result = tokio::select! {
        _ = cancel.cancelled() => return cancelled(),
        out = run_judges(
            &ctx.llm,
            tenant,
            do_hallu, &hallu_prompt,
            do_tone, &tone_prompt,
            do_auth, &auth_prompt,
        ) => out,
    };

    let JudgeOutcomes { hallu, tone, auth } = result;
    aggregate(start, &profile, hallu, tone, auth)
}

struct JudgeOutcomes {
    hallu: JudgeResult,
    tone: JudgeResult,
    auth: JudgeResult,
}

enum JudgeResult {
    Skipped,
    Ok(LlmOutput),
    Err(LlmError),
}

#[allow(clippy::too_many_arguments)]
async fn run_judges(
    router: &LlmRouter,
    tenant: &str,
    do_hallu: bool,
    hallu_prompt: &str,
    do_tone: bool,
    tone_prompt: &str,
    do_auth: bool,
    auth_prompt: &str,
) -> JudgeOutcomes {
    let h = async {
        if !do_hallu {
            return JudgeResult::Skipped;
        }
        match router
            .judge(
                JudgeKind::Hallucination,
                tenant,
                hallu_prompt,
                &hallucination::schema(),
            )
            .await
        {
            Ok(o) => JudgeResult::Ok(o),
            Err(e) => JudgeResult::Err(e),
        }
    };
    let t = async {
        if !do_tone {
            return JudgeResult::Skipped;
        }
        match router
            .judge(JudgeKind::Tone, tenant, tone_prompt, &tone::schema())
            .await
        {
            Ok(o) => JudgeResult::Ok(o),
            Err(e) => JudgeResult::Err(e),
        }
    };
    let a = async {
        if !do_auth {
            return JudgeResult::Skipped;
        }
        match router
            .judge(
                JudgeKind::Authority,
                tenant,
                auth_prompt,
                &authority::schema(),
            )
            .await
        {
            Ok(o) => JudgeResult::Ok(o),
            Err(e) => JudgeResult::Err(e),
        }
    };
    let (hallu, tone, auth) = tokio::join!(h, t, a);
    JudgeOutcomes { hallu, tone, auth }
}

fn aggregate(
    start: Instant,
    profile: &AgentProfile,
    hallu: JudgeResult,
    tone: JudgeResult,
    auth: JudgeResult,
) -> TierOutput {
    let mut reasons: Vec<TriggeredPolicy> = vec![];
    let mut block: Option<BlockSignal> = None;

    // -- Hallucination --
    if let Some(verdict) = interpret_hallucination(&hallu) {
        match verdict {
            JudgeVerdict::Allow => {}
            JudgeVerdict::BlockGrounded(violations) => {
                let reason = format!("hallucination: {}", violations.join("; "));
                reasons.push(TriggeredPolicy {
                    id: "tl:hallucination".into(),
                    severity: Severity::High,
                    reason: reason.clone(),
                });
                block.get_or_insert(BlockSignal {
                    verdict: Verdict::Block,
                    reason,
                    safe_output: None,
                });
            }
            JudgeVerdict::Escalate(reason) => {
                reasons.push(TriggeredPolicy {
                    id: "tl:hallucination_unavailable".into(),
                    severity: Severity::Medium,
                    reason: reason.clone(),
                });
                block.get_or_insert(BlockSignal {
                    verdict: Verdict::Escalate,
                    reason,
                    safe_output: None,
                });
            }
            _ => {}
        }
    }

    // -- Authority --
    if let Some(verdict) = interpret_authority(&auth) {
        match verdict {
            JudgeVerdict::Allow => {}
            JudgeVerdict::BlockGrounded(violations) => {
                let reason = format!("authority violation: {}", violations.join("; "));
                reasons.push(TriggeredPolicy {
                    id: "tl:authority".into(),
                    severity: Severity::High,
                    reason: reason.clone(),
                });
                block.get_or_insert(BlockSignal {
                    verdict: Verdict::Block,
                    reason,
                    safe_output: None,
                });
            }
            JudgeVerdict::Escalate(reason) => {
                reasons.push(TriggeredPolicy {
                    id: "tl:authority_unavailable".into(),
                    severity: Severity::Medium,
                    reason: reason.clone(),
                });
                block.get_or_insert(BlockSignal {
                    verdict: Verdict::Escalate,
                    reason,
                    safe_output: None,
                });
            }
            _ => {}
        }
    }

    // -- Tone (Revise rather than Block) --
    if let Some(verdict) = interpret_tone(&tone, profile) {
        match verdict {
            JudgeVerdict::Allow => {}
            JudgeVerdict::Revise(reason, fallback) => {
                reasons.push(TriggeredPolicy {
                    id: "tl:tone".into(),
                    severity: Severity::Low,
                    reason: reason.clone(),
                });
                block.get_or_insert(BlockSignal {
                    verdict: Verdict::Rewrite,
                    reason,
                    safe_output: fallback,
                });
            }
            JudgeVerdict::Escalate(reason) => {
                reasons.push(TriggeredPolicy {
                    id: "tl:tone_unavailable".into(),
                    severity: Severity::Low,
                    reason: reason.clone(),
                });
                block.get_or_insert(BlockSignal {
                    verdict: Verdict::Escalate,
                    reason,
                    safe_output: None,
                });
            }
            _ => {}
        }
    }

    TierOutput {
        result: TierResult {
            tier: Tier::Llm,
            status: TierStatus::Completed,
            reasons,
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        block,
    }
}

enum JudgeVerdict {
    Allow,
    BlockGrounded(Vec<String>),
    Revise(String, Option<String>),
    Escalate(String),
}

fn interpret_hallucination(j: &JudgeResult) -> Option<JudgeVerdict> {
    match j {
        JudgeResult::Skipped => None,
        JudgeResult::Err(e) => Some(JudgeVerdict::Escalate(format!("hallucination judge: {e}"))),
        JudgeResult::Ok(out) => {
            let grounded = out.json["grounded"].as_bool().unwrap_or(true);
            if grounded {
                Some(JudgeVerdict::Allow)
            } else {
                let violations: Vec<String> = out.json["violations"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(JudgeVerdict::BlockGrounded(violations))
            }
        }
    }
}

fn interpret_authority(j: &JudgeResult) -> Option<JudgeVerdict> {
    match j {
        JudgeResult::Skipped => None,
        JudgeResult::Err(e) => Some(JudgeVerdict::Escalate(format!("authority judge: {e}"))),
        JudgeResult::Ok(out) => {
            let within = out.json["within_authority"].as_bool().unwrap_or(true);
            if within {
                Some(JudgeVerdict::Allow)
            } else {
                let violations: Vec<String> = out.json["forbidden_promises"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(JudgeVerdict::BlockGrounded(violations))
            }
        }
    }
}

fn interpret_tone(j: &JudgeResult, _profile: &AgentProfile) -> Option<JudgeVerdict> {
    match j {
        JudgeResult::Skipped => None,
        JudgeResult::Err(e) => Some(JudgeVerdict::Escalate(format!("tone judge: {e}"))),
        JudgeResult::Ok(out) => {
            let matches = out.json["matches_target"].as_bool().unwrap_or(true);
            if matches {
                Some(JudgeVerdict::Allow)
            } else {
                let issues: Vec<String> = out.json["issues"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let detected = out.json["detected_tone"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let reason = if issues.is_empty() {
                    format!("tone mismatch (detected: {detected})")
                } else {
                    format!(
                        "tone mismatch (detected: {detected}): {}",
                        issues.join("; ")
                    )
                };
                // No automatic rewrite in v0 — Revise without a safe_output
                // tells the caller to escalate or canned-respond.
                Some(JudgeVerdict::Revise(reason, None))
            }
        }
    }
}

fn extract_docs(context: &serde_json::Value) -> Vec<String> {
    context
        .get("docs")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn summarise_profile(p: &AgentProfile) -> String {
    let mut s = String::new();
    s.push_str(&format!("Display name: {}\n", p.display_name));
    if !p.scope.in_scope.is_empty() {
        s.push_str("In scope:\n");
        for it in &p.scope.in_scope {
            s.push_str(&format!("- {it}\n"));
        }
    }
    if !p.knowledge_sources.is_empty() {
        s.push_str("Knowledge sources:\n");
        for k in &p.knowledge_sources {
            let kind = format!("{:?}", k.kind).to_lowercase();
            let mut line = format!("- {} ({kind})", k.kb_id);
            if let Some(description) = &k.description {
                line.push_str(&format!(": {description}"));
            }
            if let Some(url) = &k.url {
                line.push_str(&format!(" [{url}]"));
            }
            s.push_str(&line);
            s.push('\n');
        }
    }
    s
}

fn bulleted(items: &[String]) -> String {
    if items.is_empty() {
        "(none)".into()
    } else {
        items
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn skipped(start: Instant) -> TierOutput {
    TierOutput {
        result: TierResult {
            tier: Tier::Llm,
            status: TierStatus::Skipped,
            reasons: vec![],
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        block: None,
    }
}

fn cancelled() -> TierOutput {
    TierOutput {
        result: TierResult {
            tier: Tier::Llm,
            status: TierStatus::Cancelled,
            reasons: vec![],
            elapsed_ms: 0,
        },
        block: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tl_core::{
        AgentAuthority, AgentScope, AgentTone, Channel, KnowledgeSource, KnowledgeSourceKind,
    };
    use tl_llm::{JsonSchema, LlmClient, LlmOutput, ProviderTarget, ResolvedRoute, TokenBudget};

    // ---- Test fixtures ----

    /// `LlmClient` mock that always returns a canned `LlmOutput`. The
    /// shape of the output is `Hallucination`/`Tone`/`Authority`-shaped
    /// so the same mock can satisfy all three judges; the right keys
    /// are picked up by the right interpreter.
    struct CannedClient {
        out: serde_json::Value,
    }
    #[async_trait]
    impl LlmClient for CannedClient {
        async fn complete(
            &self,
            _model: &str,
            _prompt: &str,
            _schema: &JsonSchema,
            _deadline: std::time::Duration,
        ) -> Result<LlmOutput, tl_llm::LlmError> {
            Ok(LlmOutput {
                json: self.out.clone(),
                prompt_tokens: 5,
                completion_tokens: 5,
            })
        }
    }

    fn router_returning(json: serde_json::Value) -> LlmRouter {
        let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        providers.insert("p".into(), Arc::new(CannedClient { out: json }));
        let target = ProviderTarget {
            provider: "p".into(),
            model: "m".into(),
            deadline_ms: 1_000,
        };
        let mut routes = HashMap::new();
        for k in [
            JudgeKind::Hallucination,
            JudgeKind::Tone,
            JudgeKind::Authority,
        ] {
            routes.insert(
                k,
                ResolvedRoute {
                    primary: target.clone(),
                    fallback: None,
                },
            );
        }
        LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)))
    }

    /// ProfileResolver that always returns the given profile.
    struct FixedResolver(Arc<AgentProfile>);
    #[async_trait]
    impl crate::handler::ProfileResolver for FixedResolver {
        async fn resolve(&self, _workspace_id: &str, _agent_id: &str) -> Option<Arc<AgentProfile>> {
            Some(self.0.clone())
        }
    }

    fn sample_profile() -> Arc<AgentProfile> {
        Arc::new(AgentProfile {
            agent_id: "a".into(),
            display_name: "Test Agent".into(),
            scope: AgentScope {
                in_scope: vec!["billing".into()],
                out_of_scope: vec![],
            },
            authority: AgentAuthority {
                can_promise: vec!["respond within 24h".into()],
                cannot_promise: vec!["refunds".into()],
            },
            tone: AgentTone {
                target: "warm-professional".into(),
                forbidden: vec!["sarcastic".into()],
            },
            knowledge_sources: vec![],
            escalation_triggers: vec![],
            system_prompt: None,
        })
    }

    fn sample_req() -> CheckRequest {
        CheckRequest {
            workspace_id: None,
            run_id: None,
            run_event_id: None,
            run_event: None,
            agent_id: "a".into(),
            channel: Channel::Chat,
            input: "hello".into(),
            proposed_output: "hi there".into(),
            domain: None,
            policies: vec![],
            context: serde_json::Value::Null,
            trace_id: None,
            redaction: None,
        }
    }

    fn ctx_with(router: LlmRouter) -> HandlerCtx {
        let mut c = HandlerCtx::no_op();
        c.profile_resolver = Arc::new(FixedResolver(sample_profile()));
        c.llm = Arc::new(router);
        c
    }

    // ---- Tests ----

    #[tokio::test]
    async fn no_profile_yields_skipped() {
        // Default no_op resolver returns None → tier reports Skipped.
        let ctx = HandlerCtx::no_op();
        let out = run(&sample_req(), &ctx, CancellationToken::new()).await;
        assert_eq!(out.result.status, TierStatus::Skipped);
        assert!(out.block.is_none());
    }

    #[tokio::test]
    async fn empty_router_yields_skipped() {
        // Profile resolves but router has no routes → Skipped.
        let mut ctx = HandlerCtx::no_op();
        ctx.profile_resolver = Arc::new(FixedResolver(sample_profile()));
        let out = run(&sample_req(), &ctx, CancellationToken::new()).await;
        assert_eq!(out.result.status, TierStatus::Skipped);
    }

    #[tokio::test]
    async fn three_clean_verdicts_yield_completed_with_no_block() {
        // All three judges report happy paths.
        let json = json!({
            // Shared keys across the three schemas — interpreters pick
            // the right ones for each judge.
            "grounded": true,
            "violations": [],
            "matches_target": true,
            "detected_tone": "warm-professional",
            "issues": [],
            "within_authority": true,
            "forbidden_promises": []
        });
        let ctx = ctx_with(router_returning(json));
        let out = run(&sample_req(), &ctx, CancellationToken::new()).await;
        assert_eq!(out.result.status, TierStatus::Completed);
        assert!(out.block.is_none(), "no judge fired, block should be None");
        assert!(out.result.reasons.is_empty());
    }

    #[test]
    fn profile_summary_includes_web_knowledge_source_metadata() {
        let mut profile = (*sample_profile()).clone();
        profile.knowledge_sources = vec![KnowledgeSource {
            kb_id: "acme-docs".into(),
            kind: KnowledgeSourceKind::Web,
            url: Some("https://docs.acme.test/help".into()),
            description: Some("Public support docs".into()),
        }];

        let summary = summarise_profile(&profile);

        assert!(summary.contains("acme-docs"));
        assert!(summary.contains("web"));
        assert!(summary.contains("Public support docs"));
        assert!(summary.contains("https://docs.acme.test/help"));
    }

    #[tokio::test]
    async fn hallucination_violation_blocks() {
        let json = json!({
            "grounded": false,
            "violations": ["agent claimed 24/7 phone support but docs say email only"],
            "matches_target": true,
            "detected_tone": "ok",
            "issues": [],
            "within_authority": true,
            "forbidden_promises": []
        });
        let ctx = ctx_with(router_returning(json));
        let out = run(&sample_req(), &ctx, CancellationToken::new()).await;
        let b = out.block.expect("block set");
        assert_eq!(b.verdict, Verdict::Block);
        assert!(b.reason.contains("hallucination"));
        assert!(out
            .result
            .reasons
            .iter()
            .any(|r| r.id == "tl:hallucination"));
    }

    #[tokio::test]
    async fn authority_violation_blocks() {
        let json = json!({
            "grounded": true,
            "violations": [],
            "matches_target": true,
            "detected_tone": "ok",
            "issues": [],
            "within_authority": false,
            "forbidden_promises": ["promised a full refund"]
        });
        let ctx = ctx_with(router_returning(json));
        let out = run(&sample_req(), &ctx, CancellationToken::new()).await;
        let b = out.block.expect("block set");
        assert_eq!(b.verdict, Verdict::Block);
        assert!(b.reason.contains("authority"));
        assert!(out.result.reasons.iter().any(|r| r.id == "tl:authority"));
    }

    #[tokio::test]
    async fn tone_mismatch_yields_rewrite_verdict() {
        let json = json!({
            "grounded": true,
            "violations": [],
            "matches_target": false,
            "detected_tone": "curt",
            "issues": ["too clipped, no acknowledgement"],
            "within_authority": true,
            "forbidden_promises": []
        });
        let ctx = ctx_with(router_returning(json));
        let out = run(&sample_req(), &ctx, CancellationToken::new()).await;
        let b = out.block.expect("block set");
        assert_eq!(b.verdict, Verdict::Rewrite);
        assert!(b.reason.contains("tone"));
    }

    #[tokio::test]
    async fn pre_cancelled_token_short_circuits() {
        let ctx = ctx_with(router_returning(json!({})));
        let cancel = CancellationToken::new();
        cancel.cancel();
        let out = run(&sample_req(), &ctx, cancel).await;
        assert_eq!(out.result.status, TierStatus::Cancelled);
    }
}
