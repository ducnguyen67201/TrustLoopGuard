# Phase 0 - Contract And No-Op Pipeline Skeleton

Status: **implemented in this branch.**

Implementation report: `.claude/PRPs/reports/event-engine-phase-00-contract-noop-report.md`.

## Purpose

Freeze the event-engine shape without changing customer behavior. This phase
creates the wire vocabulary, no-op stage traits, and compatibility adapter that
later phases will fill in.

## Independent Ship Boundary

Phase 0 can ship by itself when:

- legacy `/v1/check` behavior is byte-compatible,
- all new stage collaborators have no-op implementations,
- new public fields are additive and optional,
- no checker changes a verdict,
- generated contracts are updated or verified unchanged.

## Inputs

| Input | Source | Required? | Notes |
|---|---|---:|---|
| `CheckRequest` | existing `/v1/check` callers | yes | Must remain accepted exactly as today |
| workspace id | header/request/server resolution | yes | Preserve current resolution behavior |
| environment id | server resolution | yes | Preserve current environment behavior |
| run/run_event ids | optional request fields | no | Carried into `Principal` when present |

## Outputs

| Output | Consumer | Notes |
|---|---|---|
| `GuardEvent` wire types | engine, SDKs, server, future adapters | Additive contract |
| optional `Decision` evidence fields | server, SDKs, traces, dashboard | Empty by default |
| no-op event pipeline context | engine tests and future server wiring | Mirrors `HandlerCtx::no_op()` |
| legacy normalizer | `/v1/check` compatibility path | Maps output check to `output.proposed` |

## Required Types

Create or prepare these modules in `tl-core`:

- `event.rs`: `GuardEvent`, `EventKind`, `Principal`, `Action`, `SideEffectClass`.
- `label.rs`: `Source`, `Origin`, `Labels`, `Trust`, `Confidentiality`, `Integrity`.
- `provenance.rs`: `ProvenanceMap`.
- `tool.rs`: `ToolMetadata`, `ParamSpec`, `ParamRole`, `AllowedSource`, `ApprovalRule`.
- `guard.rs`: additive optional decision evidence fields.

All public types must follow current wire patterns:

- `Serialize` / `Deserialize`,
- existing `JsonSchema`, `ToSchema`, and `TS` cfg attributes where public,
- `serde(rename_all = "snake_case")` for event-support enums,
- explicit dotted serde renames for `EventKind` values such as `output.proposed`,
- `#[serde(default)]` for additive fields,
- `skip_serializing_if = "Option::is_none"` for optional evidence.

## Required Engine Interfaces

Add event-stage traits in `tl-engine`. Exact module layout can be refined during
implementation, but the stage boundaries should remain:

```text
Normalizer
PrincipalResolver
ToolMetadataProvider
LabelResolver
ProvenanceResolver
Checker
SignalProvider
DecisionComposer
TracePersister
```

Each trait must be `Send + Sync`. Each implementation is held as `Arc<dyn Trait>`.
Every trait must have a no-op implementation.

## No-Op Semantics

| Component | No-op Behavior |
|---|---|
| Normalizer | maps legacy `CheckRequest` to `output.proposed`; validates only structural basics |
| PrincipalResolver | preserves existing workspace/environment/agent/run context |
| ToolMetadataProvider | returns no metadata without blocking |
| LabelResolver | leaves/defaults labels without enforcement |
| ProvenanceResolver | does not invent provenance |
| Checker | returns no finding |
| SignalProvider | returns no advisory signals |
| DecisionComposer | preserves current decision behavior |
| TracePersister | does nothing unless current writer is already configured |

## Legacy Mapping

```text
CheckRequest {
  input,
  proposed_output,
  agent_id,
  channel,
  workspace_id?,
  run_id?,
  run_event_id?,
  context?
}

-> GuardEvent {
  kind: output.proposed,
  principal: workspace/environment/agent/run/run_event,
  action.operation: "output",
  action.parameters.text: proposed_output,
  sources: input/context source entries when known,
  provenance: empty or best-effort,
  context
}
```

## Implementation Tasks

1. Add `tl-core` event vocabulary.
2. Extend `Decision` with optional evidence fields.
3. Export new types from `tl-core/src/lib.rs`.
4. Add event normalizer that supports `CheckRequest`.
5. Add event-stage traits and no-op implementations.
6. Add tests proving no-op defaults.
7. Regenerate/check OpenAPI and generated schemas.

## Testing Requirements

| Test | Expected Result |
|---|---|
| old `CheckRequest` JSON deserializes | no schema break |
| `Decision::allow` serializes without empty evidence | no noisy wire output |
| `CheckRequest` normalizes to `output.proposed` | stable legacy adapter |
| no-op pipeline returns current decision | behavior unchanged |
| golden replay old vs event path | equivalent decisions |
| codegen check | generated contracts in sync |

Recommended commands:

```bash
cargo test -p tl-core
cargo test -p tl-engine
pnpm codegen:check
pnpm test:backend
```

## Design Checklist

- [x] `GuardEvent` vocabulary exists.
- [x] `EventKind` covers output, tool, memory, file, shell, network, browser, db, api, and message events.
- [x] `Decision` has additive optional evidence fields.
- [x] All pipeline stages exist as no-ops.
- [x] Legacy `CheckRequest -> output.proposed` adapter exists.
- [x] No customer-visible behavior changes.

## Research Alignment

- Paper section VIII: output checks become one event kind.
- Paper section IX: runtime has interceptor, resolver, provenance tracker, and trace store.
- Paper thesis: the runtime, not the LLM, is the security boundary.

## Clean Architecture Gate

- `tl-core` contains data types only.
- `tl-engine` contains pipeline traits and no-op runtime behavior.
- `tl-server` does not define public DTOs.
- No framework-specific adapter types enter core crates.
- No storage/network I/O is added to the deterministic hot path.

## Not Building

- Real tool metadata persistence.
- Label propagation.
- Flow or parameter enforcement.
- Policy family expansion.
- Final pipeline wiring.

## Completion Statement

Phase 0 is complete: the event contract and no-op pipeline exist, route-level
compatibility tests prove legacy `/v1/check` responses stay quiet, and generated
contracts have no drift.
