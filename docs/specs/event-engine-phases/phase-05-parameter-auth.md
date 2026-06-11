# Phase 5 - Parameter Source Authorization

Status: **implemented** (shipped together with phase 4's enforcement scaffolding).

Implementation plan: `.claude/PRPs/plans/phase-05-parameter-auth.plan.md`.

## Purpose

Catch "right tool, wrong source" failures. A tool call can be syntactically valid
and still unsafe if an authority-bearing parameter came from the wrong source.

## Independent Ship Boundary

Phase 5 can ship by itself when:

- the parameter authorization checker exists,
- it supports OFF/SHADOW/ENFORCE,
- it consumes `ToolMetadata.allowed_sources`,
- it treats missing provenance as missing proof for authority-bearing params,
- it does not affect content-bearing params unless policy says so.

## Dependencies

- Phase 0 for event/checker contracts.
- Phase 1 for collection/provenance evidence.
- Phase 2 for tool metadata.
- Phase 3 for source labels/origins.

## Inputs

| Input | Source | Notes |
|---|---|---|
| `Action.parameters` | event action | JSON payload to inspect |
| `ProvenanceMap` | collector/resolver | parameter path -> source ids |
| `ToolMetadata.params` | registry | param roles and allowed sources |
| source origins/labels | label resolver | compare actual source with allowed source |
| enforcement mode | workspace/environment config | OFF, SHADOW, ENFORCE |

## Outputs

| Output | Mode | Notes |
|---|---|---|
| no finding | OFF | no effect |
| parameter violation evidence | SHADOW | expected vs actual source |
| block/escalate | ENFORCE | wrong or missing proof |

## Authorization Rule

For every authority-bearing parameter:

```text
param path
  -> provenance source ids
  -> source origins/labels
  -> compare against allowed_sources
  -> if no allowed proof: violation
```

Missing provenance is not clean provenance. In ENFORCE, a high-impact
authority-bearing parameter without proof must block or escalate.

## Examples

| Tool Param | Allowed Source | Unsafe Source |
|---|---|---|
| `send_email.to` | user prompt or trusted contact lookup | untrusted email body |
| `book_flight.flight_id` | flight search result | hotel listing webpage |
| `file.write.path` | user choice or workspace policy | model-synthesized untrusted string |
| `payment.destination` | verified account registry | webpage content |

## Implementation Tasks

1. Implement parameter-path lookup over JSON.
2. Resolve source ids for each authority-bearing param.
3. Compare source evidence with `allowed_sources`.
4. Emit expected-source vs actual-source evidence.
5. Add SHADOW and ENFORCE behavior.
6. Add remediation text.
7. Add tests for nested paths and missing provenance.

## Testing Requirements

| Test | Expected Result |
|---|---|
| correct source | allow |
| wrong source | block/escalate in ENFORCE |
| missing provenance | block/escalate in ENFORCE |
| content-bearing param | ignored by this checker |
| nested parameter path | correctly resolved |
| unregistered tool | conservative evidence, mode-dependent |
| SHADOW violation | evidence only, verdict unchanged |

Recommended commands:

```bash
cargo test -p tl-engine
cargo test -p tl-core
pnpm test:backend
```

## Design Checklist

- [x] Parameter-source checker exists.
- [x] Checker consumes `ToolMetadata`.
- [x] Checker consumes `ProvenanceMap`.
- [x] Authority-bearing and content-bearing params are distinct.
- [x] Missing proof fails closed in ENFORCE.
- [x] Evidence names the exact parameter path.

## Research Alignment

- Paper section XIII: parameter-level authorization.
- AuthGraph grounding: authorization depends on parameter provenance.

## Clean Architecture Gate

- No ad hoc string matching for source authority.
- No storage reads inside checker.
- Tool metadata is resolved through provider/context before checking.
- Evidence is structured enough for trace/audit.

## Not Building

- New tool metadata CRUD.
- Label resolver changes.
- Policy-family parser changes unless needed for config.
- Full trace graph analysis.

## Completion Statement

Phase 5 is complete when authority-bearing parameters require allowed-source
proof in ENFORCE mode and produce clear evidence in SHADOW mode.
