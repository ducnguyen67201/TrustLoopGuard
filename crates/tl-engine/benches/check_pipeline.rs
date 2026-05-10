//! Microbench for the synchronous and async hot paths.
//!
//! PR 3 establishes the floor: empty policies, all-allow path. PR 20 will
//! add fuller scenarios (50 policies + universal patterns + Tier 2/3 with
//! mock LLM) once those tiers exist.

use criterion::{criterion_group, criterion_main, Criterion};
use tl_core::{Channel, CheckRequest};
use tl_engine::{Engine, HandlerCtx};
use tokio::runtime::Runtime;

fn req() -> CheckRequest {
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

fn bench_check_sync(c: &mut Criterion) {
    let eng = Engine::empty();
    let r = req();
    c.bench_function("check_sync_empty_policies", |b| {
        b.iter(|| eng.check(&r));
    });
}

fn bench_check_async_default(c: &mut Criterion) {
    let rt = Runtime::new().expect("rt");
    let eng = Engine::empty();
    let r = req();
    let ctx = HandlerCtx::no_op();
    c.bench_function("check_async_empty_policies_stub_tiers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = eng.check_async(&r, &ctx).await;
            })
        });
    });
}

criterion_group!(benches, bench_check_sync, bench_check_async_default);
criterion_main!(benches);
