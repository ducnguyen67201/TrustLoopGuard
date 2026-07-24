# Stripe Refund Agent

## What It Teaches

Connect typed TrustLoopGuard financial authorization to a realistic customer
backend and payment-provider boundary:

```text
support prompt -> search SQLite order evidence
               -> prepare typed refund action in Rust
               -> permit / hold / deny
               -> vaulted payment_http execution only when authorized
               -> receipt and outcome
```

The agent process does not hold the Stripe key. The provider adapter does, and
it refuses live Stripe keys.

## Fastest Check

```bash
pnpm --filter @trustloopguard/demo stripe-refund-agent:check
```

For the broader public demo contract:

```bash
pnpm test:refund-demo
```

## Run It

Start the local stack, initialize SQLite, and provision the Rust workspace:

```bash
make local
pnpm --filter @trustloopguard/demo stripe-refund-agent:db
pnpm --filter @trustloopguard/demo stripe-refund-agent:setup
```

Run `pnpm run dev` from `demo/`, then open `http://127.0.0.1:9310`.
Use the dedicated Doppler-backed `stripe-refund-agent:live` path when testing a
real Stripe sandbox.

## Expected Proof

- A `$25` refund executes automatically.
- A `$75` refund is held for approval.
- A `$125` refund is blocked.
- Provider execution never occurs before Rust authorization.
- Internal order, payment, and provider credentials stay server-side.

## Read The Code

- [`scripted-agent.ts`](../../demo/stripe-refund-agent/scripted-agent.ts) is the
  easiest end-to-end flow to read.
- [`tool-runner.ts`](../../demo/stripe-refund-agent/tool-runner.ts) owns tools.
- [`core.ts`](../../demo/stripe-refund-agent/core.ts) owns financial actions.
- [`order-db.ts`](../../demo/stripe-refund-agent/order-db.ts) owns demo customer
  state.
- [`demo/stripe-refund-agent/README.md`](../../demo/stripe-refund-agent/README.md)
  owns full local and deployment setup.
