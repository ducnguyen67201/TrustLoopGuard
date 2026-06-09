# Phase 3 - Label Resolution And Provenance Propagation

Status: **planning documentation only.**

## Purpose

Make event evidence legible. This phase resolves trust/confidentiality/integrity
labels and propagates them over provenance, but remains shadow/observe-only.

## Independent Ship Boundary

Phase 3 can ship by itself when:

- labels can be resolved from source origins and config,
- provenance propagation is deterministic,
- trace evidence shows labels,
- no checker changes a verdict because of labels.

## Dependencies

- Phase 0 for `Source`, `Labels`, and `ProvenanceMap`.
- Phase 1 for trace evidence.

## Inputs

| Input | Source | Notes |
|---|---|---|
| `Source.origin` | SDK/gateway/MCP/server | Producer-reported fact |
| optional source labels | SDK/framework adapter | Hints, not final authority |
| `ProvenanceMap` | collector/adapter | Parameter path to source ids |
| workspace label policy | control plane | Optional override |
| redaction/PII signal | existing redaction/classifier paths | Advisory confidentiality signal |

## Outputs

| Output | Consumer | Notes |
|---|---|---|
| resolved source labels | traces/checkers | Used by later phases |
| propagated derived labels | traces/checkers | Deterministic over provenance |
| label evidence | dashboard/audit | Shows why a source was trusted/untrusted/private |

## Label Families

| Label | Values | Purpose |
|---|---|---|
| trust | trusted, untrusted, unknown | Can this source influence authority? |
| confidentiality | public, private, secret, identity | Where may this data flow? |
| integrity | low, medium, high | Can this data control privileged action? |
| origin | user, system, tool, memory, file, web, email, api, unknown | Where did the content enter? |

## Resolution Strategy

Use structure first:

1. Producer reports origin.
2. Server derives trust/confidentiality defaults from origin and workspace config.
3. Declared source metadata overrides defaults when configured.
4. Pattern/PII detectors may add confidentiality signals.
5. Classifier/LLM signals remain advisory and low authority.

Unknown external origin should resolve to untrusted evidence for later
enforcement, but Phase 3 itself does not block.

## Propagation Rules

- If any source is untrusted, the derived value is untrusted.
- If any source is secret/private/identity, the derived value carries that
  confidentiality level unless explicitly transformed by a trusted sanitizer.
- Integrity cannot be higher than the weakest authority-bearing source.
- Missing provenance is represented as unknown, not clean.

## Implementation Tasks

1. Add label defaults in code.
2. Add `source_label_policy` storage if workspace overrides are in scope.
3. Implement `LabelResolver`.
4. Implement deterministic propagation over `ProvenanceMap`.
5. Attach label evidence to traces.
6. Add tests for unknown and mixed-label sources.

## Testing Requirements

| Test | Expected Result |
|---|---|
| origin default | deterministic label output |
| workspace override | override wins over default |
| unknown origin | untrusted/unknown evidence |
| trusted + untrusted sources | derived value untrusted |
| private source | derived value private |
| missing provenance | unknown evidence, no block |
| trace payload | labels visible |

Recommended commands:

```bash
cargo test -p tl-core
cargo test -p tl-engine
cargo test -p tl-storage
make backend-test-db
```

Run storage/DB tests only when `source_label_policy` persistence is implemented.

## Design Checklist

- [ ] Label resolver exists.
- [ ] Label families match the design.
- [ ] Provenance propagation exists.
- [ ] Labels appear in traces.
- [ ] Classifier/LLM is advisory only.
- [ ] No verdict changes occur.

## Research Alignment

- Paper section XI: source labels and provenance.
- FIDES/CaMeL grounding: labels stay attached to data as it moves.

## Clean Architecture Gate

- Label resolution is deterministic and in-process.
- Workspace config reads go through cached providers.
- No framework-specific source type leaks into core.
- No checker enforces labels yet.

## Not Building

- Flow enforcement.
- Parameter-source enforcement.
- Cross-session memory graph.
- LLM-based authority decisions.

## Completion Statement

Phase 3 is complete when labels are resolved and propagated into trace evidence
without changing any decisions.
