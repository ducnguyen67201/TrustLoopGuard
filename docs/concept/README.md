# TrustLoopGuard concepts

Plain-English explanations of every moving part. Read these in order if you're
new, or use the visual map below to jump to the part you need.

## What TrustLoopGuard is

> Ultra-low-latency runtime safety layer for production AI agents. The moment before an agent speaks, sends, clicks, or commits — TrustLoopGuard decides `allow | block | rewrite | escalate`.

Customers integrate one primitive into their agent loop:

```
agent proposes output → trustloop.check(...) → allow | block | rewrite | escalate → log
```

That runtime check is the product. SDK callers receive the decision and handle it in code; gateway callers route provider traffic through TrustLoopGuard and let the Rust proxy apply dashboard-managed enforcement.

![TrustLoopGuard concept overview](assets/trustloop-concept.svg)

## Visual map

| What you need to understand | Start here | What the diagram explains |
|---|---|---|
| The product concept | [architecture.md](architecture.md) | TrustLoopGuard is a gate in the agent output path, not the agent itself. |
| Runtime data ownership | [architecture.md](architecture.md#runtime-data-flow) | SDKs and the dashboard both reach Rust; the dashboard does not own guardrail state. |
| Event-engine contract | [event-engine.md](event-engine.md) | `CheckRequest` compatibility, `GuardEvent` vocabulary, no-op stage seams, and decision evidence. |
| Environments | [environments.md](environments.md) | Runtime keys, policy deployments, runs, traces, and analytics are scoped by environment. |
| Policy authoring | [../policies/README.md](../policies/README.md) | YAML policies are validated, saved, evaluated, and then surfaced in traces. |
| Customer integration | [../INTEGRATION.md](../INTEGRATION.md) | Teams install an SDK, register an agent, write policies, call `check()`, then tune from traces. |

## Reading order

1. [architecture.md](architecture.md) — the big picture: how the pieces fit, how a request flows, where the latency goes.
2. [event-engine.md](event-engine.md) — the SDK-first event contract and no-op runtime seams.
3. [crates.md](crates.md) — what each crate is for, in order of dependency.
4. [glossary.md](glossary.md) — every domain term defined once: Channel, Verdict, Policy, Decision, hot path, etc.
5. [runs.md](runs.md) — how agent executions group decision traces for monitoring.
6. [analytics-dashboards.md](analytics-dashboards.md) — how customizable analytics queries and saved dashboard views work.
7. [gateway.md](gateway.md) — how proxy/gateway mode differs from SDK mode.
8. [agent-breakaway-arena.md](agent-breakaway-arena.md) — how the public arena demo connects to raw and guarded agent adapters.
9. [sdk-publishing.md](sdk-publishing.md) — how `@trustloopguard/sdk` is released to npm.

## When to update these docs

- Changed the shape of `CheckRequest` or `Decision`? → update `glossary.md` and `architecture.md`.
- Added a new crate or split one? → update `crates.md`.
- Changed how a request flows through the system? → update `architecture.md`.
- Changed the event-engine contract or stage seams? → update `event-engine.md`.
- Changed the proxy integration path? → update `gateway.md`.
- Added or changed execution grouping? → update `runs.md`.
- Changed the SDK release workflow or npm package process? → update `sdk-publishing.md`.
- Changed the public arena demo or adapter contract? → update `agent-breakaway-arena.md`.

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
