# TrustLoopGuard concepts

Plain-English explanations of every moving part. Read these in order if you're new.

## What TrustLoopGuard is

> Ultra-low-latency runtime safety layer for production AI agents. The moment before an agent speaks, sends, clicks, or commits — TrustLoopGuard decides `allow | block | rewrite | escalate`.

Customers integrate one primitive into their agent loop:

```
agent proposes output → trustloop.check(...) → allow | block | rewrite | escalate → log
```

That runtime check is the product. SDK callers receive the decision and handle it in code; gateway callers route provider traffic through TrustLoopGuard and let the Rust proxy apply dashboard-managed enforcement.

## Reading order

1. [architecture.md](architecture.md) — the big picture: how the pieces fit, how a request flows, where the latency goes.
2. [crates.md](crates.md) — what each of the 9 crates is for, in order of dependency.
3. [glossary.md](glossary.md) — every domain term defined once: Channel, Verdict, Policy, Decision, hot path, etc.
4. [runs.md](runs.md) — how agent executions group decision traces for monitoring.
5. [gateway.md](gateway.md) — how proxy/gateway mode differs from SDK mode.
6. [sdk-publishing.md](sdk-publishing.md) — how `@trustloopguard/sdk` is released to npm.

## When to update these docs

- Changed the shape of `CheckRequest` or `Decision`? → update `glossary.md` and `architecture.md`.
- Added a new crate or split one? → update `crates.md`.
- Changed how a request flows through the system? → update `architecture.md`.
- Changed the proxy integration path? → update `gateway.md`.
- Added or changed execution grouping? → update `runs.md`.
- Changed the SDK release workflow or npm package process? → update `sdk-publishing.md`.

Keep these docs short. If something gets long, split it. The point is to onboard a new contributor in 15 minutes, not to be exhaustive.
