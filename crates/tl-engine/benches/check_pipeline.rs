//! Microbench for the synchronous and async hot paths.
//!
//! Establishes the floor numbers we commit to in
//! `docs/concept/v0-design-decisions.md §6`. Scenarios in this file:
//!
//! - `check_sync_empty_policies`              — Tier 1 only, no work
//! - `check_async_empty_policies_stub_tiers`  — full async pipeline, no work
//! - `check_async_50_policies_4kb_draft`      — realistic tenant load
//! - `check_sync_empty_policies_4kb`          — large draft, no stored policies
//! - `check_async_cache_hit_path`             — second identical request
//! - `check_sync_policy_block_4kb`            — stored policy block path
//!
//! Run all of them with `cargo bench -p tl-engine`. The criterion HTML
//! report lands at `target/criterion/report/index.html`.

use criterion::{criterion_group, criterion_main, Criterion};
use tl_core::{
    AuthorizationSubject, Channel, CheckRequest, ShellActionParameters, ShellLanguage,
    SideEffectClass, ToolIdentity,
};
use tl_engine::{analyze_shell_command, evaluate_tool_policies, Engine, HandlerCtx};
use tl_policy::{load_any_str, load_str, AnyPolicy, FamilyPolicy, Policy};
use tokio::runtime::Runtime;

fn small_req() -> CheckRequest {
    CheckRequest {
        workspace_id: None,
        run_id: None,
        run_event_id: None,
        run_event: None,
        session_id: None,
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
        session_id: None,
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
action: deny
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

fn bench_check_sync_empty_4kb(c: &mut Criterion) {
    // Tier 1 with no stored policies against a 4KB draft.
    let eng = Engine::empty();
    let r = large_req();
    c.bench_function("check_sync_empty_policies_4kb", |b| {
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

fn bench_check_sync_policy_block_4kb(c: &mut Criterion) {
    // Same 4KB draft but with a stored policy match buried in the middle.
    // This is the policy-backed Tier 1 block latency path.
    let mut body = "Thank you for reaching out. ".repeat(74);
    body.push_str(" Reach me at 415-555-1212 if needed. ");
    body.push_str(&"Thank you for reaching out. ".repeat(75));
    let req = CheckRequest {
        workspace_id: None,
        run_id: None,
        run_event_id: None,
        run_event: None,
        session_id: None,
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
    let policy = load_str(
        r#"
id: phone-number-block
match:
  regex: "\\b\\d{3}-\\d{3}-\\d{4}\\b"
action: deny
severity: high
"#,
    )
    .expect("policy");
    let eng = Engine::new(vec![policy]);
    c.bench_function("check_sync_policy_block_4kb", |b| {
        b.iter(|| eng.check(&req));
    });
}

fn tool_policy(yaml: &str) -> FamilyPolicy {
    let AnyPolicy::Family(policy) = load_any_str(yaml).expect("tool policy") else {
        panic!("expected family policy");
    };
    policy
}

fn shell_subject(command: String) -> AuthorizationSubject {
    AuthorizationSubject::Tool {
        invocation_id: "bench-tool-use".into(),
        operation: "Bash".into(),
        tool_identity: ToolIdentity {
            server_id: "claude-code".into(),
            tool_name: "Bash".into(),
            schema_hash: "sha256:bench".into(),
        },
        parameters: serde_json::json!({
            "command": command,
            "shell": "bash",
            "cwd": "/workspace/project",
            "workspace_root": "/workspace/project",
            "run_in_background": false
        }),
        side_effect: SideEffectClass::ShellExec,
    }
}

fn fifty_tool_policies(fact_match: bool) -> Vec<FamilyPolicy> {
    (0..50)
        .map(|index| {
            if fact_match {
                tool_policy(&format!(
                    r#"
family: tool
id: bench-tool-fact-{index}
when: {{ side_effects: [shell_exec] }}
match:
  fact: {{ key: shell.risk, equals: {} }}
action: deny
reason: Bench fact policy.
"#,
                    if index == 49 {
                        "filesystem_recursive_delete"
                    } else {
                        "unmatched_bench_risk"
                    }
                ))
            } else {
                tool_policy(&format!(
                    r#"
family: tool
id: bench-tool-parameter-{index}
when: {{ side_effects: [shell_exec] }}
match:
  parameter: {{ path: /command, equals: never-match-{index} }}
action: deny
reason: Bench parameter policy.
"#
                ))
            }
        })
        .collect()
}

fn bench_shell_command_policy(c: &mut Criterion) {
    let safe_command = "printf '%s\\n' safe; ".repeat(210);
    let destructive_command = format!("{}bash -c 'rm -rf /'", "printf safe; ".repeat(310));
    let safe_parameters = ShellActionParameters {
        command: safe_command.clone(),
        shell: ShellLanguage::Bash,
        cwd: Some("/workspace/project".into()),
        workspace_root: Some("/workspace/project".into()),
        timeout_ms: None,
        run_in_background: false,
    };
    let destructive_parameters = ShellActionParameters {
        command: destructive_command,
        ..safe_parameters.clone()
    };
    c.bench_function("shell_command_parse_safe_4kb", |b| {
        b.iter(|| analyze_shell_command(&safe_parameters));
    });
    c.bench_function("shell_command_parse_nested_destructive_4kb", |b| {
        b.iter(|| analyze_shell_command(&destructive_parameters));
    });

    let parameter_policies = fifty_tool_policies(false);
    let parameter_subject = shell_subject(safe_command);
    c.bench_function("shell_command_50_parameter_policies_no_parse", |b| {
        b.iter(|| {
            evaluate_tool_policies("bench-agent", &parameter_subject, parameter_policies.iter())
                .expect("evaluate")
        });
    });

    let fact_policies = fifty_tool_policies(true);
    let fact_subject = shell_subject("rm -rf /".into());
    c.bench_function("shell_command_50_fact_policies_one_match", |b| {
        b.iter(|| {
            evaluate_tool_policies("bench-agent", &fact_subject, fact_policies.iter())
                .expect("evaluate")
        });
    });
}

criterion_group!(
    benches,
    bench_check_sync_empty,
    bench_check_async_empty_default,
    bench_check_async_50_policies_4kb,
    bench_check_sync_empty_4kb,
    bench_check_async_cache_hit,
    bench_check_sync_policy_block_4kb,
    bench_shell_command_policy,
);
criterion_main!(benches);
