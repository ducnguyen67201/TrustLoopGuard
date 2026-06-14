# TrustLoopGuardBench

TrustLoopGuardBench is the product benchmark for proving the protection delta
between the same agent running raw and running behind TrustLoopGuard. It has two
layers:

- `crates/tl-bench` is the deterministic CI harness. It runs fixed seed cases
  through the real event pipeline without server, storage, live LLMs, or runner
  side effects.
- `/v1/bench/*` is the durable agent-in-the-loop benchmark API. Rust persists a
  parent benchmark run, creates raw and guarded child red-team jobs, and derives
  the report from their scored results.

The benchmark measures outcomes at the sink whenever possible. A blocked attack
is not enough by itself: reports show attack success, benign utility, utility
under attack, false-block rate, and per-track deltas together so an overblocking
guard cannot look good by destroying useful task completion.

## Deterministic Harness

The deterministic harness in `crates/tl-bench` runs the same scenario catalogue
under two checker configurations:

| Arm | Checker modes | Purpose |
|---|---|---|
| `unguarded` | all checkers `off` | Baseline: what lands when TrustLoopGuard is not enforcing |
| `guarded` | all checkers `enforce` | Protected behavior under the deterministic checkers |

Each scenario carries a stable case id, track, kind, and simulated outcome
predicate. The harness reports both arms plus deltas:

- ASR — attack success rate, excluding benign controls from the attack
  denominator.
- BU — benign utility, the share of benign tasks still completed.
- UA — utility under attack, the share of legitimate task completion preserved
  when adversarial content is present.
- False-block rate — benign work blocked or escalated by the guard.
- Per-track breakdown, including the memory track's injection/retrieval/use
  metrics.

Run it with:

```bash
pnpm bench:smoke
cargo run -p tl-bench -- --json
```

## Durable Benchmark Runs

Durable benchmark runs are Rust-owned product data. Browser code calls
`/api/bench/*` same-origin Next routes; those routes validate UI-shaped input,
apply the loopback target allowlist, and proxy to Rust. They do not aggregate
results or store benchmark state.

```text
Browser
  -> Next /api/bench/runs
    -> Rust POST /v1/bench/runs
      -> bench_runs parent row
      -> raw child redteam job
      -> guarded child redteam job
      -> bench_run_arms maps each arm to its child job
      -> existing red-team worker executes both children
      -> GET /v1/bench/runs/{id} refreshes parent status from child statuses
      -> GET /v1/bench/runs/{id}/report derives the raw-vs-guarded report
```

The parent run is the product concept. Child red-team jobs are execution
evidence and remain available through `/v1/redteam/*`.

## API

All routes are workspace-scoped and authenticated like the rest of `/v1/*`.

| Method & path | Purpose |
|---|---|
| `POST /v1/bench/runs` | Create a parent run, create raw and guarded child jobs, attach arms, and queue both children |
| `GET /v1/bench/runs` | List parent runs, newest first |
| `GET /v1/bench/runs/{id}` | Read parent run and arms; refresh parent status from child jobs |
| `GET /v1/bench/runs/{id}/report` | Return the Rust-derived ASR/BU/UA/delta report for a completed run |
| `POST /v1/bench/runs/{id}/cancel` | Cancel active raw/guarded child jobs and mark the parent cancelled; terminal status writes are not revived |

Wire types live in `crates/tl-core/src/bench.rs` and are reflected in
`docs/openapi.yaml`, the generated TypeScript types, and the generated Python
types.

## Storage

Benchmark state lives in `crates/tl-storage`:

- `bench_runs (workspace_id, id)` — parent run identity, status, profile,
  generator, optional agent/seed metadata, error, and timestamps.
- `bench_run_arms (workspace_id, run_id, arm)` — raw/guarded arm target,
  checker configuration label, and optional child `redteam_job_id`.

Per-attack evidence is not duplicated into benchmark tables. Reports load child
results from `redteam_job_results` and compare cases by `case_id` when present,
falling back to the legacy `(seq, attack, goal)` identity for older runner
output.

## Boundaries

- The dashboard must not create web or Drizzle tables for benchmark runs,
  arms, results, or reports.
- The dashboard must not compute ASR/BU/UA or per-case comparison; Rust returns
  `BenchReportPayload`.
- Benchmark target URLs from the dashboard remain loopback-only.
- The durable benchmark reuses the red-team runner; it is not a second attack
  executor.
