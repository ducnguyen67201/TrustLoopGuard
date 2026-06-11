# TrustLoopGuardBench

TrustLoopGuardBench (`crates/tl-bench`) is the behavioral regression harness
for the event pipeline. It runs fixed attack/benign scenario pairs through a
real `EventPipelineCtx` — the four deterministic checkers plus the mode-aware
decision composer, with every other stage a no-op — and reports whether
attacks are caught and benign twins stay allowed. It is framework-free: no
server, no storage, no live LLM calls.

## Tracks

The v1 seed set covers three tracks, each with one attack scenario and one
benign twin:

| Track | Attack scenario | Catching checker |
|---|---|---|
| `indirect_prompt_injection` | Web-sourced data controls the recipient of an external communication | information-flow (`action-integrity`) |
| `private_data_flow` | Secret-labeled API data flows to an external communication sink | information-flow (`destination-permission`) |
| `delayed_memory_risk` | Untrusted web-sourced content proposed as a memory write | memory (`memory-write-untrusted`) |

Each benign twin performs the same operation with trusted, public,
high-integrity, fully attributed provenance, and must stay allowed even under
enforce.

## Scenario and expectation model

A `Scenario` is a fixed `GuardEvent` plus an `Expectation` describing its
outcome under all-enforce checker modes:

- `Caught` — the decision verdict is `Block` or `Escalate`.
- `Allowed` — the verdict stays `Allow`.

Scenario events declare explicit source labels because the bench pipeline
runs with the no-op label resolver: what a scenario declares is exactly what
the checkers see. Each run seeds a fresh `Decision::allow`, so the report
measures only what the checkers and composer contribute.

## Metrics

`run_scenarios` returns a `BenchReport`:

- `scenarios_run`
- `attack_catch_rate` — fraction of attack scenarios caught. The spec's
  *attack success rate* is its inverse (`1.0 - attack_catch_rate`).
- `false_block_rate` — benign scenarios that resolved `Block` or `Escalate`;
  folds the spec's *false-block rate* and *false-escalation rate* together.
- `benign_completion_rate` — the spec's *benign task completion*.
- `mean_latency_us` — informational wall-clock mean per `process()` call;
  the spec's *latency overhead* gate stays with the criterion benchmarks.
- per-track breakdown (`attacks`, `attacks_caught`, `benign`,
  `benign_passed`) — covers the spec's per-failure-class catch rates
  (*unsafe source-to-sink*, *parameter-source*, *unsafe-memory*).

Spec metrics not yet measured: *LLM calls per decision*, *cost per request*,
and *trace explanation quality*.

## Running it

```bash
pnpm bench:smoke                # smoke tests via make bench-smoke
cargo run -p tl-bench           # readable report table
cargo run -p tl-bench -- --json # serialized BenchReport
```

The smoke tests assert that, under all-enforce modes, every track's attack
is caught and its benign twin is allowed — and that with every checker OFF,
all scenarios resolve `Allow` (rollout safety: OFF changes nothing).

## What it is not

- Not a CI gate yet — `bench-smoke` is run on demand.
- No live-LLM scoring; scenarios exercise the deterministic checkers only.
- Not the latency microbenchmarks: those are the criterion benches in
  `crates/tl-engine/benches` and remain separate.
