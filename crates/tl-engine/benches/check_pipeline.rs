//! Microbench for the synchronous and async hot paths.
//!
//! Establishes the floor numbers we commit to in
//! `docs/concept/v0-design-decisions.md §6`. Scenarios in this file:
//!
//! - `check_sync_empty_policies`              — Tier 1 only, no work
//! - `check_async_empty_policies_stub_tiers`  — full async pipeline, no work
//! - `check_async_50_policies_4kb_draft`      — realistic tenant load
//! - `check_sync_universal_only_4kb`          — universal cost in isolation
//! - `check_async_cache_hit_path`             — second identical request
//! - `check_sync_pii_block_4kb`               — universal block path
//!
//! Run all of them with `cargo bench -p tl-engine`. The criterion HTML
//! report lands at `target/criterion/report/index.html`.

use criterion::{criterion_group, criterion_main, Criterion};
use tl_core::{Channel, CheckRequest};
use tl_engine::{Engine, HandlerCtx};
use tl_policy::{load_str, Policy};
use tokio::runtime::Runtime;

fn small_req() -> CheckRequest {
    CheckRequest {
        workspace_id: None,
        run_id: None,
        run_event_id: None,
        run_event: None,
        agent_id: "a".into(),
        channel: Channel::Chat,
        input: "hello".into(),
        proposed_output: "hi there".into(),
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
        redaction: None,
    }
}

/// 4KB-ish draft modelling a realistic agent response. No PII, no
/// injection markers — the worst-case for tier 1 is "scan everything,
/// match nothing".
fn large_req() -> CheckRequest {
    let body = "Thank you for reaching out. ".repeat(150);
    CheckRequest {
        workspace_id: None,
        run_id: None,
        run_event_id: None,
        run_event: None,
        agent_id: "a".into(),
        channel: Channel::Chat,
        input: "I have a question about my account".into(),
        proposed_output: body,
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
        redaction: None,
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

fn bench_check_async_cache_hit(c: &mut Criterion) {
    // Identical req → cache lookup hits after the first call. This
    // measures the hot path under "duplicate request burst" conditions
    // (retries, double-clicks, fan-out). Should beat the miss path
    // significantly because all tiers are skipped.
    let rt: Runtime = Runtime::new().expect("rt");
    let eng = Engine::empty();
    let r = small_req();
    let ctx = HandlerCtx::no_op();
    // Warm the cache once outside the measurement.
    rt.block_on(async {
        let _ = eng.check_async(&r, &ctx).await;
    });
    c.bench_function("check_async_cache_hit_path", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = eng.check_async(&r, &ctx).await;
            })
        });
    });
}

fn bench_check_sync_pii_block_4kb(c: &mut Criterion) {
    // Same 4KB draft but with a PII match buried in the middle. The
    // universal::pii detector should still complete in tier 1 budget;
    // this number is what we cite for "block latency" in the spec.
    let mut body = "Thank you for reaching out. ".repeat(74);
    body.push_str(" Reach me at 415-555-1212 if needed. ");
    body.push_str(&"Thank you for reaching out. ".repeat(75));
    let req = CheckRequest {
        workspace_id: None,
        run_id: None,
        run_event_id: None,
        run_event: None,
        agent_id: "a".into(),
        channel: Channel::Chat,
        input: "send me your number".into(),
        proposed_output: body,
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
        redaction: None,
    };
    let eng = Engine::empty();
    c.bench_function("check_sync_pii_block_4kb", |b| {
        b.iter(|| eng.check(&req));
    });
}

criterion_group!(
    benches,
    bench_check_sync_empty,
    bench_check_async_empty_default,
    bench_check_async_50_policies_4kb,
    bench_universal_only_4kb,
    bench_check_async_cache_hit,
    bench_check_sync_pii_block_4kb,
);
criterion_main!(benches);
