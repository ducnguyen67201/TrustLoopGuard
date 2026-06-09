# Phase 6 - Explainable Policy, Approvals, And Decision Evidence

Status: **planning documentation only.**

## Purpose

Make verdicts production-grade. A block or escalation must explain what rule was
violated, why, what source chain caused it, and what remediation or approval path
is available.

## Independent Ship Boundary

Phase 6 can ship by itself when:

- existing content policies remain compatible,
- new policy families can be represented,
- decision evidence is populated when available,
- approvals/escalations reuse existing infrastructure,
- LLM/classifier signals remain advisory for actions.

## Dependencies

- Phase 0 for decision evidence fields.
- Phase 1 for trace evidence.
- Phases 2-5 provide richer policy inputs but can be integrated incrementally.

## Inputs

| Input | Source | Notes |
|---|---|---|
| checker results | event checkers | deterministic findings |
| advisory signals | LLM/classifier | signal only for actions |
| policy families | `tl-policy` | content, flow, parameter, approval, memory |
| approval rules | tool metadata/policy | high-impact or ambiguous actions |
| escalation infrastructure | existing server worker | reuse, do not rebuild |

## Outputs

| Output | Consumer | Notes |
|---|---|---|
| populated `Decision` evidence | SDKs, traces, dashboard | violated rule, remediation, source chain |
| `Escalate` routing | escalation/human review | existing infra |
| policy-family parse/validation | server/CLI/dashboard | backward compatible |

## Decision Evidence

Evidence fields should include:

- `violated_rule`,
- `remediation`,
- `source_chain`,
- `risk_source`,
- `failure_mode`,
- `harm_class`,
- `constraints` for future sandbox/adapter enforcement.

Empty optional fields should not serialize noisily.

## Policy Families

| Family | Purpose |
|---|---|
| content | existing output/content policies |
| flow | source-to-sink and action-integrity rules |
| parameter_source | allowed source rules for authority params |
| approval | human/admin approval requirements |
| memory | write/retrieval memory policies |

Existing YAML policies must keep parsing and behaving as they do today.

## Composer Rules

- Deterministic ENFORCE findings can decide action verdicts.
- SHADOW findings are persisted but do not change verdicts.
- LLM/classifier signals may add evidence.
- LLM/classifier signals cannot block actions by themselves.
- Conflicting or insufficient evidence should escalate when configured.
- Worst verdict wins across enforce-enabled deterministic findings.

## Implementation Tasks

1. Extend `tl-policy` AST for policy families.
2. Preserve legacy content policy parser behavior.
3. Update decision composer for evidence and approvals.
4. Wire approval/escalation to existing escalation path.
5. Ensure `Decision` serde/codegen includes new fields.
6. Add policy validation and compatibility tests.
7. Update docs/openapi/generated artifacts when DTOs change.

## Testing Requirements

| Test | Expected Result |
|---|---|
| existing policy YAML | parses and behaves unchanged |
| new flow policy | parses and validates |
| new parameter-source policy | parses and validates |
| decision evidence empty | omitted on wire |
| decision evidence populated | serialized correctly |
| deterministic block + advisory allow | deterministic block wins |
| advisory-only action block | does not block by itself |
| escalation | existing worker payload enqueued |

Recommended commands:

```bash
cargo test -p tl-policy
cargo test -p tl-core
cargo test -p tl-engine
cargo test -p tl-server
pnpm codegen:check
pnpm test:backend
```

## Design Checklist

- [ ] Existing policies remain compatible.
- [ ] New policy families are represented.
- [ ] Decision evidence is populated.
- [ ] Blocks explain violated rule and remediation.
- [ ] Escalations route to existing infrastructure.
- [ ] LLM/classifier remains signal-only for actions.

## Research Alignment

- Paper section XV: policy as a reasoning layer.
- Paper section V.2: conflicting evidence should escalate.
- Paper section XIX: block without diagnosis is not production-grade.

## Clean Architecture Gate

- Policy parsing stays in `tl-policy`.
- Decision composition stays in `tl-engine`.
- HTTP/escalation orchestration stays in `tl-server`.
- Public DTOs stay in `tl-core`.
- No dashboard-owned runtime policy logic.

## Not Building

- A new human-review product surface.
- A separate workflow/orchestration system.
- LLM-as-boundary behavior for actions.
- Full trace graph analysis.

## Completion Statement

Phase 6 is complete when verdicts can carry structured evidence and approvals
while preserving existing policy behavior and current escalation infrastructure.
