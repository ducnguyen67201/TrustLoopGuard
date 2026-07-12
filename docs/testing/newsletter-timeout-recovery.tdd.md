# Newsletter timeout recovery TDD evidence

## Source and journey

No implementation plan was supplied. The journey came from captured visual evidence: as a visitor submitting the newsletter form, I need a stalled request to recover so I can retry without losing my email.

## RED and GREEN

- RED checkpoint `6ffe99b9`: `pnpm --filter marketing test` failed because the shared timeout and visual-evidence helper did not exist.
- GREEN: `pnpm --filter marketing test` passed six tests after both marketing forms adopted the shared client.
- Static/build validation: `pnpm --filter marketing typecheck` and `pnpm --filter marketing build` passed.

## Guarantees

| Guarantee | Test | Type | Result |
|---|---|---|---|
| A stalled browser request aborts instead of leaving the form in `Sending` indefinitely | `times out a stalled signup request so the form can recover` | Unit | PASS |
| Successful and rejected API responses retain their existing behavior | `completes a successful signup request`; `returns a stable error from a rejected signup response` | Unit | PASS |
| Failure evidence uses clamped viewport coordinates and contains no submitted email | `builds clamped visual evidence without including the submitted email` | Unit | PASS |
| Browser reporting emits one schema-valid `GRANNY_EVENT` line for the failed control | `emits the exact GRANNY_EVENT prefix and JSON payload once`; `reports browser evidence around the failed submit control` | Unit | PASS |

## Coverage and known gaps

`pnpm --filter marketing test:coverage` reports 100% lines, 81.25% branches, and 85.71% functions for `subscribe-client.ts`. The marketing package does not currently have a React component-test harness, so the two forms are covered through their shared behavioral client plus production typecheck/build validation.

## Concept impact

No `docs/concept/` document is affected. This is page-specific marketing-form recovery and does not change TrustLoopGuard runtime ownership, wire contracts, SDKs, or shared dashboard UI conventions.
