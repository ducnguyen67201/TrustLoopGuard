# Phase 3.5 - Direct Event Ingestion (Observe-Only)

Status: **implemented in this branch.**

Implementation report: `.claude/PRPs/reports/event-engine-phase-03b-event-ingestion-report.md`.

## Purpose

Give SDK-shaped events a direct entry point so labeled evidence flows from
real traffic while everything stays observe-only. This pulls forward task 3
of phase 7 ("expose SDK/MCP event entry shape when ready"): the `GuardEvent`
contract froze in phases 0-3, the pipeline is event-native, and collecting
real sources and provenance before enforcement exists is what validates the
label design.

## Independent Ship Boundary

Phase 3.5 can ship by itself when:

- a full `GuardEvent` (sources + provenance) can be submitted directly,
- the event runs through the existing pipeline and its evidence persists
  in traces,
- the response is an explicit observe-only `Decision` (always `allow`),
- `/v1/check` behavior is byte-identical,
- no checker or enforcement behavior exists on the new path.

## Dependencies

- Phase 0 for the `GuardEvent` contract.
- Phase 1 for trace evidence persistence.
- Phase 2 for action resolution.
- Phase 3 for label resolution and provenance propagation.

## Inputs

| Input | Source | Notes |
|---|---|---|
| `GuardEvent` body | SDK / adapter / integrator | The frozen wire type, verbatim |
| workspace id | `x-tlg-workspace-id` header, else `principal.workspace_id`, else default | Pipeline overwrites principal identity with the server-resolved value |
| environment id | header resolution (same as `/v1/check`) | |
| workspace settings | settings store | Data-handling gate |

## Outputs

| Output | Consumer | Notes |
|---|---|---|
| `Decision` (always `allow`) | SDK caller | Reason: `observe-only: event recorded; checkers not yet enforcing` |
| persisted trace with full event evidence | dashboard/audit | Tool resolution + label resolution + derived labels |

## Contract Decisions

- **Route**: `POST /v1/events`, behind the same bearer-auth layer as
  `/v1/check`.
- **Response**: the existing `Decision` wire type with `verdict: allow` and
  an explicit observe-only reason. When later phases wire live checkers, the
  same endpoint and SDK methods start returning real verdicts with no
  breaking change.
- **No tier engine run**: the legacy engine evaluates check text, which
  events do not carry. The decision is seeded `allow` and only the event
  pipeline enriches evidence.
- **No run check-stats**: ingested events are not guard checks;
  `record_check` is deliberately not called so run statistics stay accurate.
- **Trace domain**: ingested events persist with `domain: "event"` —
  additive for dashboards; the legacy `customer_support` default is a
  chat-era value that does not describe event traffic.
- **Run linkage**: `principal.run_id` must be a UUID that exists in the
  resolved workspace/environment; `principal.run_event_id` must be a UUID
  and requires `run_id`. No run-event creation on this path.
- **Data handling**: event redaction does not exist yet, so workspaces not
  in `raw_allowed` mode are rejected with a clear error rather than
  silently persisting raw payloads.

## Validation Limits

Submitted events are bounded with named limits (violations return 422):

- 64 sources; ids unique, non-empty, at most 256 bytes; kinds at most
  256 bytes,
- 128 provenance paths; non-empty, at most 512 bytes; at most 32 source ids
  per path,
- non-empty `action.operation` and `principal.agent_id` (at most 256 bytes),
- serialized `parameters` and `context` at most 64 KiB each.

## Implementation Tasks

1. Add the `/v1/events` handler and service mirroring the `/v1/check` split.
2. Validate events against the limits above.
3. Run the pipeline, stamp latency, persist the trace.
4. Register the route, OpenAPI path, and `events` tag.
5. Add `submit_event` to the Rust, Python, and TypeScript SDKs.
6. Add endpoint and SDK tests.

## Testing Requirements

| Test | Expected Result |
|---|---|
| minimal event | 200, allow, observe-only reason, trace id |
| full event with registered tool + label policy | trace carries resolution, labels with basis, derived labels |
| spoofed principal workspace | trace shows server-resolved workspace |
| over-limit / malformed event | 422 with specific message |
| run linkage errors | 400 invalid / 404 unknown run |
| non-raw_allowed workspace | 400 |
| `/v1/check` before/after | identical responses |
| SDK round trips | typed `Decision` in all three SDKs |

Recommended commands:

```bash
cargo test -p tl-server --test event_ingestion
cargo test -p tl-sdk-rust
cargo test --workspace
pnpm codegen:check
```

## Design Checklist

- [x] Direct `GuardEvent` ingestion exists.
- [x] Evidence from phases 2-3 persists for ingested events.
- [x] Response is explicitly observe-only.
- [x] Input limits enforced.
- [x] `/v1/check` unchanged.
- [x] No enforcement behavior added.

## Research Alignment

- Phase 7 task 3, pulled forward: SDK/MCP events use the same `GuardEvent`
  contract.
- Collecting real provenance during the observe-only window validates the
  label design before enforcement phases depend on it.

## Clean Architecture Gate

- The endpoint reuses the pipeline seams; no new evaluation logic.
- Wire contract unchanged — the body is the existing `GuardEvent`.
- Decision semantics stay owned by the existing runtime path; this endpoint
  only seeds a constant observe-only decision.

## Not Building

- SDK provenance-collecting adapters (framework hooks) — separate phase.
- Event redaction.
- Checker or enforcement behavior.
- Run-event creation, run check-stat recording, escalation wiring.
- Removal of the legacy `/v1/check` adapter.

## Completion Statement

Phase 3.5 is complete when a full `GuardEvent` can be submitted directly,
its phase 2-3 evidence persists in traces, and the response is an explicit
observe-only allow — with `/v1/check` untouched.
