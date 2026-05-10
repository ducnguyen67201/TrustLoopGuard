//! Microbench for the synchronous and async hot paths.
//!
//! Establishes the Tier 1 floor: stub Tiers 2/3 + universal baselines +
//! up to 50 tenant policies. PR 20 will replace the stubs with realistic
//! Tier 2 + Tier 3 to measure the full pipeline.

use criterion::{criterion_group, criterion_main, Criterion};
use tl_core::{Channel, CheckRequest};
use tl_engine::{Engine, HandlerCtx};
use tl_policy::{load_str, Policy};
use tokio::runtime::Runtime;

fn small_req() -> CheckRequest {
    CheckRequest {
        agent_id: "a".into(),
        channel: Channel::Chat,
        input: "hello".into(),
        proposed_output: "hi there".into(),
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
    }
}

/// 4KB-ish draft modelling a realistic agent response. No PII, no
/// injection markers — the worst-case for tier 1 is "scan everything,
/// match nothing".
fn large_req() -> CheckRequest {
    let body = "Thank you for reaching out. ".repeat(150);
    CheckRequest {
        agent_id: "a".into(),
        channel: Channel::Chat,
        input: "I have a question about my account".into(),
        proposed_output: body,
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
    }
}

/// 50 tenant policies, each with a unique literal matcher that won't
/// fire on the bench drafts. Stresses the per-policy iteration loop in
/// tier 1 without confounding the result with rewrite logic.
fn fifty_policies() -> Vec<Policy> {
    (0..50)
        .map(|i| {
            let yaml = format!(
                r#"
id: bench-pol-{i}
match:
  literal: "needle-{i}-{i}-never-in-text"
action: block
severity: low
"#
            );
            load_str(&yaml).expect("policy")
        })
        .collect()
}

fn bench_check_sync_empty(c: &mut Criterion) {
    let eng = Engine::empty();
    let r = small_req();
    c.bench_function("check_sync_empty_policies", |b| {
        b.iter(|| eng.check(&r));
    });
}

fn bench_check_async_empty_default(c: &mut Criterion) {
    let rt: Runtime = Runtime::new().expect("rt");
    let eng = Engine::empty();
    let r = small_req();
    let ctx = HandlerCtx::no_op();
    c.bench_function("check_async_empty_policies_stub_tiers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = eng.check_async(&r, &ctx).await;
            })
        });
    });
}

fn bench_check_async_50_policies_4kb(c: &mut Criterion) {
    let rt: Runtime = Runtime::new().expect("rt");
    let eng = Engine::new(fifty_policies());
    let r = large_req();
    let ctx = HandlerCtx::no_op();
    c.bench_function("check_async_50_policies_4kb_draft", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = eng.check_async(&r, &ctx).await;
            })
        });
    });
}

fn bench_universal_only_4kb(c: &mut Criterion) {
    // Tier 1 with no tenant policies but full universal baseline scan
    // against a 4KB draft. This isolates universal cost.
    let eng = Engine::empty();
    let r = large_req();
    c.bench_function("check_sync_universal_only_4kb", |b| {
        b.iter(|| eng.check(&r));
    });
}

criterion_group!(
    benches,
    bench_check_sync_empty,
    bench_check_async_empty_default,
    bench_check_async_50_policies_4kb,
    bench_universal_only_4kb,
);
criterion_main!(benches);
