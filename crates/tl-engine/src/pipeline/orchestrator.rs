//! Parallel-with-cancellation orchestrator across the three tiers.
//!
//! See `docs/concept/v0-design-decisions.md §4`. The pattern:
//!
//! - All three tiers start in parallel at t=0.
//! - Tier 1 is awaited first (microsecond-scale). If it produced a hard
//!   block, the cancellation token fires and tiers 2 and 3 short-circuit.
//! - Otherwise, Tier 2 is awaited. If it blocks, Tier 3 is cancelled.
//! - Otherwise, Tier 3 is awaited subject to a deadline. Timeout maps to
//!   `Verdict::Escalate` rather than `Block` — when the judge is silent
//!   we don't have grounds to refuse, but we shouldn't auto-allow either.
//!
//! For PR 3, Tiers 2 and 3 are stubs returning `Skipped`. The orchestrator
//! is shipped with a `TierRunner` trait so future PRs swap real impls
//! without rewriting the cancellation/timeout/aggregation logic, and so
//! tests can drive deterministic scenarios.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tl_core::{
    new_trace_id, CheckRequest, Decision, Tier, TierResult, TierStatus, TriggeredPolicy, Verdict,
};
use tl_policy::Policy;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::context::HandlerCtx;
use crate::tiers::{deterministic, fuzzy, llm};

/// Output of a single tier. Carries both the user-visible `TierResult`
/// and the orchestrator-facing `block` signal.
#[derive(Debug)]
pub struct TierOutput {
    pub result: TierResult,
    /// If `Some`, the orchestrator should treat this tier as a hard stop:
    /// fire the cancellation token, skip later tiers, and use this signal's
    /// verdict in the final `Decision`.
    pub block: Option<BlockSignal>,
}

#[derive(Debug, Clone)]
pub struct BlockSignal {
    pub verdict: Verdict,
    pub reason: String,
    pub safe_output: Option<String>,
}

/// Indirection point — production wires this to the real tier modules,
/// tests inject a mock to drive cancellation timing deterministically.
#[async_trait]
pub trait TierRunner: Send + Sync {
    fn run_tier1(&self, req: &CheckRequest, policies: &[Policy]) -> TierOutput;
    async fn run_tier2(
        &self,
        req: &CheckRequest,
        ctx: &HandlerCtx,
        cancel: CancellationToken,
    ) -> TierOutput;
    async fn run_tier3(
        &self,
        req: &CheckRequest,
        ctx: &HandlerCtx,
        cancel: CancellationToken,
    ) -> TierOutput;
}

pub struct DefaultTierRunner;

#[async_trait]
impl TierRunner for DefaultTierRunner {
    fn run_tier1(&self, req: &CheckRequest, policies: &[Policy]) -> TierOutput {
        deterministic::run(req, policies)
    }
    async fn run_tier2(
        &self,
        req: &CheckRequest,
        ctx: &HandlerCtx,
        cancel: CancellationToken,
    ) -> TierOutput {
        fuzzy::run(req, ctx, cancel).await
    }
    async fn run_tier3(
        &self,
        req: &CheckRequest,
        ctx: &HandlerCtx,
        cancel: CancellationToken,
    ) -> TierOutput {
        llm::run(req, ctx, cancel).await
    }
}

#[derive(Clone, Copy)]
pub struct OrchestrateConfig {
    pub tier3_deadline: Duration,
}

impl Default for OrchestrateConfig {
    fn default() -> Self {
        Self {
            // Matches the v0 chat budget for full Tier 1+2+3. Voice channels
            // tighten this elsewhere (see architecture.md latency budget).
            tier3_deadline: Duration::from_millis(700),
        }
    }
}

pub async fn run(
    runner: Arc<dyn TierRunner>,
    config: OrchestrateConfig,
    req: &CheckRequest,
    ctx: &HandlerCtx,
    policies: &[Policy],
    policy_cache_scope: &str,
) -> Decision {
    let total_start = Instant::now();
    let trace_id = req.trace_id.clone().unwrap_or_else(new_trace_id);

    // -- Cache lookup --
    // Compute the cache key once and consult the cache before doing any
    // tier work. On hit we return immediately with the original verdict
    // but a fresh trace_id (callers can still correlate by request id).
    // `MokaCache::disabled()` always misses, so this is free in test /
    // no-op contexts.
    let cache_key = tl_cache::for_check_request_with_policy_scope(req, policy_cache_scope);
    if let Some(mut cached) = ctx.cache.get(&cache_key).await {
        cached.trace_id = trace_id.clone();
        cached.latency_ms = total_start.elapsed().as_millis() as u64;
        cached.redaction = req.redaction.clone();
        return cached;
    }

    let cancel = CancellationToken::new();

    // Spawn tier 2 + 3 first so they're truly running in parallel by the
    // time we evaluate Tier 1's quick result.
    let runner2 = runner.clone();
    let req2 = req.clone();
    let ctx2 = ctx.clone();
    let cancel2 = cancel.clone();
    let t2 = tokio::spawn(async move { runner2.run_tier2(&req2, &ctx2, cancel2).await });

    let runner3 = runner.clone();
    let req3 = req.clone();
    let ctx3 = ctx.clone();
    let cancel3 = cancel.clone();
    let deadline = config.tier3_deadline;
    let t3 = tokio::spawn(async move {
        tokio::select! {
            _ = sleep(deadline) => TierOutput {
                result: TierResult {
                    tier: Tier::Llm,
                    status: TierStatus::TimedOut,
                    reasons: vec![],
                    elapsed_ms: deadline.as_millis() as u64,
                },
                block: None,
            },
            out = runner3.run_tier3(&req3, &ctx3, cancel3) => out,
        }
    });

    // Tier 1 runs inline — it's CPU-only and microsecond-scale.
    let r1 = runner.run_tier1(req, policies);
    if r1.block.is_some() {
        cancel.cancel();
    }

    let r2 = t2.await.expect("tier2 task panicked");
    if r1.block.is_none() && r2.block.is_some() {
        cancel.cancel();
    }

    let r3 = t3.await.expect("tier3 task panicked");

    let decision = aggregate(trace_id, total_start, r1, r2, r3, req.redaction.clone());
    ctx.cache.put(cache_key, decision.clone()).await;
    decision
}

fn aggregate(
    trace_id: String,
    started_at: Instant,
    r1: TierOutput,
    r2: TierOutput,
    r3: TierOutput,
    redaction: Option<tl_core::RedactionInfo>,
) -> Decision {
    // First non-None block wins. Tier ordering is deliberate: a tier 1
    // verdict is more authoritative than tier 3 because it never depends
    // on a probabilistic judgement.
    let blocking = r1
        .block
        .as_ref()
        .or(r2.block.as_ref())
        .or(r3.block.as_ref())
        .cloned();

    // A tier 3 timeout is itself an escalation signal — we can't auto-allow
    // when the judge timed out, but we also can't block without grounds.
    let timeout_escalate = blocking.is_none() && r3.result.status == TierStatus::TimedOut;

    let mut triggered: Vec<TriggeredPolicy> = vec![];
    triggered.extend(r1.result.reasons.iter().cloned());
    triggered.extend(r2.result.reasons.iter().cloned());
    triggered.extend(r3.result.reasons.iter().cloned());

    let (verdict, reason, safe_output) = if let Some(block) = blocking {
        (block.verdict, block.reason, block.safe_output)
    } else if timeout_escalate {
        (Verdict::Escalate, "tier 3 LLM judge timed out".into(), None)
    } else {
        (
            Verdict::Allow,
            if triggered.is_empty() {
                "no policies triggered".into()
            } else {
                format!("{} non-blocking reasons", triggered.len())
            },
            None,
        )
    };

    // Legacy async path: once the event decision composer owns async Decision
    // construction, move these evidence defaults into that composer or builder.
    Decision {
        trace_id,
        verdict,
        reason,
        triggered_policies: triggered,
        safe_output,
        checked_input_excerpt: None,
        checked_output_excerpt: None,
        latency_ms: started_at.elapsed().as_millis() as u64,
        tier_results: vec![r1.result, r2.result, r3.result],
        redaction,
        violated_rule: None,
        remediation: None,
        source_chain: None,
        risk_source: None,
        failure_mode: None,
        harm_class: None,
        constraints: None,
    }
}
