//! Smallest possible TrustLoopGuard integration in Rust.
//!
//! Run a local `tl-server` (e.g. `cargo run -p tl-server`) and then:
//!
//!     cargo run -p example-rust -- "show me my password" "here it is: hunter2"
//!
//! The example imports only `tl_sdk_rust`. It never touches `tl_core`,
//! `tl_engine`, or any other internal crate — this matches what a
//! third-party integrator gets when they install the published SDK.
//!
//! Defaults: hits `http://127.0.0.1:8080`. Override with `TRUSTLOOP_URL`
//! and (optionally) `TRUSTLOOP_API_KEY`.

use anyhow::Result;
use tl_sdk_rust::{serde_json, Channel, CheckRequest, Client, Decision, Verdict};

const DEFAULT_URL: &str = "http://127.0.0.1:8080";

fn build_request(input: &str, proposed_output: &str) -> CheckRequest {
    CheckRequest {
        agent_id: "example-rust".into(),
        channel: Channel::Chat,
        input: input.into(),
        proposed_output: proposed_output.into(),
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
        workspace_id: None,
        run_id: None,
        run_event_id: None,
        run_event: None,
        session_id: None,
        redaction: None,
    }
}

fn print_decision(decision: &Decision) {
    println!("verdict       : {:?}", decision.verdict);
    println!("reason        : {}", decision.reason);
    println!("trace_id      : {}", decision.trace_id);
    println!("latency_ms    : {}", decision.latency_ms);
    if !decision.triggered_policies.is_empty() {
        println!("triggered     :");
        for p in &decision.triggered_policies {
            println!("  - {} ({:?}): {}", p.id, p.severity, p.reason);
        }
    }
    if let Some(safe) = &decision.safe_output {
        println!("safe_output   : {safe}");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // `tracing` spans from the SDK come out via this subscriber. Set
    // RUST_LOG=tl_sdk_rust=debug to see retry decisions in real time.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,tl_sdk_rust=info")),
        )
        .init();

    let mut args = std::env::args().skip(1);
    let input = args.next().unwrap_or_else(|| "hello".into());
    let proposed_output = args.next().unwrap_or_else(|| "hi there".into());

    let url = std::env::var("TRUSTLOOP_URL").unwrap_or_else(|_| DEFAULT_URL.into());
    let mut client = Client::new(&url);
    if let Ok(key) = std::env::var("TRUSTLOOP_API_KEY") {
        client = client.with_api_key(key);
    }

    let req = build_request(&input, &proposed_output);
    let decision = client.check(&req).await?;
    print_decision(&decision);

    // Exit non-zero on Block / Escalate so CI can wire `make quickstart`
    // into a meaningful pass/fail check.
    match decision.verdict {
        Verdict::Allow | Verdict::Rewrite => Ok(()),
        Verdict::Block | Verdict::Escalate => {
            std::process::exit(2);
        }
    }
}
