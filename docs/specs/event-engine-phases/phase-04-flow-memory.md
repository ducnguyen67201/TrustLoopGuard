# Phase 4 - Information Flow And Memory Write-Time Block

Status: **implemented in this branch** (together with the phase 5
parameter-auth checker, pulled forward onto the same enforcement-mode
machinery).

Implementation plan: `.claude/PRPs/plans/phase-04-flow-memory.plan.md`.
Implementation report: `.claude/PRPs/reports/phase-04-flow-memory-report.md`.

## Purpose

Add the first real opt-in enforcement: information-flow checks and memory
write-time blocking. This is where the system starts protecting action safety,
but only through explicit `OFF -> SHADOW -> ENFORCE` rollout.

## Independent Ship Boundary

Phase 4 can ship by itself when:

- flow and memory checkers exist,
- each checker supports OFF, SHADOW, and ENFORCE,
- default mode is OFF,
- SHADOW records evidence without changing decisions,
- ENFORCE changes decisions only for opted-in workspaces/environments.

## Dependencies

- Phase 0 for event pipeline and checker interfaces.
- Phase 1 for trace evidence.
- Phase 3 for labels and provenance.
- Phase 2 is recommended for side-effect metadata.

## Inputs

| Input | Source | Notes |
|---|---|---|
| `GuardEvent.kind` | normalizer | Determines whether flow/memory applies |
| `Action.side_effect` | tool metadata/action resolver | Needed for sink classification |
| `Labels` | label resolver | Trust/confidentiality/integrity |
| `ProvenanceMap` | collector/propagation | Source chain for values |
| enforcement mode | workspace/environment config | OFF, SHADOW, or ENFORCE |

## Outputs

| Output | Mode | Notes |
|---|---|---|
| no finding | OFF | No evaluation or no effect |
| checker evidence | SHADOW | Persisted to trace, verdict unchanged |
| block/escalate decision | ENFORCE | Only for opted-in workspaces |

## Checker Responsibilities

### InformationFlowChecker

Implements:

- action-integrity: high-impact actions must be authorized by trusted context,
- destination-permission: sensitive data may flow only to allowed sinks.

Examples:

- private source -> external email recipient not allowed: block/escalate.
- untrusted webpage controls payment destination: block/escalate.
- trusted user instruction controls send target: allow if policy permits.

### MemoryChecker

Implements v1 memory control:

- block or escalate untrusted content becoming authority-bearing memory,
- preserve memory write evidence in traces,
- do not implement retrieval-time cross-session memory graph yet.

## Enforcement Modes

| Mode | Evaluation | Decision Effect | Trace Effect |
|---|---|---|---|
| OFF | optional/no | none | none or minimal |
| SHADOW | yes | none | full hypothetical evidence |
| ENFORCE | yes | worst verdict wins | full evidence |

## Implementation Tasks

1. Add `EnforcementMode`.
2. Add checker result/evidence shape.
3. Implement `InformationFlowChecker`.
4. Implement write-time `MemoryChecker`.
5. Update decision composer to honor modes.
6. Persist checker evidence in traces.
7. Add rollout defaults: all OFF.

## Testing Requirements

| Test | Expected Result |
|---|---|
| checker OFF | no verdict change |
| checker SHADOW | evidence recorded, verdict unchanged |
| checker ENFORCE private -> external | block/escalate |
| checker ENFORCE trusted -> allowed sink | allow |
| untrusted -> authority memory | block/escalate in ENFORCE |
| LLM signal present | does not override deterministic checker |
| missing provenance in ENFORCE high-impact action | conservative escalate/block |

Recommended commands:

```bash
cargo test -p tl-engine
cargo test -p tl-core
cargo test -p tl-server
pnpm test:backend
```

## Design Checklist

- [x] Flow checker exists.
- [x] Memory write-time checker exists.
- [x] OFF/SHADOW/ENFORCE is tested.
- [x] First behavior change is opt-in.
- [x] Shadow evidence is persisted.
- [x] Retrieval-time cross-session graph remains deferred.

## Research Alignment

- Paper section XII: information-flow control.
- Paper section XVII: memory security.
- Temporal model: T1/T2 full, T3 write-time block only for v1.

## Clean Architecture Gate

- Checkers are deterministic and in-process.
- Checkers do not perform DB/network I/O.
- Enforcement mode is data/config, not a separate code fork.
- LLM/classifier signals do not decide action safety.

## Not Building

- Full memory provenance graph.
- Sandbox enforcement adapter.
- ClickHouse/OLAP trace analytics.

## Completion Statement

Phase 4 is complete when flow and memory write-time checks can run in shadow and
enforce modes, with default OFF and trace-backed evidence.
