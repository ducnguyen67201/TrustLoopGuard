# Full Demo Cookbook

This collection explains every runnable top-level scenario under `demo/`.
It excludes `shared/`, `node_modules/`, and local data because they support
demos but are not demos themselves.

The guides do not copy implementation code. They point to the existing demo
entrypoints, setup instructions, and tests so there is still one executable
source of truth.

## Choose A Demo

| Demo directory | Learn this concept | Fastest verification |
| --- | --- | --- |
| [`agent-visibility`](agent-visibility.md) | Decorate an agent once and group tool/output traces in one Run | Demo package typecheck |
| [`arena`](arena-adapter.md) | Expose an agent through the Attacks-compatible adapter contract | `arena:check` |
| [`contextual-agent`](contextual-agent.md) | Guard a reusable public workflow at input and output boundaries | `test:contextual-demo` |
| [`dispute`](dispute.md) | Compare raw and guarded money actions, then gate execution on the effect | Two offline checks |
| [`financial-refund`](financial-refund.md) | Exercise typed financial authorization without external services | `financial-refund:check` |
| [`healthcare-agent`](healthcare-agent.md) | Stop unsafe healthcare requests before the model and guard its reply | `test:healthcare-demo` |
| [`livekit`](livekit.md) | Apply SDK or gateway guardrails to a realtime voice agent | Python compile check |
| [`procurement-agent`](procurement-agent.md) | Authorize a canonical purchase-order action before execution | `test:procurement-demo` |
| [`stripe-refund-agent`](stripe-refund-agent.md) | Connect typed refund authorization to a customer DB and payment provider | `stripe-refund-agent:check` |

## Suggested Learning Order

1. Start with [agent visibility](agent-visibility.md) to see one decorated
   agent produce a grouped tool trace and output trace.
2. Run [financial refund](financial-refund.md) to understand typed
   authorization without a server or provider.
3. Use [dispute](dispute.md) to compare permit, deny, defer, and
   require-approval outcomes.
4. Move to the integrated public demos: [procurement](procurement-agent.md),
   [healthcare](healthcare-agent.md), [contextual workflows](contextual-agent.md),
   and [Stripe refund](stripe-refund-agent.md).
5. Use [LiveKit](livekit.md) and the [arena adapter](arena-adapter.md) when
   integrating a framework or the Attacks workflow.

## Shared Runtime Rule

All guarded demo paths call the Rust API directly through a TrustLoopGuard SDK,
especially `POST /v1/events`. Marketing pages are presentation and same-origin
proxy surfaces; they do not own policy evaluation, runtime authorization,
traces, agents, or financial state.

See [TEST_LOG.md](TEST_LOG.md) for the commands most recently run while checking
this catalog.
