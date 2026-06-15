# Red-Team Dispatch

Red-team dispatch turns a one-shot attack run into a durable, Rust-owned **job**.
A caller dispatches one target endpoint and receives a job id immediately; Rust
runs the work in the background, persists every scored result, and serves job
history for polling, reports, and dashboard views.

This is the durable counterpart to the ephemeral [arena](agent-breakaway-arena.md)
demo. The arena is a local comparison harness; red-team dispatch is product data
owned by Rust. TrustLoopGuardBench reuses red-team jobs as child execution
records; its parent concept is described in
[TrustLoopGuardBench](trustloopguard-bench.md).

## Ownership boundary

Rust owns the job and its results. The runner owns no durable product state.

- **Rust (`crates/tl-server/src/redteam`)** is the source of truth. It validates
  dispatch input, persists the job and per-attack results, drives execution
  through an in-process worker, and exposes `/v1/redteam/*`. Durable state lives
  in `crates/tl-storage` (`RedteamJobRepo`).
- **A compatible private runner** is a stateless executor. Rust reaches it over
  HTTP at `REDTEAM_RUNNER_URL`, sends only the validated target/profile payload,
  polls until completion, and copies scored results into Rust storage.

The runner stays outside the Rust source-of-truth boundary. It may keep
transient in-memory job state while a run is active, but it must not persist
policies, traces, decisions, jobs, reports, or workspace data.

## Request flow

```text
Browser (Attacks tab)
  | POST /api/redteam/dispatch
  |   Next proxy: auth -> loopback allowlist -> snake_case body
  v
Next API route -> POST /v1/redteam/dispatch -> tl-server
                                                   | persist job (queued)
                                                   | dispatch_tx.try_send(job)
                                                   | return 201 job summary
                                                   v
                                      in-process tokio worker
                                                   | set_status(running)
                                                   | POST /redteam/jobs -> private runner
                                                   | poll GET /redteam/jobs/{id}
                                                   | persist results + counts
                                                   v
                                      set_status(complete | error)

Browser polls GET /api/redteam/jobs/{id}
  -> GET /v1/redteam/jobs/{id}
  -> { job, results }
```

Dispatch never blocks on runner execution. When `REDTEAM_RUNNER_URL` is unset or
invalid, the worker is not spawned and dispatch returns `503` with:

```text
red-team execution is not configured for this deployment; contact TrustLoopGuard
to enable managed or enterprise execution
```

When the worker queue is saturated or closed, dispatch also returns `503` and the
job is marked `error` rather than stranded in `queued`.

## Job lifecycle

A job's `JobStatus` is persisted, so a future durable queue can requeue in-flight
work from storage.

```text
queued -> running -> complete
               \-> error
queued/running -> cancelled
```

`complete`, `error`, and `cancelled` are terminal. Cancellation is cooperative:
the cancel endpoint marks the job `cancelled`, and the worker checks status
before starting and between runner polls.

## API

All routes are workspace-scoped and authenticated like the rest of `/v1/*`.

| Method and path | Purpose |
|---|---|
| `POST /v1/redteam/dispatch` | Create a job (`queued`) and hand it to the worker; returns `RedteamJobSummary` (`201`) or `503` when dispatch is unavailable |
| `GET /v1/redteam/jobs` | List workspace jobs, newest first (`agent_id`, `limit` filters) |
| `GET /v1/redteam/jobs/{id}` | Return one job plus its per-attack results |
| `GET /v1/redteam/jobs/{id}/results` | Return per-attack results only |
| `GET /v1/redteam/attacks` | Return flattened attack records across jobs in the workspace |
| `POST /v1/redteam/jobs/{id}/cancel` | Cooperatively cancel a queued/running job |

Wire types live in `crates/tl-core/src/redteam.rs` and are reflected in
`docs/openapi.yaml`, the generated TypeScript SDK, and the generated Python SDK.
The public dispatch request intentionally exposes no runner engine selector.

A completed job can be turned into a shareable vulnerability report (single-run
or same-agent before/after comparison) via `/v1/redteam/jobs/{id}/report` and
the public share endpoints. See
[Red-Team Report Sharing](redteam-report-sharing.md).

## Storage

Two workspace-scoped tables in `crates/tl-storage` own red-team dispatch data:

- `redteam_jobs (workspace_id, id)` - one row per dispatched job: target,
  profile, status, rolled-up `attacks` / `landed` / `blocked` counts, optional
  `agent_id`, optional `error`, and timestamps. The legacy `generator` column is
  internal compatibility metadata and is not part of the public wire contract.
- `redteam_job_results (workspace_id, job_id, seq)` - one row per scored attack:
  attack name, goal, outcome, `landed`, prompt, reply, optional `trace_id`, and
  optional benchmark comparison metadata (`case_id`, `track`, `kind`,
  `trial_index`).

The orchestrator writes results and counts. It does not re-score runner output.

## Runner contract

`REDTEAM_RUNNER_URL` points to a compatible private runner that implements:

- `GET /health`
- `POST /redteam/jobs`
- `GET /redteam/jobs/{id}`

The contract is documented in
[`docs/contracts/redteam-runner-v1.md`](../contracts/redteam-runner-v1.md), with
neutral fixtures under `docs/contracts/fixtures/redteam-runner/`.

The server-to-runner payload uses camelCase and contains only non-secret
dispatch fields such as `targetUrl` and `profile`. Browser code never receives
the runner URL and never calls the runner directly.

## The in-process worker

v1 uses an in-process `tokio` mpsc channel and bounded worker
(`spawn_dispatch_worker`) rather than a durable queue. A semaphore caps
concurrent jobs; excess jobs wait in the channel. The worker never panics: any
failure marks the job `error` with a message.

The deferred seam is the channel itself. Replacing it with a durable queue plus
requeue-on-boot is a future change; `JobStatus` is already persisted to make
that addition straightforward.

## Hardening loop

A finished job's results feed the same suggest -> apply -> re-run loop the arena
uses ([Hardening Loop](agent-breakaway-arena.md#hardening-loop)). The dashboard
turns landed attacks into candidate guard policies, Rust verifies them, and any
survivors are persisted through Rust-owned policy APIs. See
[Red-Team Hardening](redteam-harden.md).

## Configuration

- `REDTEAM_RUNNER_URL` - optional server-only base URL for a compatible private
  runner. When unset, `POST /v1/redteam/dispatch` returns the gated `503`
  message above and the rest of the server continues normally.
