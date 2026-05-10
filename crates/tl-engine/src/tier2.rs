//! Tier 2 — fuzzy similarity. Real implementation as of PR 6.
//!
//! Delegates to `ctx.fuzzy.check(draft)` which encapsulates an HNSW
//! index over semantic policy matchers + a Levenshtein scan for typo
//! bypass. The default `NoOpFuzzyChecker` returns no hits, in which
//! case this tier reports `Skipped`.

use std::time::Instant;

use tl_core::{CheckRequest, Tier, TierResult, TierStatus, TriggeredPolicy, Verdict};
use tl_policy::Action;
use tokio_util::sync::CancellationToken;

use crate::handler::{FuzzyHit, HandlerCtx};
use crate::orchestrate::{BlockSignal, TierOutput};

pub async fn run(req: &CheckRequest, ctx: &HandlerCtx, cancel: CancellationToken) -> TierOutput {
    let start = Instant::now();
    if cancel.is_cancelled() {
        return cancelled();
    }

    let hits = tokio::select! {
        _ = cancel.cancelled() => return cancelled(),
        h = ctx.fuzzy.check(&req.proposed_output) => h,
    };

    if hits.is_empty() {
        return TierOutput {
            result: TierResult {
                tier: Tier::Fuzzy,
                status: TierStatus::Skipped,
                reasons: vec![],
                elapsed_ms: start.elapsed().as_millis() as u64,
            },
            block: None,
        };
    }

    let mut reasons: Vec<TriggeredPolicy> = vec![];
    let mut block: Option<BlockSignal> = None;

    for hit in hits {
        reasons.push(TriggeredPolicy {
            id: hit.policy_id.clone(),
            severity: hit.severity,
            reason: hit.message.clone(),
        });
        if block.is_none() {
            if let Some(signal) = block_signal_from_hit(&hit) {
                block = Some(signal);
            }
        }
    }

    TierOutput {
        result: TierResult {
            tier: Tier::Fuzzy,
            status: TierStatus::Completed,
            reasons,
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        block,
    }
}

fn block_signal_from_hit(hit: &FuzzyHit) -> Option<BlockSignal> {
    let verdict = match hit.action {
        Action::Allow => return None,
        Action::Block => Verdict::Block,
        Action::Rewrite => Verdict::Rewrite,
        Action::Escalate => Verdict::Escalate,
    };
    Some(BlockSignal {
        verdict,
        reason: format!("tier2 policy `{}` triggered: {}", hit.policy_id, hit.message),
        safe_output: hit.safe_output.clone(),
    })
}

fn cancelled() -> TierOutput {
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
