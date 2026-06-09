# Phase 1 - Input Collection, Normalization, And Observe-Only Traces

Status: **implemented in this branch.**

Implementation report: `.claude/PRPs/reports/event-engine-phase-01-input-collection-observe-report.md`.

## Purpose

Implement the first input layer. This phase answers where evidence is collected,
how raw inputs become `GuardEvent`, and what happens after collection. It records
event evidence without enforcement.

## Independent Ship Boundary

Phase 1 can ship by itself when:

- raw inputs normalize into `GuardEvent`,
- traces include event evidence when available,
- missing event evidence does not block,
- legacy decisions remain unchanged,
- trace writing stays fire-and-forget.

## Dependencies

- Phase 0 contract and no-op pipeline.

## Inputs

| Input | Collection Point | Fidelity | Notes |
|---|---|---:|---|
| legacy `CheckRequest` | `/v1/check` | medium | Always available |
| gateway request/response | provider-compatible gateway | low | Sees model I/O and proposed tool calls |
| SDK adapter event | framework/tool wrapper | high | Sees actual execution boundary |
| MCP proxy event | MCP boundary | medium | Sees protocol-level tool requests |

## Outputs

| Output | Consumer | Notes |
|---|---|---|
| normalized `GuardEvent` | event pipeline | Self-contained proposed step |
| event trace payload | trace writer/dashboard | Includes event kind, principal, action, sources, provenance |
| current `Decision` | legacy caller | Same verdict as current behavior |

## Collection Responsibilities

### Gateway

The gateway can collect:

- provider request metadata,
- prompt/completion shape when retention allows,
- model-proposed tool-call requests,
- low-fidelity source labels such as `input.observed` or `model.output`.

The gateway cannot prove actual execution or full parameter provenance.

### SDK/framework adapter

The SDK adapter should collect before the function runs:

- operation name,
- parameters,
- source ids,
- source origin and known labels,
- parameter-to-source provenance,
- run/session/task context,
- redaction state when applicable.

This is the full-fidelity product path.

### MCP proxy

The MCP proxy can collect:

- tool server identity,
- tool call name,
- tool call parameters,
- MCP response sources,
- medium-fidelity provenance at protocol boundaries.

## Runtime Flow

```text
raw input
  -> normalize to GuardEvent
  -> pass through no-op resolvers/checkers
  -> current content decision
  -> attach event evidence to trace payload
  -> enqueue trace asynchronously
```

## Implementation Tasks

1. Define an internal raw-input wrapper for legacy and event-shaped input.
2. Route `/v1/check` through the normalizer.
3. Add gateway event capture where provider traffic already passes through Rust.
4. Document SDK adapter hook points for later SDK implementation.
5. Extend trace payload serialization with event evidence.
6. Promote only dashboard-filtered evidence to trace columns.
7. Preserve non-blocking trace enqueue behavior.

## Trace Evidence

Trace payload should be able to carry:

- `event_kind`,
- `principal`,
- `action`,
- `sources`,
- `provenance`,
- `source_chain` when available,
- full `Decision`,
- run/run_event links.

Potential promoted columns:

- `event_kind`,
- `risk_source`,
- `failure_mode`,
- `harm_class`.

Do not promote flexible `sources` and `provenance` until a dashboard filter
requires it.

## Testing Requirements

| Test | Expected Result |
|---|---|
| legacy request normalizes | `output.proposed` event |
| gateway event normalizes | low-fidelity event with no enforcement |
| SDK-like event normalizes | high-fidelity event with sources/provenance |
| missing provenance | allowed in observe-only mode |
| trace payload includes event evidence | persisted/enqueued data is enriched |
| full trace queue | request still returns; warning logged |

Recommended commands:

```bash
cargo test -p tl-core
cargo test -p tl-server
cargo test -p tl-storage
pnpm test:backend
```

Run DB tests only if migrations or Postgres repo code changes:

```bash
make backend-test-db
```

## Design Checklist

- [x] Raw inputs normalize into one `GuardEvent` contract.
- [x] Gateway collection is explicitly low fidelity.
- [x] SDK adapter collection is explicitly high fidelity.
- [x] Trace evidence records event fields.
- [x] No enforcement behavior changes.
- [x] Trace writes remain asynchronous.

## Research Alignment

- Paper section IX: action interceptor and trace store.
- Paper section XI: provenance capture.
- Paper section XIX: traces are security artifacts.
- ATBench/trajectory framing: action history matters.

## Clean Architecture Gate

- Collection adapters translate into abstract `GuardEvent`.
- Core engine does not depend on LiveKit, LangChain, OpenAI Agents SDK, or MCP SDK types.
- Trace enrichment does not add synchronous database work to the decision path.
- Server remains orchestration, not business-rule ownership.

## Not Building

- Real flow enforcement.
- Real parameter authorization.
- Durable cross-session memory graph.
- Dashboard UX for all evidence fields.

## Completion Statement

Phase 1 is complete when every supported raw input path can produce a
`GuardEvent`, traces can record that evidence, and legacy decision behavior is
unchanged.
