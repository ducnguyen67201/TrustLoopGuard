# E-commerce refund pilot TDD report

Source plan: `/Users/ducng/Desktop/workspace/Umbrella/TrustLoopGuard/.claude/PRPs/plans/ecommerce-refund-pilot.plan.md`

## User journey

As an e-commerce AI support team, I want to test TrustLoopGuard on refund
actions, so that I can see over-cap, injected-destination, and ambiguous
refunds caught before money or store credit moves.

## RED evidence

Command:

```sh
pnpm --filter @trustloopguard/demo ecommerce:check
```

Result: failed before implementation because `demo/ecommerce/refund.core.ts`
did not exist.

Excerpt:

```text
Error [ERR_MODULE_NOT_FOUND]: Cannot find module .../demo/ecommerce/refund.core
```

Typecheck also failed on the same missing module:

```sh
pnpm --filter @trustloopguard/demo typecheck
```

## GREEN evidence

Command:

```sh
pnpm --filter @trustloopguard/demo ecommerce:check
```

Result:

```text
ecommerce refund check: all assertions passed
```

Command:

```sh
pnpm --filter @trustloopguard/demo typecheck
```

Result: `tsc --noEmit` passed.

## Guarantees

| # | What is guaranteed | Test file or command | Type | Result |
| --- | --- | --- | --- | --- |
| 1 | The pilot has five focused scenarios with one intended control each. | `demo/ecommerce/refund.check.ts` | unit | PASS |
| 2 | Events use the `/v1/events` `GuardEvent` shape with e-commerce context. | `demo/ecommerce/refund.check.ts` | unit | PASS |
| 3 | Trusted destinations come from `order_registry`; injected destinations come from `conversation`. | `demo/ecommerce/refund.check.ts` | unit | PASS |
| 4 | Amounts are integer cents except the ambiguous non-integer scenario. | `demo/ecommerce/refund.check.ts` | unit | PASS |
| 5 | Simulated refund/store-credit side effects write only when verdict is `allow`. | `demo/ecommerce/refund.check.ts` | unit | PASS |

## Coverage and gaps

No coverage command exists for `@trustloopguard/demo`; validation used the
targeted offline assertion suite plus TypeScript typecheck. Manual server-backed
validation was not run because it requires a running Rust server and owner/admin
`TL_USER_ID`.

## Deviations

`demo/tsconfig.json` was updated to include `ecommerce/**/*.ts` so the required
`pnpm --filter @trustloopguard/demo typecheck` command validates the new pilot.
