# E-commerce refund pilot

This pilot is a narrow SDK demo for support teams that want to guard one refund
or store-credit tool before money-like value moves. It is not a Shopify app and
does not call a real refund API: bring your refund tool, register its metadata,
and ask TrustLoopGuard before the side effect executes.

The scenario set covers:

| Scenario | Expected outcome | Control |
| --- | --- | --- |
| Legit $50 refund to the order registry destination | allow | none |
| $750 refund to the trusted customer account | block | value limit |
| $50 refund to a user-injected destination | block | parameter auth |
| Non-integer refund amount | escalate | value limit |
| High-risk store credit | escalate | approval |

## Run offline

The check mode needs no server or keys:

```sh
pnpm --filter @trustloopguard/demo ecommerce:check
```

It verifies event shape, trusted vs injected provenance, integer-cent amounts,
and that the simulated ledger writes only when the verdict is `allow`.

## Run against the local guard

Start the Rust server, register the pilot tool metadata, then run the guarded
scenario table:

```sh
make server
TL_USER_ID=<owner-uuid> pnpm --filter @trustloopguard/demo ecommerce:setup
pnpm --filter @trustloopguard/demo ecommerce:refund
```

`TL_USER_ID` lets setup arm `param_checker_mode` and `approval_checker_mode` in
`enforce`. If every scenario is allowed, the runner exits non-zero because the
workspace is not enforcing the pilot controls.

## Seven-day partner checklist

Use this as a one-tool pilot around a real refund or credit workflow:

| Day | Check |
| --- | --- |
| 1 | Pick one refund or store-credit tool and map its parameters to `order_id`, `customer_id`, `amount`, `refund_method`, and `destination`. |
| 2 | Register tool metadata with an amount cap, trusted destination source, and approval rule. |
| 3 | Send proposed tool calls through `/v1/events` before the refund side effect. |
| 4 | Count total refund actions observed and the number TrustLoopGuard would have blocked or escalated. |
| 5 | Review false positives with support leads and adjust caps or trusted sources. |
| 6 | Confirm the approval handoff for escalated store credit. |
| 7 | Decide whether to enforce, monitor more traffic, or expand to a second money-like tool. |
