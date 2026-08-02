# Automatic guard-agent Runs TDD evidence

## Source

The journeys were derived from direct integration feedback: decorating an
agent once should make its executions visible under `/runs` without adding
`client.withRun(...)` at every reply call site.

## User journeys

- As an SDK integrator, I can wrap an agent once and keep calling
  `agent.reply(...)` while Featherlane AI creates a Run carrying the configured
  agent ID.
- As a multi-turn or framework integrator, an explicit active Run is reused
  rather than nested.
- As an integrator that wants standalone traces, I can set `run: false` without
  changing reply call sites or disabling enforcement.
- As an operator, Run bookkeeping failures do not replace the guard result or
  the agent's own error.

## RED and GREEN report

| Behavior | RED evidence | GREEN evidence |
| --- | --- | --- |
| Automatic lifecycle | `pnpm --filter @featherlane-ai/sdk test -- guard.test.ts` failed because only `POST /v1/events` occurred; no Run was created or completed. | The same command passed with `POST /v1/runs`, a run-linked event, and `PATCH /v1/runs/{id}` covered. |
| Failed agent execution | The reproducer observed no Run requests when the wrapped agent threw. | The automatic Run is patched to `failed`, while the original agent error remains the caller-visible error. |
| Best-effort bookkeeping | Start and completion failure tests returned typed SDK errors instead of the guarded reply. | Both tests pass; enforcement continues after a Run start failure, and a completion failure does not hide the guarded reply. |

Checkpoint commits on `codex/automatic-guard-agent-runs` preserve the cycles:

- `2106893d` — failing automatic lifecycle specification.
- `588fe141` — minimal automatic lifecycle implementation.
- `810964c1` — failing Run transport-error specification.
- `726f1d62` — best-effort bookkeeping implementation.

## Test specification

| # | What is guaranteed | Test target | Type | Result |
| --- | --- | --- | --- | --- |
| 1 | A plain decorated reply creates one `chat_session` Run with the configured agent ID | `sdks/typescript/test/guard.test.ts` | Integration/unit | PASS |
| 2 | The output event inherits the generated `run_id` and the Run completes | `sdks/typescript/test/guard.test.ts` | Wire contract | PASS |
| 3 | An active explicit Run is reused instead of nested | `sdks/typescript/test/guard.test.ts` | Integration/unit | PASS |
| 4 | `run: false` preserves ungrouped trace behavior | `sdks/typescript/test/guard.test.ts` | Unit | PASS |
| 5 | Agent exceptions mark the Run failed and remain the primary error | `sdks/typescript/test/guard.test.ts` | Error path | PASS |
| 6 | Run start/finish transport failures do not replace enforcement results | `sdks/typescript/test/guard.test.ts` | Error path | PASS |
| 7 | The package artifact exports the updated typed decorator surface | `pnpm --filter @featherlane-ai/sdk test:package` | Package integration | PASS |

## Verification

- `pnpm --filter @featherlane-ai/sdk test`: PASS, 98 tests.
- `pnpm --filter @featherlane-ai/sdk typecheck`: PASS.
- `pnpm --filter @featherlane-ai/sdk test:package`: PASS, 339 packed files.
- `pnpm --filter web test -- lib/onboarding.test.ts components/onboarding/ConnectAgentStep.test.tsx`: PASS, 243 tests in the configured web suite.
- `pnpm --filter docs exec fumadocs-mdx && pnpm --filter docs typecheck`: PASS.

## Coverage and known gaps

`pnpm --filter @featherlane-ai/sdk exec vitest run --coverage` could not run
because the repository does not install `@vitest/coverage-v8`. The focused
lifecycle, nesting, opt-out, agent-error, Run-start-error, and Run-finish-error
branches are covered by executable tests, but no percentage is claimed.

Automatic Runs intentionally use one reply as the generic lifecycle boundary.
Framework-owned longer sessions, such as a LiveKit room, should open one
explicit scoped Run; the decorator detects and reuses it. No Rust endpoint or
wire-contract change was required.
