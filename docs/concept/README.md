# TrustLoopGuard concepts

Plain-English explanations of every moving part. Read these in order if you're
new, or use the visual map below to jump to the part you need.

## What TrustLoopGuard is

> Ultra-low-latency runtime safety layer for production AI agents. The moment before an agent speaks, sends, clicks, or commits — TrustLoopGuard returns `permit | transform | require_approval | defer | deny`.

Customers integrate one primitive into their agent loop:

```
agent proposes output → guard(...) / GuardEvent → authorization kernel → decision + receipt
```

That runtime check is the product. SDK callers receive the decision and handle it in code; gateway callers route provider traffic through TrustLoopGuard and let the Rust proxy apply dashboard-managed enforcement.

![TrustLoopGuard concept overview](assets/trustloop-concept.svg)

## Visual map

| What you need to understand | Start here | What the diagram explains |
|---|---|---|
| The product concept | [architecture.md](architecture.md) | TrustLoopGuard is a gate in the agent output path, not the agent itself. |
| Runtime data ownership | [architecture.md](architecture.md#runtime-data-flow) | SDKs and the dashboard both reach Rust; the dashboard does not own guardrail state. |
| Event-engine contract | [event-engine.md](event-engine.md) | `GuardEvent` vocabulary, event-stage seams, policy evaluation, and decision evidence. |
| Authorization kernel | [authorization-kernel.md](authorization-kernel.md) | Shared effects, approvals, grants, leases, and receipts across every domain. |
| Financial authorization | [financial-authorization.md](financial-authorization.md) | Typed financial policy, execution, ledger, outcome, and reversal semantics. |
| Policies | [policies.md](policies.md) | Unified Rust policy registry, policy families, environment deployment, and domain wrappers. |
| Environments | [environments.md](environments.md) | Runtime keys, policy deployments, runs, traces, and analytics are scoped by environment. |
| Product usage analytics | [product-analytics.md](product-analytics.md) | PostHog observes marketing and dashboard use without owning guardrail/runtime data. |
| Policy authoring | [../policies/README.md](../policies/README.md) | YAML policies are validated, saved, evaluated, and then surfaced in traces. |
| Customer integration | [../INTEGRATION.md](../INTEGRATION.md) | Teams install an SDK, register an agent, write policies, call `guard()`, then tune from traces. |

## Reading order

1. [architecture.md](architecture.md) — the big picture: how the pieces fit, how a request flows, where the latency goes.
2. [event-engine.md](event-engine.md) — the SDK-first event contract and no-op runtime seams.
3. [financial-authorization.md](financial-authorization.md) — the typed financial action contract and policy family.
4. [crates.md](crates.md) — what each crate is for, in order of dependency.
5. [glossary.md](glossary.md) — every domain term defined once: Channel, Authorization effect, Policy, Decision, hot path, etc.
6. [runs.md](runs.md) — how agent executions group decision traces for monitoring.
7. [analytics-dashboards.md](analytics-dashboards.md) — how customizable analytics queries and saved dashboard views work.
8. [gateway.md](gateway.md) — how proxy/gateway mode differs from SDK mode.
9. [agent-breakaway-arena.md](agent-breakaway-arena.md) — the raw-vs-guarded comparison concept and the agent adapter contract the demos use.
10. [sdk-publishing.md](sdk-publishing.md) — how `@trustloopguard/sdk` is released to npm.

## When to update these docs

- Changed the shape of `GuardEvent` or `Decision`? → update `glossary.md` and `architecture.md`.
- Added a new crate or split one? → update `crates.md`.
- Changed how a request flows through the system? → update `architecture.md`.
- Changed the event-engine contract or stage seams? → update `event-engine.md`.
- Changed the financial action contract, financial policy family, outcome semantics, or reversal vocabulary? → update `financial-authorization.md` and `glossary.md`.
- Changed the proxy integration path? → update `gateway.md`.
- Added or changed execution grouping? → update `runs.md`.
- Changed the SDK release workflow or npm package process? → update `sdk-publishing.md`.
- Changed the raw-vs-guarded comparison concept or the agent adapter contract? → update `agent-breakaway-arena.md`.
- Changed PostHog initialization, identity, or product event names? → update `product-analytics.md`.

## Diagram workflow

Architecture diagrams are generated from D2 source files in
[`../diagrams`](../diagrams). Edit the `.d2` file first, then regenerate the
SVG assets used by both repo Markdown and the docs website:

```bash
pnpm docs:diagrams
# or
make diagrams
```

The command requires the D2 CLI. On macOS:

```bash
brew install d2
```

Keep these docs short. If something gets long, split it. The point is to onboard a new contributor in 15 minutes, not to be exhaustive.
