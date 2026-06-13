# Red-Team Dispatch

The red-team dispatch orchestrator turns a one-shot attack run into a durable,
server-owned **job**. A caller dispatches an attack against one agent endpoint and
gets a `jobId` back immediately; Rust runs the attack in the background, persists
every scored attack, and serves the job + results for polling and history.

This is the durable counterpart to the ephemeral [arena](agent-breakaway-arena.md)
demo: the arena compares a raw-vs-guarded pair and persists nothing, while a
red-team job attacks a single target and is owned by Rust like any other product
data.

## Ownership boundary

Rust owns the job and its results; the attack runner owns nothing.

- **Rust (`crates/tl-server/src/redteam`)** is the source of truth. It persists the
  job and per-attack results, drives execution through an in-process worker, and
  exposes the `/v1/redteam/*` API. Durable state lives in `crates/tl-storage`
  (`RedteamJobRepo`).
- **The attack runner (TrustLoopRed sidecar)** is a stateless executor. It is the
  only component that can run [hackagent](glossary.md#attack-generator); Rust cannot
  call hackagent directly (it is a blocking Python library with no service mode), so
  the sidecar is mandatory. The runner is reached over HTTP at `REDTEAM_RUNNER_URL`
  and is deliberately not part of the product wire contract.

The runner therefore stays outside the Rust source-of-truth boundary as an attack
harness — but unlike the arena, the durable job, its status, and its results are
Rust-owned and queryable.

## Request flow

```text
Browser (Attacks tab)
  │  POST /api/redteam/dispatch        (Next proxy: auth → SSRF allowlist → translate)
  ▼
Next API route  ──►  POST /v1/redteam/dispatch  ──►  tl-server
                                                       │ persist job (queued)
                                                       │ dispatch_tx.try_send(job) → 201 {jobId}
                                                       ▼
                                              in-process tokio worker (semaphore-bounded)
                                                       │ set_status(running)
                                                       │ POST /redteam/jobs ─► runner ─► attack(s) ─► hackagent / builtin
                                                       │ poll GET /redteam/jobs/{id} until done
                                                       │ persist per-attack results + rolled-up counts
                                                       ▼
                                              set_status(complete | error)

Browser polls GET /api/redteam/jobs/{id} → /v1/redteam/jobs/{id} → {job, results}
```

Dispatch never blocks on the runner: the handler persists the job, hands it to the
worker channel, and returns `201` with the job summary. When `REDTEAM_RUNNER_URL` is
unset (no worker) or the channel is saturated, dispatch returns `503` and the job is
marked `error` rather than stranded in `queued`.

## Job lifecycle

A job's `JobStatus` is persisted, so a restart can (in a future revision) requeue
in-flight work from storage.

```text
queued ──► running ──► complete
                  └──► error
queued/running ──► cancelled   (cooperative)
```

`complete`, `error`, and `cancelled` are terminal. Cancellation is cooperative: the
cancel endpoint marks the job `cancelled`, and the worker checks status between
runner polls (and before it starts) so a cancelled job is never revived into
`running` and its results are not overwritten. There is no hard-kill of an attack
already executing inside the runner.

## API

All routes are workspace-scoped and authenticated like the rest of `/v1/*`.

| Method & path | Purpose |
|---|---|
| `POST /v1/redteam/dispatch` | Create a job (`queued`) and hand it to the worker; returns `RedteamJobSummary` (`201`), or `503` when dispatch is unavailable |
| `GET /v1/redteam/jobs` | List workspace jobs, newest first (`agent_id`, `limit` filters) |
| `GET /v1/redteam/jobs/{id}` | A job plus its per-attack results (`RedteamJobDetail`) |
| `GET /v1/redteam/jobs/{id}/results` | Per-attack results only |
| `GET /v1/redteam/attacks` | Every attack result across all jobs in the workspace, flattened with parent-job context (target, profile, created_at), newest job first (`attack`, `outcome`, `limit` filters); returns `RedteamAttackRecordListResponse` |
| `POST /v1/redteam/jobs/{id}/cancel` | Cooperatively cancel; returns the updated (or unchanged terminal) summary |

Wire types live in `crates/tl-core/src/redteam.rs` (`JobStatus`, `RedteamGenerator`,
`RedteamDispatchRequest`, `RedteamJobSummary`, `RedteamJobResult`, `RedteamJobDetail`,
`RedteamAttackRecord`, and the list responses) and are reflected in `docs/openapi.yaml`.

A completed job can be turned into a shareable vulnerability report (single-run or a
same-agent before/after comparison) via `/v1/redteam/jobs/{id}/report` and the public
share endpoints — see [Red-Team Report Sharing](redteam-report-sharing.md).

## Storage

Two workspace-scoped tables in `crates/tl-storage`:

- `redteam_jobs (workspace_id, id)` — one row per dispatched job: target, profile,
  generator, status, rolled-up `attacks` / `landed` / `blocked` counts, optional
  `error`, and timestamps.
- `redteam_job_results (workspace_id, job_id, seq)` — one row per scored attack:
  attack name, goal, outcome, `landed`, prompt, reply, and optional `trace_id`.

The orchestrator writes results and counts; it does not re-score. Scoring is the
runner's job — Rust copies the verdict verbatim.

## The in-process worker

v1 uses an in-process `tokio` mpsc channel and a bounded worker (`spawn_dispatch_worker`)
rather than a durable queue. A semaphore caps concurrent jobs because hackagent runs
are heavy; excess jobs wait in the channel. The worker never panics: any failure
marks the job `error` with a message.

The one deferred seam is the channel itself: replacing it with a durable queue plus
requeue-on-boot is the planned next step, and `JobStatus` is already persisted to
make that a clean addition. The seam is marked at the `spawn_dispatch_worker`
definition in code.

## Generators

`RedteamGenerator` selects how the runner crafts attacks:

- `deterministic` (default) — the runner's built-in attack catalogue. No external
  engine, no LLM. This is the validated path.
- `hackagent` — hackagent-generated adversarial cases. This path is unvalidated end
  to end and is wired behind an explicit opt-in; the runner falls back to the
  deterministic catalogue when the toolkit or its LLM is unreachable, so a hackagent
  request never dead-ends a job.

## Hardening loop

A finished job's results feed the same suggest → apply → re-run loop the arena uses
([Hardening Loop](agent-breakaway-arena.md#hardening-loop)). The dashboard turns the
attacks that landed on the guard into a guard policy, applies it through the
Rust-owned `/v1/policies` API, and re-dispatches the same target — successive jobs in
the history show the gap closing. The applied policy is durable product data owned by
Rust exactly like a hand-authored one.

## Configuration

- `REDTEAM_RUNNER_URL` — base URL of the attack runner. When unset, the dispatch
  worker is not spawned and `POST /v1/redteam/dispatch` returns `503`. The rest of the
  server runs normally.
