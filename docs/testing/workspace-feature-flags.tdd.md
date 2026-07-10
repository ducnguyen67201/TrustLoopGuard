# Workspace feature flags: TDD evidence

## Source and user journeys

The journeys were derived from the requested workspace-level rollout controls:

- As an operator, I want Attacks and Knowledge sources disabled by default so unreleased features are not exposed.
- As an operator, I want each feature enabled independently per workspace so rollout can be gradual.
- As a user, I should receive a not-found response when directly opening a disabled feature page.

## Task report

| Behavior | RED evidence | GREEN evidence | Guarantee |
|---|---|---|---|
| Persist default-off flags and return them with workspace membership | `cargo test -p tl-storage --features postgres-it --test team_repo --no-run` failed with `no field is_knowledge_base_enabled` and `no field is_attacks_enabled` | `cargo test -p tl-storage --features postgres-it --test team_repo` passed 1 test | New and listed workspaces expose both persisted flags as `false` by default. |
| Evaluate flags independently | `pnpm --filter web exec vitest run lib/workspace-features.test.ts` failed because `workspace-features` did not exist | Focused Vitest run passed | Either feature can be enabled without enabling the other. |
| Reject direct access while disabled | `pnpm --filter web exec vitest run app/workspace-feature-pages.test.tsx` failed because both page promises resolved | Focused Vitest run passed | Disabled Attacks and Knowledge sources pages call `notFound()`; enabled pages render. |
| Hide disabled navigation | `pnpm --filter web exec vitest run components/app-sidebar.test.ts` failed because `getVisibleNavGroups` did not exist | Focused Vitest run passed | Sidebar groups omit disabled feature links and retain independently enabled links. |

## Test specification

| # | What is guaranteed | Test target | Type | Result |
|---|---|---|---|---|
| 1 | Created and queried workspace records default both feature flags off | `crates/tl-storage/tests/team_repo.rs` | Postgres integration | PASS |
| 2 | Feature evaluation is independent | `apps/web/lib/workspace-features.test.ts` | Unit | PASS |
| 3 | Direct disabled routes, including new knowledge-source creation, return not found and enabled routes render | `apps/web/app/workspace-feature-pages.test.tsx` | Component/page | PASS |
| 4 | Sidebar navigation hides disabled links | `apps/web/components/app-sidebar.test.ts` | Unit/component contract | PASS |
| 5 | Existing web behavior remains intact | `pnpm --filter web test` | Web suite | PASS, 215 tests; the final focused gate run passed 10 tests |
| 6 | Rust server behavior remains intact | `cargo test -p tl-server` | Rust unit/integration | PASS, 192 unit tests plus integration suites |

## Coverage and known gaps

`pnpm --filter web test:coverage` passed. The new `lib/workspace-features.ts` module has 100% statement, branch, function, and line coverage. Repository-wide frontend coverage is 45.64% statements/lines, below the skill's 80% target because the existing coverage configuration includes many untested application pages; this change does not attempt that unrelated repository-wide remediation. The affected page gates and navigation filter have focused tests.

## Merge evidence

- RED checkpoint: `27f81ae0 test: add workspace feature flag coverage`
- RED checkpoint: `cf106f1e test: cover disabled workspace feature routes`
- GREEN implementation and verification are captured by this report and the final implementation commit.
