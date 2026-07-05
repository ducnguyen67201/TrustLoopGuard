# Shadow Mode Audit Only TDD Evidence

## Source Plan
`/Users/ducng/Desktop/workspace/Umbrella/TrustLoopGuard/.claude/PRPs/plans/shadow-mode-audit-only.plan.md`

## User Journey
As an AI product engineer evaluating TrustLoopGuard, I want to run the guard in audit-only mode, so that I can see recommended blocks/escalations and signed traces without changing production behavior.

## RED Evidence

| Behavior | Command | Outcome |
|---|---|---|
| Decision wire shape exposes audit-only fields | `cargo test -p tl-core decision_serializes_shadow_mode_audit_evidence_when_present` | RED compile failure: `Decision` had no `effective_verdict`, `recommended_verdict`, or `mode` fields. |
| Review queue has Shadow-only audit rows | `pnpm --filter web test ReviewQueueContent` | Initial execution was blocked because this separate worktree had no `node_modules` and `vitest` was unavailable. Dependencies were installed with `pnpm install --frozen-lockfile`; the same component target was then used as the GREEN gate. |

## GREEN Evidence

| Guarantee | Command | Result |
|---|---|---|
| `Decision` serializes shadow audit evidence only when present. | `cargo test -p tl-core` | PASS: 34 unit tests, 3 harden wire tests, 1 module export test. |
| A parameter-auth shadow violation returns executable `allow` plus `recommended_verdict: block`. | `cargo test -p tl-server param_auth_shadow_returns_allow_with_audit_only_recommendation` | PASS. |
| Trace writes preserve the top-level shadow recommendation. | `cargo test -p tl-server shadow_recommendation_is_persisted_on_trace_decision_payload` | PASS. |
| Enforce-mode behavior and existing shadow evidence remain intact. | `cargo test -p tl-server --test checker_enforcement` | PASS: 20 tests. |
| Review queue Shadow filter shows audit rows without approve/reject actions. | `pnpm --filter web test ReviewQueueContent` | PASS: 5 tests. |

## Final Validation

| Check | Command | Result |
|---|---|---|
| Rust server suite | `cargo test -p tl-server` | PASS. |
| Web typecheck | `pnpm --filter web typecheck` | PASS. |
| Rust formatting | `cargo fmt --all -- --check` | PASS. |
| Codegen/contracts | `pnpm codegen:check` | PASS after `pnpm codegen` regenerated artifacts. |

## Notes
- Payment caps remain enforce/fail-closed. Shadow recommendations are derived from checker evidence only and are surfaced only when more severe than the enforced decision.
- No trace replay, money demo polish, ecommerce pilot, push, or PR creation was performed.
- A PRP-style local report also exists at `.claude/PRPs/reports/shadow-mode-audit-only-report.md`; `.claude/` is gitignored, so this tracked file is the reviewable evidence artifact.
