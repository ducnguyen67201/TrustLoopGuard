# Dispute And Money Agent

## What It Teaches

The `dispute/` directory exposes three views of the same rule: a money-moving
tool must execute only after TrustLoopGuard returns `permit`.

1. **Scenario matrix:** permit, deny, defer, and require-approval money actions.
2. **Bring your own agent:** the smallest explicit `submitEvent()` gate around a
   payment callback.
3. **NorthPay targets:** the same refund agent exposed raw and guarded for the
   Attacks page.

## Fastest Checks

```bash
pnpm --filter @trustloopguard/demo dispute:check
pnpm --filter @trustloopguard/demo dispute:scenarios:check
```

The first checks the local NorthPay parser and refund ledger. The second checks
the five-outcome money matrix without a server, model, or payment provider.

## Run The Guarded Matrix

```bash
make server
TL_USER_ID=<owner-uuid> pnpm --filter @trustloopguard/demo dispute:setup
pnpm --filter @trustloopguard/demo dispute:scenarios
```

No Stripe key means simulated payments. Only an explicit `sk_test_...` key can
enable Stripe sandbox payments; live keys are refused.

Run `dispute:byo` for the copyable execution gate or `dispute:serve:doppler`
for raw and guarded Attacks targets.

## Expected Proof

- A legitimate refund executes.
- Amount cap and injected-destination attempts are denied.
- An unverifiable amount is deferred.
- A wire transfer requires approval.
- Every non-permit outcome produces zero provider calls.

## Read The Code

- [`scenarios.core.ts`](../../demo/dispute/scenarios.core.ts) builds and checks
  the scenario events.
- [`byo.example.ts`](../../demo/dispute/byo.example.ts) is the minimal explicit
  integration.
- [`serve.ts`](../../demo/dispute/serve.ts) creates the raw and guarded targets.
- [`demo/README.md`](../../demo/README.md#money-agent--guarded-scenarios-flagship)
  owns the full setup.
