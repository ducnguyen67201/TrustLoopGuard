//! Tier 3 — LLM judges. **Stub for PR 3.**
//!
//! The real implementation lands in PR 7-9 (`tl-llm` crate: provider trait,
//! `LlmRouter`, three judges via `tokio::join!`). For now this returns
//! `Skipped` so the orchestrator's deadline + cancel-token paths exercise
//! against a free, instant stub.

use std::time::Instant;

use tl_core::{CheckRequest, Tier, TierResult, TierStatus};
use tokio_util::sync::CancellationToken;

use crate::handler::HandlerCtx;
use crate::orchestrate::TierOutput;

pub async fn run(_req: &CheckRequest, _ctx: &HandlerCtx, cancel: CancellationToken) -> TierOutput {
    let start = Instant::now();
    if cancel.is_cancelled() {
        return TierOutput {
            result: TierResult {
                tier: Tier::Llm,
                status: TierStatus::Cancelled,
                reasons: vec![],
                elapsed_ms: 0,
            },
            block: None,
        };
    }
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
