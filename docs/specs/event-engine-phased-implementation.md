# Event Engine - Master Implementation Spec

Status: **Phase 0 implemented; later phases remain planning documentation.**
This is the master index for the event-engine implementation plan. It breaks
the target design into focused, independently executable phase specs.

This file does not claim the behavior is shipped. `docs/concept/` remains the
canonical source for current product behavior.

## Source Design

- [`event-engine-design.md`](./event-engine-design.md) - high-level target shape
- [`event-engine-class-and-db-design.md`](./event-engine-class-and-db-design.md) - type, trait, and database sketches
- [`event-engine-roadmap.md`](./event-engine-roadmap.md) - critical path and research grounding
- [`integration-interception.md`](./integration-interception.md) - gateway vs SDK/adapter vs MCP capture
- [`../research/trustloopguard-runtime-security-architecture/main.pdf`](../research/trustloopguard-runtime-security-architecture/main.pdf) - research basis

## Why This Exists

The event engine is too large to build as one PR. The implementation must be
split so each phase can:

- ship independently,
- compile and test without later phases,
- start with no-op or observe-only behavior,
- check off a specific part of the design,
- align with the research paper,
- preserve clean crate boundaries.

The first implementation focus is the input layer: where evidence is collected,
how raw inputs become `GuardEvent`, and what happens after collection.

## Phase Specs

| Phase | Spec | Purpose | Default Behavior |
|---:|---|---|---|
| 0 | [`phase-00-contract-noop.md`](./event-engine-phases/phase-00-contract-noop.md) | Event contract and no-op pipeline skeleton | Implemented with no behavior change |
| 1 | [`phase-01-input-collection-observe.md`](./event-engine-phases/phase-01-input-collection-observe.md) | Collect and normalize inputs; persist event evidence | Observe-only |
| 2 | [`phase-02-tool-metadata.md`](./event-engine-phases/phase-02-tool-metadata.md) | Tool metadata registry and action resolution | Metadata only |
| 3 | [`phase-03-labels-provenance.md`](./event-engine-phases/phase-03-labels-provenance.md) | Label resolution and provenance propagation | Shadow evidence only |
| 4 | [`phase-04-flow-memory.md`](./event-engine-phases/phase-04-flow-memory.md) | Information-flow checker and memory write-time block | OFF by default |
| 5 | [`phase-05-parameter-auth.md`](./event-engine-phases/phase-05-parameter-auth.md) | Parameter-source authorization | OFF by default |
| 6 | [`phase-06-policy-approvals.md`](./event-engine-phases/phase-06-policy-approvals.md) | Explainable policy, approvals, and decision evidence | Backward compatible |
| 7 | [`phase-07-final-wiring-bench.md`](./event-engine-phases/phase-07-final-wiring-bench.md) | Final pipeline wiring, rollout controls, and bench start | Explicit rollout only |

## Global Architecture Rules

These rules apply to every phase:

- Rust remains the source of truth for runtime guardrail behavior.
- `tl-core` owns public wire contracts.
- `tl-engine` owns the runtime pipeline and deterministic checkers.
- `tl-server` owns HTTP/auth/orchestration and stays thin.
- `tl-storage` owns durable runtime/control-plane state.
- `apps/web` must not own runtime guardrail data.
- Framework-specific adapter code must not enter `tl-core` or `tl-engine`.
- Every enforcement-capable checker ships `OFF -> SHADOW -> ENFORCE`.
- Trace writes remain asynchronous and non-blocking.
- LLM/classifier output is an advisory signal for actions, never the security boundary.
- Legacy `/v1/check` behavior remains unchanged until explicit opt-in enforcement.

## Canonical Runtime Shape

Every input path becomes the same event contract:

```text
RawInput from CheckRequest / SDK / gateway / MCP
  -> Normalizer
  -> GuardEvent {
       kind,
       principal,
       action,
       sources,
       provenance,
       context
     }
```

The event then moves through a fixed pipeline:

```text
GuardEvent
  -> Resolve principal + action
  -> Enrich labels + provenance
  -> Run checkers, each OFF/SHADOW/ENFORCE
  -> Compose decision
  -> Persist trace evidence asynchronously
```

## Collection Model

| Collection Path | Where It Runs | What It Sees | Fidelity | Role |
|---|---|---|---|---|
| Gateway | Provider-compatible proxy | prompts, completions, model-proposed tool calls | low | easy on-ramp |
| SDK/framework adapter | inside the agent loop before function execution | proposed actions, sources, provenance, run/session context | high | full product |
| MCP proxy | between agent and MCP tool servers | MCP tool calls and responses | medium | tool-boundary integration |

The LLM does not execute tools. It asks the orchestrator to call functions. The
high-fidelity TrustLoopGuard boundary is the adapter hook before that function
runs.

## Phase Independence Contract

A phase is independently executable only if:

- it has a clear input and output contract,
- its default behavior is no-op, disabled, or shadow-only,
- it compiles and tests without later phases,
- it updates generated contracts when public DTOs change,
- it updates `docs/concept/` only when behavior actually ships,
- it includes evidence that the relevant design checklist is satisfied,
- it names the research-paper section it implements or prepares.

## Required Done Phrases For Future PRs

Use these checks in each implementation PR:

- Input contract is defined and tested.
- Output contract is defined and tested.
- No-op/default behavior is explicit and tested.
- This phase can ship independently without later phases.
- Design checklist items satisfied: `<list>`.
- Research alignment verified against paper sections: `<list>`.
- Legacy `/v1/check` compatibility is preserved unless this phase explicitly enables opt-in enforcement.
- OFF/SHADOW/ENFORCE behavior is tested for every enforcement-capable checker.
- Trace persistence remains fire-and-forget and non-blocking.
- LLM/classifier signals are advisory for actions and are never the security boundary.
- Rust backend remains source of truth; web does not own runtime guardrail data.
- No framework-specific types were introduced into `tl-core` or `tl-engine`.
- Code follows existing `Arc<dyn Trait>` plus `NoOp` collaborator patterns.
- Generated contracts are updated or `pnpm codegen:check` proves no drift.
- Concept docs are updated only for shipped behavior, not speculative target design.

## Validation Command Menu

Use the smallest meaningful gate for each phase, then broaden before merge:

```bash
cargo fmt --all -- --check
cargo test -p tl-core
cargo test -p tl-engine
cargo test -p tl-server
pnpm codegen:check
pnpm test:backend
make backend-test-db
pnpm verify:full
```

Run `make backend-test-db` when a phase changes Diesel migrations or Postgres
repositories.
