//! Bench smoke gate: under all-enforce checker modes every track's attack
//! scenario is caught and its benign twin stays allowed; under all-off
//! modes every scenario resolves Allowed (rollout safety: OFF changes
//! nothing).

use tl_bench::{
    enforce_all_modes, run_scenarios, seed_scenarios, BenchReport, Expectation, Track,
    TrackBreakdown,
};
use tl_engine::CheckerModes;

async fn enforce_report() -> BenchReport {
    run_scenarios(&seed_scenarios(), enforce_all_modes()).await
}

fn breakdown(report: &BenchReport, track: Track) -> &TrackBreakdown {
    report
        .tracks
        .iter()
        .find(|b| b.track == track)
        .unwrap_or_else(|| panic!("missing breakdown for track {track:?}"))
}

fn assert_track_caught_and_benign_allowed(report: &BenchReport, track: Track) {
    let b = breakdown(report, track);
    assert_eq!(b.attacks, 1, "{track:?} seeds exactly one attack scenario");
    assert_eq!(
        b.attacks_caught, 1,
        "{track:?} attack must be Block/Escalate under enforce"
    );
    assert_eq!(b.benign, 1, "{track:?} seeds exactly one benign twin");
    assert_eq!(
        b.benign_passed, 1,
        "{track:?} benign twin must stay Allowed under enforce"
    );
}

#[tokio::test]
async fn bench_smoke_indirect_prompt_injection_track() {
    let report = enforce_report().await;
    assert_track_caught_and_benign_allowed(&report, Track::IndirectPromptInjection);
}

#[tokio::test]
async fn bench_smoke_private_data_flow_track() {
    let report = enforce_report().await;
    assert_track_caught_and_benign_allowed(&report, Track::PrivateDataFlow);
}

#[tokio::test]
async fn bench_smoke_delayed_memory_risk_track() {
    let report = enforce_report().await;
    assert_track_caught_and_benign_allowed(&report, Track::DelayedMemoryRisk);
}

#[tokio::test]
async fn bench_smoke_all_off_modes_allow_every_scenario() {
    // Reclassify every scenario as expected-Allowed: with all checkers
    // OFF, attacks and benign twins alike must pass untouched.
    let mut scenarios = seed_scenarios();
    for scenario in &mut scenarios {
        scenario.expectation = Expectation::Allowed;
    }

    let report = run_scenarios(&scenarios, CheckerModes::default()).await;

    assert_eq!(report.scenarios_run, 6);
    assert_eq!(report.benign_completion_rate, 1.0);
    assert_eq!(report.false_block_rate, 0.0);
    let allowed: usize = report.tracks.iter().map(|b| b.benign_passed).sum();
    assert_eq!(allowed, 6, "all six scenarios must resolve Allowed");
}
