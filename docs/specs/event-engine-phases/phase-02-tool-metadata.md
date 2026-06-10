# Phase 2 - Tool Metadata Registry And Action Resolution

Status: **implemented in this branch.**

Implementation report: `.claude/PRPs/reports/event-engine-phase-02-tool-metadata-report.md`.

## Purpose

Give actions structured semantics before checkers rely on them. A tool name is
not enough to decide safety; the runtime needs side effects, parameter roles,
allowed sources, approval rules, and sandbox hints.

## Independent Ship Boundary

Phase 2 can ship by itself when:

- tool metadata can be stored and retrieved,
- action resolution can attach metadata to events,
- unregistered tools are represented conservatively,
- no checker blocks solely because metadata exists or is missing.

## Dependencies

- Phase 0 for `ToolMetadata` and `GuardEvent` types.
- Phase 1 is recommended so metadata resolution is visible in traces.

## Inputs

| Input | Source | Notes |
|---|---|---|
| `ToolMetadata` payload | control-plane API or seed data | Workspace scoped |
| `GuardEvent.action.operation` | event pipeline | Lookup key |
| workspace id | principal/server context | Cache and shard key |

## Outputs

| Output | Consumer | Notes |
|---|---|---|
| stored metadata | control plane and runtime provider | Durable registry |
| resolved action semantics | event pipeline | Side effect, param roles, allowed sources |
| unregistered-tool evidence | traces/later checkers | Conservative but not enforced yet |

## Metadata Contract

Metadata should describe:

- `tool`,
- `side_effect`,
- `reversible`,
- `params[]`,
- `ParamSpec.path`,
- `ParamSpec.role`,
- `ParamSpec.allowed_sources`,
- optional `approval`,
- optional `sandbox_hint`.

Parameter roles:

- `authority_bearing`: decides who/where/what authority is used.
- `content_bearing`: carries data but does not grant authority by itself.

## Storage Design

Create a workspace-scoped `tool_metadata` table:

- primary key: `(workspace_id, tool)`,
- `side_effect`,
- `reversible`,
- `spec JSONB`,
- `enabled`,
- timestamps,
- `deleted_at`.

Mirror existing repository patterns:

- Diesel queries stay inside `tl-storage`.
- Repository exposes typed methods.
- Reads are cached.
- Soft-deleted rows are hidden from normal reads.

## Runtime Resolution

```text
GuardEvent.action.operation
  -> ToolMetadataProvider.get(workspace_id, operation)
  -> Some(metadata): attach side_effect and param specs
  -> None: mark unregistered-tool evidence for later phases
```

No blocking happens in this phase.

## Implementation Tasks

1. Add migration and Diesel schema for `tool_metadata`.
2. Add `ToolMetadataRepo`.
3. Add server store trait/adapter if CRUD is in scope.
4. Add control-plane CRUD routes if needed.
5. Add `ToolMetadataProvider` implementation.
6. Add validation for side effects, param paths, roles, and allowed sources.
7. Add trace evidence for resolved/unregistered metadata.

## Testing Requirements

| Test | Expected Result |
|---|---|
| insert/get metadata | typed metadata round trips |
| list metadata | only active workspace rows returned |
| update metadata | cache invalidates or refreshes |
| soft delete | normal get returns not found |
| invalid param path | validation error |
| unregistered tool | evidence only, no block |
| action resolution | event carries resolved side effect and params |

Recommended commands:

```bash
cargo test -p tl-core
cargo test -p tl-storage
cargo test -p tl-server
make backend-test-db
pnpm codegen:check
```

## Design Checklist

- [x] Tool metadata registry exists.
- [x] Registry is workspace scoped.
- [x] Metadata reads are cacheable.
- [x] Action resolution attaches side effect and param roles.
- [x] Unregistered tool default is conservative evidence.
- [x] No enforcement behavior changes.

## Research Alignment

- Paper section X: tool and action metadata.
- ToolSafe/ToolSword/ShieldAgent grounding: safety checks need pre-invocation
  action semantics.

## Clean Architecture Gate

- Control-plane CRUD stays out of the deterministic checker loop.
- Hot path reads through a provider/cache seam.
- Durable registry is Rust/storage owned.
- Web, if updated later, must proxy Rust APIs.

## Not Building

- Parameter-source authorization.
- Information-flow enforcement.
- Sandbox adapter enforcement.
- Tool-registry signing or supply-chain verification.

## Completion Statement

Phase 2 is complete when workspace-scoped tool metadata can be managed, cached,
and attached to events without changing decisions.
