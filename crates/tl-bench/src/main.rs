//! TrustLoopGuardBench CLI: run the seed scenarios as a guarded-vs-unguarded
//! differential and print the behavioral report.
//!
//! Usage: `cargo run -p tl-bench` for a readable table, or
//! `cargo run -p tl-bench -- --json` for the serialized report.

use tl_bench::{
    run_differential_scenarios, seed_scenarios, BenchArmReport, DifferentialBenchReport,
};

fn print_table(report: &DifferentialBenchReport) {
    println!("TrustLoopGuardBench — deterministic guarded-vs-unguarded differential");
    println!();
    println!("  scenarios_run           {}", report.scenarios_run);
    println!(
        "  asr_reduction           {:.2}",
        report.delta.attack_success_rate_reduction
    );
    println!(
        "  benign_utility_delta    {:.2}",
        report.delta.benign_utility_delta
    );
    println!(
        "  ua_delta                {:.2}",
        report.delta.utility_under_attack_delta
    );
    println!(
        "  false_block_delta       {:.2}",
        report.delta.false_block_delta
    );
    println!(
        "  latency_overhead_us     {}",
        report.delta.latency_overhead_us
    );
    println!();
    println!(
        "  {:<12} {:>7} {:>7} {:>7} {:>11} {:>8}",
        "arm", "BU", "UA", "ASR", "false_block", "lat_us"
    );
    for arm in &report.arms {
        print_arm(arm);
    }
}

fn print_arm(report: &BenchArmReport) {
    println!(
        "  {:<12} {:>7.2} {:>7.2} {:>7.2} {:>11.2} {:>8}",
        format!("{:?}", report.arm).to_ascii_lowercase(),
        report.benign_utility_rate,
        report.utility_under_attack_rate,
        report.attack_success_rate,
        report.false_block_rate,
        report.mean_latency_us
    );
    for breakdown in &report.tracks {
        println!(
            "    {:<28} attacks={:<2} landed={:<2} benign={:<2} utility={:<2}",
            breakdown.track.as_str(),
            breakdown.attacks,
            breakdown.attacks_succeeded,
            breakdown.benign,
            breakdown.benign_succeeded
        );
    }
}

#[tokio::main]
async fn main() {
    let json = std::env::args().any(|arg| arg == "--json");
    let scenarios = seed_scenarios();
    let report = run_differential_scenarios(&scenarios).await;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report serializes")
        );
    } else {
        print_table(&report);
    }
}
