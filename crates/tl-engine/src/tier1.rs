//! Tier 1 — deterministic matchers (regex + literal). Microsecond-scale.
//!
//! Wraps the existing `engine_match::policy_matches` so the parallel-cancel
//! orchestrator can call into it like any other tier. Tier 1 is sync —
//! pure CPU work, no I/O — so we don't take a `CancellationToken`: by the
//! time tier 1 finishes there's nothing to cancel.

use std::time::Instant;

use tl_core::{CheckRequest, Tier, TierResult, TierStatus, TriggeredPolicy, Verdict};
use tl_policy::{Action, Policy};

use crate::engine_match::policy_matches;
use crate::orchestrate::{BlockSignal, TierOutput};

pub fn run(req: &CheckRequest, policies: &[Policy]) -> TierOutput {
    let start = Instant::now();
    let mut reasons: Vec<TriggeredPolicy> = vec![];
    let mut block: Option<BlockSignal> = None;

    for policy in policies {
        if !policy_matches(policy, req) {
            continue;
        }
        reasons.push(TriggeredPolicy {
            id: policy.id.clone(),
            severity: policy.severity,
            reason: format!("policy `{}` matched", policy.id),
        });
        // First triggered policy with a non-Allow action sets the verdict
        // for this tier. Severity-aware aggregation is deliberately deferred
        // — for v0 the policy author orders rules by priority.
        if block.is_none() {
            if let Some(signal) = block_signal_from_action(policy) {
                block = Some(signal);
            }
        }
    }

    TierOutput {
        result: TierResult {
            tier: Tier::Deterministic,
            status: TierStatus::Completed,
            reasons,
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        block,
    }
}

fn block_signal_from_action(policy: &Policy) -> Option<BlockSignal> {
    let verdict = match policy.action {
        Action::Allow => return None,
        Action::Block => Verdict::Block,
        Action::Rewrite => Verdict::Rewrite,
        Action::Escalate => Verdict::Escalate,
    };
    Some(BlockSignal {
        verdict,
        reason: format!("tier1 policy `{}` triggered", policy.id),
        safe_output: policy.rewrite.clone(),
    })
}
