//! Decision engine. Two entry points:
//!
//! - `Engine::check` — synchronous, deterministic-only (Tier 1). Used by
//!   the existing `/v1/check` handler, the replay tool, and benchmarks.
//!   Microsecond-scale.
//!
//! - `Engine::check_async` — full pipeline through the parallel-cancel
//!   orchestrator. Tiers 2 and 3 are stubs in PR 3 and get fleshed out
//!   in PR 5/6 (`tl-fuzzy`) and PR 7-9 (`tl-llm`).

use std::sync::Arc;
use std::time::Instant;

use tl_core::{new_trace_id, CheckRequest, Decision, TriggeredPolicy, Verdict};
use tl_policy::Policy;

pub mod engine_match;
pub mod handler;
pub mod orchestrate;
pub mod tier1;
pub mod tier2;
pub mod tier3;

pub use handler::{
    DecisionCache, Embedder, HandlerCtx, JudgeError, LlmJudge, NoOpCache, NoOpEmbedder, NoOpJudge,
    NoOpProfileResolver, ProfileResolver,
};
pub use orchestrate::{
    BlockSignal, DefaultTierRunner, OrchestrateConfig, TierOutput, TierRunner,
};

pub struct Engine {
    policies: Vec<Policy>,
    runner: Arc<dyn TierRunner>,
    config: OrchestrateConfig,
}

impl Engine {
    pub fn new(policies: Vec<Policy>) -> Self {
        Self {
            policies,
            runner: Arc::new(DefaultTierRunner),
            config: OrchestrateConfig::default(),
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![])
    }

    /// Override the tier runner. Used by tests to drive deterministic
    /// scenarios for cancellation, timeout, and fan-out behaviour.
    pub fn with_runner(mut self, runner: Arc<dyn TierRunner>) -> Self {
        self.runner = runner;
        self
    }

    pub fn with_config(mut self, config: OrchestrateConfig) -> Self {
        self.config = config;
        self
    }

    pub fn policies(&self) -> &[Policy] {
        &self.policies
    }

    /// Synchronous check — Tier 1 only. Designed to be called on the hot
    /// path when the caller can't `.await` (replay, criterion benches,
    /// embedded users). Equivalent to running `check_async` against a
    /// Tier 2 + 3 that always Skip, but without the spawn/await overhead.
    pub fn check(&self, req: &CheckRequest) -> Decision {
        let start = Instant::now();
        let trace_id = req.trace_id.clone().unwrap_or_else(new_trace_id);

        let mut triggered: Vec<TriggeredPolicy> = vec![];
        for policy in &self.policies {
            if engine_match::policy_matches(policy, req) {
                triggered.push(TriggeredPolicy {
                    id: policy.id.clone(),
                    severity: policy.severity,
                    reason: format!("policy `{}` matched", policy.id),
                });
            }
        }

        let (verdict, safe_output, reason) = if triggered.is_empty() {
            (Verdict::Allow, None, "no policies triggered".to_string())
        } else {
            let first = &self.policies[0];
            let action = first.action;
            let v = match action {
                tl_policy::Action::Allow => Verdict::Allow,
                tl_policy::Action::Block => Verdict::Block,
                tl_policy::Action::Rewrite => Verdict::Rewrite,
                tl_policy::Action::Escalate => Verdict::Escalate,
            };
            let safe = first.rewrite.clone();
            (
                v,
                safe,
                format!("{} policy(ies) triggered", triggered.len()),
            )
        };

        Decision {
            trace_id,
            verdict,
            reason,
            triggered_policies: triggered,
            safe_output,
            latency_ms: start.elapsed().as_millis() as u64,
            tier_results: vec![],
        }
    }

    /// Full-pipeline async check. Runs Tier 1 + Tier 2 + Tier 3 in
    /// parallel with cancellation. See `orchestrate::run` for semantics.
    pub async fn check_async(&self, req: &CheckRequest, ctx: &HandlerCtx) -> Decision {
        orchestrate::run(self.runner.clone(), self.config, req, ctx, &self.policies).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use tl_core::{Channel, Tier, TierResult, TierStatus};
    use tokio_util::sync::CancellationToken;

    type BoxFut<T> = Pin<Box<dyn Future<Output = T> + Send>>;

    fn req() -> CheckRequest {
        CheckRequest {
            agent_id: "a".into(),
            channel: Channel::Chat,
            input: "hi".into(),
            proposed_output: "hello".into(),
            domain: None,
            policies: vec![],
            context: serde_json::Value::Null,
            trace_id: None,
        }
    }

    #[test]
    fn empty_engine_allows() {
        let eng = Engine::empty();
        let d = eng.check(&req());
        assert_eq!(d.verdict, Verdict::Allow);
    }

    // -------- Async orchestrator tests --------
    //
    // MockRunner lets each test drive the three tiers deterministically.
    // What we're verifying:
    //   1. All-allow path returns Allow with three Completed tier_results.
    //   2. Tier 1 block fires the cancel token; tiers 2 + 3 observe
    //      cancellation and report Cancelled status.
    //   3. Tier 1+2 allow + Tier 3 deadline elapses -> Escalate.
    //   4. Default runner (real tier1, stub tier2/tier3) yields Allow with
    //      Completed/Skipped/Skipped statuses.

    type MockTier1 = Box<dyn Fn() -> TierOutput + Send + Sync>;
    type MockTierAsync =
        Box<dyn Fn(CancellationToken) -> BoxFut<TierOutput> + Send + Sync>;

    struct MockRunner {
        t1: MockTier1,
        t2: MockTierAsync,
        t3: MockTierAsync,
    }

    #[async_trait]
    impl TierRunner for MockRunner {
        fn run_tier1(&self, _req: &CheckRequest, _policies: &[Policy]) -> TierOutput {
            (self.t1)()
        }
        async fn run_tier2(
            &self,
            _req: &CheckRequest,
            _ctx: &HandlerCtx,
            cancel: CancellationToken,
        ) -> TierOutput {
            (self.t2)(cancel).await
        }
        async fn run_tier3(
            &self,
            _req: &CheckRequest,
            _ctx: &HandlerCtx,
            cancel: CancellationToken,
        ) -> TierOutput {
            (self.t3)(cancel).await
        }
    }

    fn allow_output(tier: Tier) -> TierOutput {
        TierOutput {
            result: TierResult {
                tier,
                status: TierStatus::Completed,
                reasons: vec![],
                elapsed_ms: 0,
            },
            block: None,
        }
    }

    #[tokio::test]
    async fn three_allow_tiers_yield_allow_with_three_results() {
        let runner = Arc::new(MockRunner {
            t1: Box::new(|| allow_output(Tier::Deterministic)),
            t2: Box::new(|_| Box::pin(async { allow_output(Tier::Fuzzy) })),
            t3: Box::new(|_| Box::pin(async { allow_output(Tier::Llm) })),
        });
        let eng = Engine::empty().with_runner(runner);
        let d = eng.check_async(&req(), &HandlerCtx::no_op()).await;
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.tier_results.len(), 3);
        assert_eq!(d.tier_results[0].tier, Tier::Deterministic);
        assert_eq!(d.tier_results[1].tier, Tier::Fuzzy);
        assert_eq!(d.tier_results[2].tier, Tier::Llm);
    }

    #[tokio::test]
    async fn tier1_block_cancels_tiers_2_and_3() {
        let t2_cancelled = Arc::new(AtomicBool::new(false));
        let t3_cancelled = Arc::new(AtomicBool::new(false));
        let t2_flag = t2_cancelled.clone();
        let t3_flag = t3_cancelled.clone();

        let runner = Arc::new(MockRunner {
            t1: Box::new(|| TierOutput {
                result: TierResult {
                    tier: Tier::Deterministic,
                    status: TierStatus::Completed,
                    reasons: vec![],
                    elapsed_ms: 0,
                },
                block: Some(BlockSignal {
                    verdict: Verdict::Block,
                    reason: "tier 1 hard block".into(),
                    safe_output: None,
                }),
            }),
            t2: Box::new(move |cancel| {
                let flag = t2_flag.clone();
                Box::pin(async move {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            flag.store(true, Ordering::SeqCst);
                            TierOutput {
                                result: TierResult {
                                    tier: Tier::Fuzzy,
                                    status: TierStatus::Cancelled,
                                    reasons: vec![],
                                    elapsed_ms: 0,
                                },
                                block: None,
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_secs(5)) =>
                            allow_output(Tier::Fuzzy),
                    }
                })
            }),
            t3: Box::new(move |cancel| {
                let flag = t3_flag.clone();
                Box::pin(async move {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            flag.store(true, Ordering::SeqCst);
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
                        _ = tokio::time::sleep(Duration::from_secs(5)) =>
                            allow_output(Tier::Llm),
                    }
                })
            }),
        });

        let eng = Engine::empty().with_runner(runner);
        let d = eng.check_async(&req(), &HandlerCtx::no_op()).await;
        assert_eq!(d.verdict, Verdict::Block);
        assert!(t2_cancelled.load(Ordering::SeqCst), "tier 2 saw cancel");
        assert!(t3_cancelled.load(Ordering::SeqCst), "tier 3 saw cancel");
        assert_eq!(d.tier_results[1].status, TierStatus::Cancelled);
        assert_eq!(d.tier_results[2].status, TierStatus::Cancelled);
    }

    #[tokio::test]
    async fn tier3_timeout_escalates() {
        let runner = Arc::new(MockRunner {
            t1: Box::new(|| allow_output(Tier::Deterministic)),
            t2: Box::new(|_| Box::pin(async { allow_output(Tier::Fuzzy) })),
            t3: Box::new(|_| {
                Box::pin(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    allow_output(Tier::Llm)
                })
            }),
        });
        let eng = Engine::empty()
            .with_runner(runner)
            .with_config(OrchestrateConfig {
                tier3_deadline: Duration::from_millis(20),
            });
        let d = eng.check_async(&req(), &HandlerCtx::no_op()).await;
        assert_eq!(d.verdict, Verdict::Escalate);
        assert_eq!(d.tier_results[2].status, TierStatus::TimedOut);
    }

    #[tokio::test]
    async fn default_runner_with_no_policies_yields_allow() {
        // Default runner: real tier1 (no policies → no reasons), stub
        // tier2/tier3 returning Skipped.
        let eng = Engine::empty();
        let d = eng.check_async(&req(), &HandlerCtx::no_op()).await;
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.tier_results.len(), 3);
        assert_eq!(d.tier_results[0].status, TierStatus::Completed);
        assert_eq!(d.tier_results[1].status, TierStatus::Skipped);
        assert_eq!(d.tier_results[2].status, TierStatus::Skipped);
    }
}
