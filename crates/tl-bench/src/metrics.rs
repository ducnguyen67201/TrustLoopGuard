//! Behavioral metrics reported by a bench run.
//!
//! Mapping to the Phase 7 spec metric list:
//!
//! - attack success rate = `1.0 - attack_catch_rate`
//! - unsafe source-to-sink rate / parameter-source catch rate /
//!   unsafe-memory catch rate: per-track `attacks_caught / attacks` in
//!   [`TrackBreakdown`]
//! - benign task completion = `benign_completion_rate`
//! - false-block rate and false-escalation rate are folded into
//!   `false_block_rate` (any `Block` or `Escalate` on a benign scenario)
//! - latency overhead = `mean_latency_us` (informational; the criterion
//!   microbenchmarks in `tl-engine/benches` remain the latency gate)
//! - LLM calls per decision, cost per request, and trace explanation
//!   quality are not yet measured.

use serde::Serialize;

use crate::scenarios::Track;

/// Per-track scenario counts for one bench run.
#[derive(Debug, Clone, Serialize)]
pub struct TrackBreakdown {
    pub track: Track,
    /// Scenarios expected to be caught under enforce.
    pub attacks: usize,
    /// Attack scenarios that resolved `Block` or `Escalate`.
    pub attacks_caught: usize,
    /// Scenarios expected to stay allowed under enforce.
    pub benign: usize,
    /// Benign scenarios that resolved `Allow`.
    pub benign_passed: usize,
}

/// Aggregate behavioral report for one bench run.
#[derive(Debug, Clone, Serialize)]
pub struct BenchReport {
    pub scenarios_run: usize,
    /// Fraction of attack scenarios that resolved `Block` or `Escalate`.
    pub attack_catch_rate: f64,
    /// Fraction of benign scenarios that resolved `Block` or `Escalate`.
    pub false_block_rate: f64,
    /// Fraction of benign scenarios that resolved `Allow`.
    pub benign_completion_rate: f64,
    /// Mean wall-clock time per `process()` call. Informational only.
    pub mean_latency_us: u64,
    /// Per-track breakdown, in [`Track::ALL`] order.
    pub tracks: Vec<TrackBreakdown>,
}
