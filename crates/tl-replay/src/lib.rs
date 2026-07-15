//! Replay a stored decision against a new engine snapshot. Lets customers
//! tune policies confidently: "if I add policy X, what would have happened
//! to yesterday's traffic?"

use serde::{Deserialize, Serialize};
use tl_core::{AuthorizationEffect, Decision};
use tl_engine::Engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayDiff {
    pub trace_id: String,
    pub before: AuthorizationEffect,
    pub after: AuthorizationEffect,
    pub changed: bool,
}

pub fn diff(original: &Decision, replayed: &Decision) -> ReplayDiff {
    ReplayDiff {
        trace_id: original.trace_id.clone(),
        before: original.effect,
        after: replayed.effect,
        changed: original.effect != replayed.effect,
    }
}

pub fn replay_against(
    engine: &Engine,
    original: &Decision,
    req: &tl_core::CheckRequest,
) -> ReplayDiff {
    let new_decision = engine.check(req);
    diff(original, &new_decision)
}
