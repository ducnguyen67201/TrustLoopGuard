# TDD Evidence: Trace Replay Export

## Source Plan

Local plan source:
`.claude/PRPs/plans/trace-replay-export.plan.md`

## User Journey

As a security or operations reviewer, I want to open a trace and inspect the
stored decision evidence, so that I can prove why an agent action was allowed,
blocked, rewritten, or escalated without re-running policy logic.

## RED / GREEN Summary

| Stage | Command | Result | Evidence |
|---|---|---|---|
| RED | `cargo test -p tl-server --test traces` | Failed | The trace detail route was not registered, so point lookup returned the router 404 instead of a trace/API error envelope. |
| RED | `pnpm --filter web test TraceDetailPageContent` | Failed after dependency install | The trace detail component did not yet render the expected evidence/export UI. |
| GREEN | `cargo test -p tl-server traces` | Passed | Trace lookup returns scoped traces and a typed not-found response. |
| GREEN | `pnpm --filter web test TraceDetailPageContent` | Passed | The detail page renders action parameters, sources, provenance, policy/check evidence, and raw JSON export. |

## Test Specification

| # | What is guaranteed | Test file or command | Test type | Result |
|---|---|---|---|---|
| 1 | `GET /v1/traces/{trace_id}` fetches a trace through the Rust store using resolved workspace/environment. | `crates/tl-server/tests/traces.rs` | Backend integration | PASS |
| 2 | Unknown or other-workspace traces return `not_found`. | `crates/tl-server/tests/traces.rs` | Backend integration | PASS |
| 3 | The trace detail UI renders decision reason, action parameters, sources, provenance, policy evidence, checker evidence, and raw JSON export. | `apps/web/components/workspace/TraceDetailPageContent.test.tsx` | Component | PASS |
| 4 | Old/minimal trace payloads render empty states instead of crashing. | `apps/web/components/workspace/TraceDetailPageContent.test.tsx` | Component | PASS |
| 5 | Review queue rows link to trace detail. | `apps/web/components/workspace/ReviewQueueContent.test.tsx` | Component | PASS |

## Validation Results

```text
cargo test -p tl-server --test traces
pnpm --filter web test TraceDetailPageContent
pnpm --filter web test ReviewQueueContent
pnpm --filter web typecheck
cargo fmt --all -- --check
pnpm codegen:check
```

## Coverage and Known Gaps

This branch renders stored evidence only; it intentionally does not re-run
policies, create PDF exports, or add public trace sharing. Manual browser
validation was not performed in the worker pass, so the PR relies on component
tests plus server route tests for automated proof.
