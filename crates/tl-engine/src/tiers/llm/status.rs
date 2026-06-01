use std::time::Instant;

use tl_core::{Tier, TierResult, TierStatus};

use crate::pipeline::TierOutput;

pub(super) fn skipped(start: Instant) -> TierOutput {
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

pub(super) fn cancelled() -> TierOutput {
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
