# Financial Refund

## What It Teaches

Exercise the typed financial-authorization lifecycle without Rust, a model, or
a payment provider. A mock SDK-shaped client creates a grant, prepares refund
actions, applies caps and approvals, executes permitted actions, exports
receipts, records outcomes, and enforces idempotency.

This is the best demo for understanding financial states before connecting a
real provider.

## Run It

```bash
pnpm --filter @trustloopguard/demo financial-refund
```

The command prints the outcome table for all scenarios.

## Fastest Check

```bash
pnpm --filter @trustloopguard/demo financial-refund:check
```

## Expected Proof

| Scenario | Final result | Provider calls |
| --- | --- | --- |
| Refund below threshold | Executed | 1 |
| Held refund approved | Executed | 1 |
| Held refund denied | Denied | 0 |
| Duplicate retry | Executed once | 1 total |
| Missing grant | Denied | 0 |

## Read The Code

- [`core.ts`](../../demo/financial-refund/core.ts) owns the authorization flow.
- [`mock-client.ts`](../../demo/financial-refund/mock-client.ts) models the
  SDK-shaped financial backend.
- [`scenarios.ts`](../../demo/financial-refund/scenarios.ts) prints the demo.
- [`financial-refund.check.ts`](../../demo/financial-refund/financial-refund.check.ts)
  asserts the offline contract.
