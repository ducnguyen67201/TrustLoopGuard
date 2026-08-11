# Post-Run Evaluations

Post-run evaluation turns the evidence from one completed [Run](runs.md) into an immutable,
auditable result for each participating agent. A Run is the only evaluation boundary. A
conversation, framework session, OTel trace, or provider request may help an adapter decide when to
finalize a Run, but none of them starts evaluation directly.

## Ownership

Rust owns the complete pipeline:

```text
terminal Run finalization
  -> capture barrier closes
  -> immutable RunSnapshot
  -> durable evaluation job lease
  -> per-agent result and findings
  -> read-only release-gate evidence
```

`tl-core` owns wire contracts, `tl-policy` owns `family: evaluation` parsing, `tl-eval` owns pure
grading and aggregation, `tl-storage` owns snapshots/jobs/results, and `tl-server` owns APIs and
worker orchestration. The dashboard only uses same-origin proxies and renders Rust responses.

## Agent Profiles and Policy Assignments

An environment-scoped agent evaluation profile controls whether evaluation is enabled, whether
decision evidence is written in `best_effort` or `durable` mode, how content is minimized, capture
quiet/deadline timing, and whether incomplete evidence fails or is inconclusive.

Evaluation policy assignments are distinct from policy ownership and runtime deployment:

- `policies.owner_agent_id` is lifecycle ownership.
- `policy_environment_deployments` controls synchronous runtime enforcement.
- `agent_evaluation_policy_assignments` controls post-run evaluation.

When an agent first participates in a Run, Rust resolves enabled assignments to concrete policy
versions and freezes their YAML, hash, weight, and critical flag in the Run manifest. Later policy
edits cannot change that Run's evidence contract. The first resolution freezes the manifest even
when no policies are assigned, so later assignments cannot apply retroactively.

Gateway production activation enables a durable profile and additively assigns two deterministic
starter policies: intervention count and terminal provider failure count. Existing assignments are
preserved. A retry followed by success is not a terminal provider failure; only an exhausted
provider plan on a failed Run increments that metric. An empty manifest remains `not_configured`
and never satisfies production readiness.

Runs created before this pipeline was installed are returned with
`evaluation_eligibility: legacy_incomplete`. They retain their historical evidence but do not
receive a retroactive manifest. Runs created after the migration are marked `eligible`.

## Capture and Snapshot Boundary

Finalization and evidence completeness are separate states. `POST /v1/runs/{id}/finalize` records
the terminal execution boundary and arms capture. The barrier closes after either the expected
durable telemetry flush receipt arrives, a configured quiet period passes, or the maximum wait
expires. A flush receipt satisfies the barrier only when its whole accepted batch is committed by
the deadline; rejected correlated spans leave capture incomplete. Expiry or dropped required
evidence produces an `incomplete` snapshot.

Snapshots contain a bounded, privacy-normalized projection of events, decision traces, spans,
metrics, policy trigger counts, participant manifests, and evidence identifiers. Each snapshot and
manifest has a canonical BLAKE3 content address. A manual re-evaluation creates another snapshot
version; previous snapshots and results remain immutable. Evidence arriving after a closed snapshot
is retained as `late_evidence` and does not rewrite history.

## Jobs, Graders, and Verdicts

Postgres is the work queue. Workers claim due jobs with `FOR UPDATE SKIP LOCKED`, increment attempts,
and set an expiring lease before evaluating. An abandoned lease becomes claimable after expiry.
The idempotency identity includes workspace, Run, agent, snapshot hash, manifest hash, and evaluator
version, so retries cannot create a second result for the same evidence.

Built-in graders cover runtime-policy observations and bounded integer Run metrics. Policy replay
freezes referenced deterministic content-policy YAML, version, and hash in the Run manifest, then
replays verified redacted `GuardEvent` evidence through the existing engine adapter. Semantic and
unsupported policy families are rejected at assignment time rather than evaluated against a mutable
live registry. Rubric policies are prefiltered and submitted in one structured batch for a Run/agent;
there is never one LLM call per policy. Missing adapters, malformed outputs, skipped checks, and
timeouts remain explicit findings.

When a rubric batch is attempted, its route, provider/model, outcome, token counts, fallback flag,
latency, and error code are stored with the immutable evaluation result as guardrail usage. Captured
content is never included in that audit object.

Aggregation uses integer basis points and stable policy ordering. Any critical failure fails the
agent result. Failed, inconclusive, errored, and not-applicable findings remain visible. Incomplete
required evidence follows the stricter profile/policy behavior frozen into the snapshot and can
never produce `passed`.
For a multi-agent Run, each participant receives its own manifest and result; consumers project the
worst participant verdict when they need one Run-level status.

Failed, inconclusive, and errored results can enqueue durable email deliveries after persistence.
The evaluator does not send network requests in its database transaction; delivery ownership,
deduplication, leasing, and retries are defined in [notifications.md](notifications.md).

## Golden Datasets and Release Gates

Golden datasets are immutable, content-addressed manifests of cases. Cases declare trajectory or
end-state scoring, weights, criticality, reference hashes, and hard budgets for turns, tool calls,
tokens, and duration. Featherlane does not execute customer agent code: a customer's sandbox runner
submits the completed Run associated with a case, and the same snapshot evaluator scores it.

A campaign maps dataset cases to completed Runs. Missing, skipped, budget-exceeded, and errored cases
are explicit evidence, never implicit zero-score passes. A release gate is a read-only projection over
immutable campaign or recent-Run results for one agent, environment, and required manifest hash. Its
verdict is `passed`, `failed`, or `insufficient_evidence`; it never deploys or mutates customer code.

## HTTP Surface

- `GET/PUT /v1/agents/{id}/evaluation-profile`
- `GET/PUT /v1/agents/{id}/evaluation-policy-assignments`
- `GET /v1/runs/{id}/evaluations`
- `POST /v1/runs/{id}/evaluations` to request a new snapshot version

The read response includes durable job state as well as result history, so clients can distinguish
waiting, queued, running, completed, and error states. Re-evaluation may target selected participant
agents; it creates a new evidence snapshot while preserving the Run's already frozen policy
manifest and all prior snapshots and results.

All mutations use authenticated workspace/environment context. Durable evaluation state is never
owned by Next.js or a web database.
