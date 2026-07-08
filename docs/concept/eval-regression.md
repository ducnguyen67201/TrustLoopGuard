# Eval regression cases

TrustLoopGuard's evolving eval loop starts with a durable regression-case
ledger. A case is a red-team finding that has already produced a verified
guardrail candidate and should be re-run when policies, tool metadata, agent
profiles, or checker modes change.

## Current scope

`POST /v1/redteam/jobs/{id}/harden` accepts two independent write flags:

- `persist: true` saves verified guardrail survivors disabled for operator opt
  in.
- `promote_regression: true` upserts verified survivors into
  `redteam_regression_cases`.

Promotion is explicit because it is durable state. The case key is stable across
re-hardening: source job id, substrate, artifact id, and evidence seqs. Repeating
the same promotion refreshes the existing row instead of creating duplicates.

`GET /v1/redteam/regressions` lists the ledger newest-first, with optional
`agent_id` and `source_job_id` filtering. Each case stores the expected guarded
outcome (`block`, `escalate`, or `stop`), source job id, evidence seqs,
substrate, artifact id, attack, and goal. The full captured trace remains in
red-team session storage.

`POST /v1/redteam/regressions/run` starts a new red-team job from promoted
cases. The request must name `source_job_id`; optional `case_keys` narrows the
run to specific ledger rows, and optional `limit` caps the selected rows. Rust
loads the source job, copies its target, agent, environment, and profile, maps
each selected case into an `AttackVector`, and sends it through the same runner
dispatch queue as normal red-team jobs. The response returns the new
`RedteamJobSummary`, selected case count, and selected case keys.

`GET /v1/redteam/regressions/results/{job_id}` summarizes a completed
regression job against cases promoted from `source_job_id`. The summary is
computed from durable case rows plus durable red-team sessions. Each case is
marked `passed`, `failed`, `missing`, or `inconclusive`; missing sessions are
explicit so skipped cases cannot look like passes. The endpoint accepts the same
case-key filters as the run endpoint for CI-style scoped checks.

Every result summary also upserts a durable
`redteam_regression_result_snapshots` row keyed by regression job, source job,
and selected case keys. `GET /v1/redteam/regressions/results` lists those
snapshots newest-first, with optional `source_job_id`, `job_id`, `agent_id`, and
`limit` filters. The first trend record stores the pass/fail/missing/
inconclusive counts and the selected case-key scope; repeated reads of the same
summary refresh the same row instead of duplicating history.

`tl redteam regressions check <job_id> --source-job-id <source_job_id>` is the
CI gate over that summary contract. It fetches
`GET /v1/redteam/regressions/results/{job_id}`, prints the pass/fail/missing/
inconclusive counts, and exits non-zero when counts exceed `--max-failed`,
`--max-missing`, or `--max-inconclusive` thresholds. Repeated `--case-key`
filters and `--limit` let CI run scoped suites. The same read also refreshes the
durable result snapshot, so dashboard/history views see the latest CI result.

`tl redteam regressions history` lists durable result snapshots through
`GET /v1/redteam/regressions/results`, with optional source job, regression job,
agent, and limit filters.

The attacks dashboard includes a compact regression suite panel over the same
contracts. It lists promoted cases, dispatches a regression run for the selected
source job, checks the latest result snapshot, and shows pass/fail/missing/
inconclusive counts.

## Still planned

The first historical rollup, CI gate, and dashboard panel are shipped. The next
layer should add latency, false-block, and LLM-call metrics over the result
summary/history contract.
