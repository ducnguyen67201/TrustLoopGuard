//! TrustLoopGuardBench CLI: run the seed scenarios under all-enforce
//! checker modes and print the behavioral report.
//!
//! Usage: `cargo run -p tl-bench` for a readable table, or
//! `cargo run -p tl-bench -- --json` for the serialized report.

use tl_bench::{enforce_all_modes, run_scenarios, seed_scenarios, BenchReport};

fn print_table(report: &BenchReport) {
    println!("TrustLoopGuardBench v1 — event pipeline, all checkers in enforce mode");
    println!();
    println!("  scenarios_run           {}", report.scenarios_run);
    println!("  attack_catch_rate       {:.2}", report.attack_catch_rate);
    println!("  false_block_rate        {:.2}", report.false_block_rate);
    println!(
        "  benign_completion_rate  {:.2}",
        report.benign_completion_rate
    );
    println!("  mean_latency_us         {}", report.mean_latency_us);
    println!();
    println!(
        "  {:<28} {:>7} {:>7} {:>7} {:>7}",
        "track", "attacks", "caught", "benign", "passed"
    );
    for breakdown in &report.tracks {
        println!(
            "  {:<28} {:>7} {:>7} {:>7} {:>7}",
            breakdown.track.as_str(),
            breakdown.attacks,
            breakdown.attacks_caught,
            breakdown.benign,
            breakdown.benign_passed
        );
    }
}

#[tokio::main]
async fn main() {
    let json = std::env::args().any(|arg| arg == "--json");
    let scenarios = seed_scenarios();
    let report = run_scenarios(&scenarios, enforce_all_modes()).await;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report serializes")
        );
    } else {
        print_table(&report);
    }
}
