//! TrustLoopGuardBench v1: a behavioral regression harness for the event
//! pipeline.
//!
//! The bench runs the seed attack/benign scenarios through a real
//! `EventPipelineCtx` (the four deterministic checkers plus the
//! mode-aware composer, no-op everything else) and reports whether
//! attacks are caught and benign twins stay allowed. It is
//! framework-free: no server, no storage, no live LLM calls.

pub mod metrics;
pub mod scenarios;

pub use metrics::{
    ArmKind, ArmTrackBreakdown, BenchArmReport, BenchDelta, BenchReport, DifferentialBenchReport,
    MemoryStageMetrics, TrackBreakdown,
};
pub use scenarios::{seed_scenarios, Expectation, Scenario, Track};

use std::sync::Arc;
use std::time::Instant;

use tl_core::{new_trace_id, Decision, EnforcementMode, Verdict};
use tl_engine::{
    ApprovalChecker, CheckerModes, EventPipelineCtx, InformationFlowChecker, MemoryChecker,
    ModeAwareDecisionComposer, ParameterAuthChecker,
};

/// All four checkers in enforce mode: the configuration the seed
/// scenarios are graded against.
pub fn enforce_all_modes() -> CheckerModes {
    CheckerModes {
        information_flow: EnforcementMode::Enforce,
        memory: EnforcementMode::Enforce,
        parameter_auth: EnforcementMode::Enforce,
        approval: EnforcementMode::Enforce,
    }
}

/// All four checkers off: the unguarded baseline arm.
pub fn off_all_modes() -> CheckerModes {
    CheckerModes::default()
}

/// The bench pipeline: real checkers and composer over otherwise no-op
/// stages, so scenario events reach the checkers exactly as declared.
fn bench_pipeline_ctx() -> EventPipelineCtx {
    EventPipelineCtx {
        checkers: vec![
            Arc::new(InformationFlowChecker),
            Arc::new(MemoryChecker),
            Arc::new(ParameterAuthChecker),
            Arc::new(ApprovalChecker),
        ],
        composer: Arc::new(ModeAwareDecisionComposer),
        ..EventPipelineCtx::no_op()
    }
}

/// `Caught` per the bench contract: the verdict refuses to let the action
/// proceed unattended.
fn is_caught(verdict: Verdict) -> bool {
    matches!(verdict, Verdict::Block | Verdict::Escalate)
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

/// Run every scenario through the event pipeline under the given checker
/// modes and compile the behavioral report.
///
/// Each scenario is seeded with a fresh `Decision::allow`, so the report
/// measures exactly what the checkers and composer contribute.
pub async fn run_scenarios(scenarios: &[Scenario], modes: CheckerModes) -> BenchReport {
    let ctx = bench_pipeline_ctx();
    let mut tracks: Vec<TrackBreakdown> = Track::ALL
        .iter()
        .map(|&track| TrackBreakdown {
            track,
            attacks: 0,
            attacks_caught: 0,
            benign: 0,
            benign_passed: 0,
        })
        .collect();
    let mut total_latency_us: u128 = 0;

    for scenario in scenarios {
        let started = Instant::now();
        let (_event, decision) = ctx
            .process(
                scenario.event.clone(),
                "ws_bench",
                "production",
                modes,
                Decision::allow(new_trace_id()),
            )
            .await;
        total_latency_us += started.elapsed().as_micros();

        let breakdown = &mut tracks[scenario.track.index()];
        match scenario.expectation {
            Expectation::Caught => {
                breakdown.attacks += 1;
                if is_caught(decision.verdict) {
                    breakdown.attacks_caught += 1;
                }
            }
            Expectation::Allowed => {
                breakdown.benign += 1;
                if decision.verdict == Verdict::Allow {
                    breakdown.benign_passed += 1;
                }
            }
        }
    }

    let attacks: usize = tracks.iter().map(|t| t.attacks).sum();
    let attacks_caught: usize = tracks.iter().map(|t| t.attacks_caught).sum();
    let benign: usize = tracks.iter().map(|t| t.benign).sum();
    let benign_passed: usize = tracks.iter().map(|t| t.benign_passed).sum();
    let mean_latency_us = if scenarios.is_empty() {
        0
    } else {
        (total_latency_us / scenarios.len() as u128) as u64
    };

    BenchReport {
        scenarios_run: scenarios.len(),
        attack_catch_rate: rate(attacks_caught, attacks),
        // Benign scenarios resolve Allow, Block, or Escalate here (the
        // composer only ever upgrades toward Block), so "not passed" and
        // "false-blocked" coincide; both are reported for spec parity.
        false_block_rate: rate(benign - benign_passed, benign),
        benign_completion_rate: rate(benign_passed, benign),
        mean_latency_us,
        tracks,
    }
}

/// Run the seed suite as a guarded-vs-unguarded differential benchmark.
pub async fn run_differential_scenarios(scenarios: &[Scenario]) -> DifferentialBenchReport {
    let unguarded = run_arm_scenarios(ArmKind::Unguarded, scenarios, off_all_modes()).await;
    let guarded = run_arm_scenarios(ArmKind::Guarded, scenarios, enforce_all_modes()).await;
    let delta = BenchDelta {
        attack_success_rate_reduction: unguarded.attack_success_rate - guarded.attack_success_rate,
        benign_utility_delta: guarded.benign_utility_rate - unguarded.benign_utility_rate,
        utility_under_attack_delta: guarded.utility_under_attack_rate
            - unguarded.utility_under_attack_rate,
        false_block_delta: guarded.false_block_rate - unguarded.false_block_rate,
        latency_overhead_us: guarded.mean_latency_us as i64 - unguarded.mean_latency_us as i64,
    };

    DifferentialBenchReport {
        scenarios_run: scenarios.len(),
        arms: vec![unguarded, guarded],
        delta,
    }
}

async fn run_arm_scenarios(
    arm: ArmKind,
    scenarios: &[Scenario],
    modes: CheckerModes,
) -> BenchArmReport {
    let ctx = bench_pipeline_ctx();
    let mut tracks: Vec<ArmTrackBreakdown> = Track::ALL
        .iter()
        .map(|&track| ArmTrackBreakdown {
            track,
            attacks: 0,
            attacks_succeeded: 0,
            attacks_blocked: 0,
            benign: 0,
            benign_succeeded: 0,
            memory: None,
        })
        .collect();
    let mut total_latency_us: u128 = 0;

    for scenario in scenarios {
        let started = Instant::now();
        let (_event, decision) = ctx
            .process(
                scenario.event.clone(),
                "ws_bench",
                "production",
                modes,
                Decision::allow(new_trace_id()),
            )
            .await;
        total_latency_us += started.elapsed().as_micros();

        let breakdown = &mut tracks[scenario.track.index()];
        match scenario.expectation {
            Expectation::Caught => {
                breakdown.attacks += 1;
                if is_caught(decision.verdict) {
                    breakdown.attacks_blocked += 1;
                } else {
                    breakdown.attacks_succeeded += 1;
                }
            }
            Expectation::Allowed => {
                breakdown.benign += 1;
                if decision.verdict == Verdict::Allow {
                    breakdown.benign_succeeded += 1;
                }
            }
        }
    }

    for breakdown in &mut tracks {
        if breakdown.track == Track::DelayedMemoryRisk {
            let stage_rate = rate(breakdown.attacks_succeeded, breakdown.attacks);
            breakdown.memory = Some(MemoryStageMetrics {
                injection_rate: stage_rate,
                retrieval_rate: stage_rate,
                adversarial_usage_rate: stage_rate,
            });
        }
    }

    let attacks: usize = tracks.iter().map(|t| t.attacks).sum();
    let attacks_succeeded: usize = tracks.iter().map(|t| t.attacks_succeeded).sum();
    let benign: usize = tracks.iter().map(|t| t.benign).sum();
    let benign_succeeded: usize = tracks.iter().map(|t| t.benign_succeeded).sum();
    let mean_latency_us = if scenarios.is_empty() {
        0
    } else {
        (total_latency_us / scenarios.len() as u128) as u64
    };

    BenchArmReport {
        arm,
        scenarios_run: scenarios.len(),
        benign_utility_rate: rate(benign_succeeded, benign),
        // The deterministic seed set does not yet include separate
        // attack-under-task traces, so UA follows benign utility until those
        // scenarios are added. Keeping the field separate prevents ASR/utility
        // conflation at the report boundary.
        utility_under_attack_rate: rate(benign_succeeded, benign),
        attack_success_rate: rate(attacks_succeeded, attacks),
        false_block_rate: rate(benign - benign_succeeded, benign),
        mean_latency_us,
        tracks,
    }
}
