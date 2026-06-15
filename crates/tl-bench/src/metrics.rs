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

/// Benchmark arm under test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmKind {
    /// Checkers are off; this models the same agent without TrustLoopGuard enforcement.
    Unguarded,
    /// Checkers are in enforce mode; this models the TrustLoopGuard-protected agent.
    Guarded,
}

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

/// Cross-session memory-poisoning stage metrics for a benchmark arm.
///
/// The current deterministic seed set models the delayed-memory chain as a
/// compact simulated sink: if the untrusted memory write is allowed, the same
/// payload is considered injectable, retrievable, and usable in a later
/// privileged action. Layer 2 can later replace that simulation with observed
/// multi-session traces while preserving the report surface.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStageMetrics {
    pub injection_rate: f64,
    pub retrieval_rate: f64,
    pub adversarial_usage_rate: f64,
}

/// Per-track sink/outcome counts for one differential arm.
#[derive(Debug, Clone, Serialize)]
pub struct ArmTrackBreakdown {
    pub track: Track,
    pub attacks: usize,
    pub attacks_succeeded: usize,
    pub attacks_blocked: usize,
    pub benign: usize,
    pub benign_succeeded: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryStageMetrics>,
}

/// Aggregate sink/outcome metrics for one benchmark arm.
#[derive(Debug, Clone, Serialize)]
pub struct BenchArmReport {
    pub arm: ArmKind,
    pub scenarios_run: usize,
    pub benign_utility_rate: f64,
    pub utility_under_attack_rate: f64,
    pub attack_success_rate: f64,
    pub false_block_rate: f64,
    pub mean_latency_us: u64,
    pub tracks: Vec<ArmTrackBreakdown>,
}

impl BenchArmReport {
    pub fn track(&self, track: Track) -> Option<&ArmTrackBreakdown> {
        self.tracks
            .iter()
            .find(|breakdown| breakdown.track == track)
    }
}

/// Guarded-vs-unguarded deltas. Positive `attack_success_rate_reduction` means
/// the guard reduced successful attacks.
#[derive(Debug, Clone, Serialize)]
pub struct BenchDelta {
    pub attack_success_rate_reduction: f64,
    pub benign_utility_delta: f64,
    pub utility_under_attack_delta: f64,
    pub false_block_delta: f64,
    pub latency_overhead_us: i64,
}

/// Differential benchmark report comparing the same scenario suite across arms.
#[derive(Debug, Clone, Serialize)]
pub struct DifferentialBenchReport {
    pub scenarios_run: usize,
    pub arms: Vec<BenchArmReport>,
    pub delta: BenchDelta,
}

impl DifferentialBenchReport {
    pub fn arm(&self, arm: ArmKind) -> Option<&BenchArmReport> {
        self.arms.iter().find(|report| report.arm == arm)
    }
}
