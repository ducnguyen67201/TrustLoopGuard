# TDD Evidence: Flagship Money Agent Demo

## Source Plan

Local plan source:
`.claude/PRPs/plans/flagship-money-agent-demo.plan.md`

## User Journey

As a founder demoing TrustLoopGuard, I want a two-minute money-moving agent
showcase, so that investors and design partners immediately understand that
unsafe money movement is stopped before the side effect fires.

## RED / GREEN Summary

| Stage | Command | Result | Evidence |
|---|---|---|---|
| RED | `pnpm --filter @trustloopguard/demo dispute:scenarios:check` | Failed | New test required `formatScenarioTranscript`, which was not exported by `scenarios.core.ts`. |
| GREEN | `pnpm --filter @trustloopguard/demo dispute:scenarios:check` | Passed | Scenario rows now include trace/reason evidence and the transcript formatter emits the demo summary. |

## Test Specification

| # | What is guaranteed | Test file or command | Test type | Result |
|---|---|---|---|---|
| 1 | A payment fires only when the guard returns `allow`. | `demo/dispute/scenarios.check.ts` | Unit/smoke | PASS |
| 2 | Scenario rows carry `traceId` and `reason` evidence from the decision. | `demo/dispute/scenarios.check.ts` | Unit/smoke | PASS |
| 3 | The transcript includes the demo title, payment summary, stopped-action summary, trace ids, and reasons. | `demo/dispute/scenarios.check.ts` | Unit/smoke | PASS |
| 4 | The existing NorthPay dispute demo still works. | `pnpm --filter @trustloopguard/demo dispute:check` | Smoke | PASS |
| 5 | The demo package type-checks. | `pnpm --filter @trustloopguard/demo typecheck` | Static | PASS |

## Validation Results

```text
pnpm --filter @trustloopguard/demo dispute:scenarios:check
scenarios check: all assertions passed

pnpm --filter @trustloopguard/demo dispute:check
dispute demo check: all assertions passed

pnpm --filter @trustloopguard/demo typecheck
tsc --noEmit
```

## Coverage and Known Gaps

This branch uses the existing demo smoke-test style instead of package-level
coverage because `@trustloopguard/demo` does not define a coverage script. The
side-effect gate, evidence fields, and transcript output are covered by the
offline check; live server and Stripe test-mode validation remain manual because
they require local server credentials and optional Stripe test credentials.
