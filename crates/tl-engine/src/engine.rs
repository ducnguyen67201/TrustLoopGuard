use std::sync::Arc;
use std::time::Instant;

use tl_core::{new_trace_id, CheckRequest, Decision, Verdict};
use tl_policy::Policy;

use crate::context::HandlerCtx;
use crate::pipeline::{self, DefaultTierRunner, OrchestrateConfig, TierRunner};
use crate::tiers::deterministic;

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

    /// Synchronous check — Tier 1 only. Designed to be called when the
    /// caller can't `.await` (replay, criterion benches, embedded users).
    /// Runs the same Tier 1 policy logic as `check_async` without spawning
    /// the Tier 2 / 3 tasks.
    pub fn check(&self, req: &CheckRequest) -> Decision {
        let start = Instant::now();
        let trace_id = req.trace_id.clone().unwrap_or_else(new_trace_id);

        let out = deterministic::run(req, &self.policies);

        let (verdict, reason, safe_output) = match out.block {
            Some(b) => (b.verdict, b.reason, b.safe_output),
            None => (
                Verdict::Allow,
                if out.result.reasons.is_empty() {
                    "no policies triggered".to_string()
                } else {
                    format!("{} non-blocking reasons", out.result.reasons.len())
                },
                None,
            ),
        };

        // Legacy sync path: once the event decision composer owns sync
        // Decision construction, move these evidence defaults into that
        // composer or builder.
        Decision {
            trace_id,
            verdict,
            effective_verdict: None,
            recommended_verdict: None,
            mode: None,
            reason,
            triggered_policies: out.result.reasons,
            safe_output,
            checked_input_excerpt: None,
            checked_output_excerpt: None,
            latency_ms: start.elapsed().as_millis() as u64,
            tier_results: vec![],
            redaction: req.redaction.clone(),
            violated_rule: None,
            remediation: None,
            source_chain: None,
            risk_source: None,
            failure_mode: None,
            harm_class: None,
            constraints: None,
        }
    }

    /// Full-pipeline async check. Runs Tier 1 + Tier 2 + Tier 3 in
    /// parallel with cancellation. See `orchestrate::run` for semantics.
    pub async fn check_async(&self, req: &CheckRequest, ctx: &HandlerCtx) -> Decision {
        self.check_async_with_policies(req, ctx, &self.policies)
            .await
    }

    /// Full-pipeline async check with a caller-supplied runtime policy set.
    /// Server deployments use this to resolve enabled cloud policies at
    /// request time while keeping the engine's pure matching semantics.
    pub async fn check_async_with_policies(
        &self,
        req: &CheckRequest,
        ctx: &HandlerCtx,
        policies: &[Policy],
    ) -> Decision {
        let policy_cache_scope = policy_cache_scope(policies);
        pipeline::orchestrator::run(
            self.runner.clone(),
            self.config,
            req,
            ctx,
            policies,
            &policy_cache_scope,
        )
        .await
    }
}

fn policy_cache_scope(policies: &[Policy]) -> String {
    let mut values: Vec<_> = policies
        .iter()
        .map(|policy| {
            (
                policy.id.as_str(),
                serde_json::to_value(policy).unwrap_or(serde_json::Value::Null),
            )
        })
        .collect();
    values.sort_by(|a, b| a.0.cmp(b.0));
    serde_json::to_string(&values).unwrap_or_default()
}
