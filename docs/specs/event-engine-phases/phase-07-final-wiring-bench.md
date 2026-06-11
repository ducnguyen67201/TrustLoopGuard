# Phase 7 - Final Wiring, Rollout Controls, And Bench Start

Status: **implemented in this branch** (the pipeline wiring for `/v1/check`,
gateway, and `/v1/events` shipped with phases 3b-6; this branch adds the
checker-mode write path, per-environment rollout scoping, advisory signal
shedding, legacy compatibility fixtures, operational mode/latency tracing,
and the TrustLoopGuardBench smoke harness).

## Purpose

Wire the independently built event-engine pieces into one runtime path and start
TrustLoopGuardBench once traces are rich enough to evaluate real behavior.

## Independent Ship Boundary

Phase 7 can ship when:

- legacy checks enter through the event pipeline,
- gateway/SDK/MCP events use the same `GuardEvent` contract,
- rollout controls gate every checker,
- default behavior remains safe,
- initial bench scenarios can run against structured traces/checkers.

## Dependencies

- Phases 0-6 for full wiring.
- Bench smoke can begin after Phase 4 produces structured enforcement evidence.

## Inputs

| Input | Source | Notes |
|---|---|---|
| `GuardEvent` | normalizer/adapters | one contract |
| checker configs | workspace/environment settings | OFF/SHADOW/ENFORCE |
| metadata/labels/provenance | prior phases | evidence inputs |
| traces | trace store | bench and dashboard evidence |
| bench scenarios | test harness | attack/utility/runtime metrics |

## Outputs

| Output | Consumer | Notes |
|---|---|---|
| event pipeline as default internal path | `/v1/check`, gateway, SDKs | compatible by default |
| rollout controls | operators/workspaces | per-checker mode |
| bench v1 smoke | developers/CI | initial regression harness |
| metrics | dashboard/bench | false block/escalation, latency, evidence accuracy |

## Final Wiring Rules

- `CheckRequest` always goes through the normalizer.
- Event-shaped inputs go through the same event pipeline.
- Checkers run according to workspace/environment mode.
- Trace persistence stays async.
- LLM/advisory signal path is sheddable under load.
- Deterministic core stays available under overload.

## Rollout Controls

Every checker must support:

- OFF: no behavior change,
- SHADOW: evidence only,
- ENFORCE: verdict can change.

Rollout should be scoped by workspace and environment. Defaults are OFF unless a
phase-specific migration/config explicitly sets a safer shadow default.

## Bench Start

TrustLoopGuardBench starts once Phase 4 evidence is available and matures after
Phase 6.

Initial tracks:

1. indirect prompt injection,
2. private-data flow,
3. delayed memory risk.

Metrics:

- attack success rate,
- unsafe source-to-sink rate,
- parameter-source catch rate,
- unsafe-memory catch rate,
- benign task completion,
- false-block rate,
- false-escalation rate,
- latency overhead,
- LLM calls per decision,
- cost per request,
- trace explanation quality.

## Implementation Tasks

1. Route `/v1/check` through the event pipeline.
2. Route gateway event-shaped checks through the same pipeline.
3. Expose SDK/MCP event entry shape when ready.
4. Add or wire checker-mode config source.
5. Add end-to-end compatibility fixtures.
6. Add bench smoke harness and seed scenarios.
7. Add operational metrics for latency and checker modes.

## Testing Requirements

| Test | Expected Result |
|---|---|
| legacy e2e fixture | same decision as current behavior by default |
| SDK-like event | full provenance reaches checkers and trace |
| checker OFF | no verdict changes |
| checker SHADOW | evidence only |
| checker ENFORCE | configured verdict changes |
| overload/shedding | advisory signal skipped before deterministic core |
| bench smoke | one scenario per initial track runs |

Recommended commands:

```bash
cargo test -p tl-core
cargo test -p tl-engine
cargo test -p tl-server
pnpm test:backend
pnpm verify:full
```

## Design Checklist

- [x] One event pipeline is the internal default.
- [x] Rollout controls are wired.
- [x] Legacy compatibility is proven.
- [x] Bench smoke exists.
- [x] Deterministic core remains in-process.
- [x] Scaling seams remain swappable.

## Research Alignment

- Paper sections XX-XXII: benchmark dimensions and evolving evaluation loop.
- Paper section XIX: structured traces enable monitoring and audit.
- Paper thesis: runtime enforcement remains outside the LLM.

## Clean Architecture Gate

- No duplicate runtime backend in web.
- No framework-specific dependencies in core crates.
- Redis/broker/ClickHouse/edge sidecar remain behind interfaces until scaling
  triggers require them.
- Final wiring does not bypass no-op/default safety.

## Not Building

- Full ClickHouse deployment unless volume requires it.
- External durable broker unless horizontal scale requires it.
- Edge sidecar as required runtime.
- Supply-chain signing.
- Full cross-session memory graph.

## Completion Statement

Phase 7 is complete when all event-engine components are wired through one
rollout-controlled path, legacy behavior is compatible by default, and bench
smoke scenarios can run against structured evidence.
