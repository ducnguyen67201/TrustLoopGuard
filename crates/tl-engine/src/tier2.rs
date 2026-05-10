//! Tier 2 — fuzzy similarity. **Stub for PR 3.**
//!
//! The real implementation lands in PR 5 (`tl-fuzzy` crate: `fastembed-rs` +
//! HNSW) and PR 6 (engine wiring). For now this returns `Skipped` so the
//! orchestrator's wire shape is exercised end-to-end without pulling in
//! a 100MB embedding model.

use std::time::Instant;

use tl_core::{CheckRequest, Tier, TierResult, TierStatus};
use tokio_util::sync::CancellationToken;

use crate::handler::HandlerCtx;
use crate::orchestrate::TierOutput;

pub async fn run(_req: &CheckRequest, _ctx: &HandlerCtx, cancel: CancellationToken) -> TierOutput {
    let start = Instant::now();
    // Honor cancellation immediately. Real Tier 2 will be 5-20ms; the stub
    // is instant. Either way we want to surface the cancelled status if
    // the orchestrator already decided to abort.
    if cancel.is_cancelled() {
        return TierOutput {
            result: TierResult {
                tier: Tier::Fuzzy,
                status: TierStatus::Cancelled,
                reasons: vec![],
                elapsed_ms: 0,
            },
            block: None,
        };
    }
    TierOutput {
        result: TierResult {
            tier: Tier::Fuzzy,
            status: TierStatus::Skipped,
            reasons: vec![],
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        block: None,
    }
}
