# Session-scoped guard-agent Runs TDD evidence

## Source

The journey came from integration feedback: a customer should decorate an
agent once, see its guarded traces under /runs, and keep one Run alive for the
real framework session without surrounding every turn with client.withRun.

## User journeys

- A generic agent keeps the safe one-Run-per-reply default.
- A LiveKit integrator passes one lifecycle option while decorating the agent;
  output and local-tool traces reuse one live_call Run until AgentSession close.
- A tool-only LiveKit agent receives the same automatic session grouping even
  without a reply method.
- Concurrent first boundaries create one Run, and a racing close sends one
  terminal update after active boundaries settle.
- Run persistence failures are observable but never replace guard, agent, or
  tool outcomes.

## RED specification

The new tests were authored before the session controller and LiveKit helper:

- guard.test.ts imported the absent liveKitRun and GuardAgentRunWarning exports;
- sequential and concurrent replies expected one persistent run_id instead of
  the existing one-shot lifecycle;
- tool-discovery.test.ts expected tool-only LiveKit calls to enter one session
  Run;
- lifecycle tests expected close-reason mapping, idle close, duplicate close,
  lazy external ID validation, boundary retry, and warning callbacks.

The prp-implement workflow requires one consolidated validation pass after the
coherent code/docs change, so the intentionally failing suite was not executed
as a separate command. This is a workflow deviation from a literal red command,
not a claim that the pre-implementation code could pass the specification.

## Test specification

| # | Guarantee | Test target | Type |
|---|---|---|---|
| 1 | Default reply scope remains one completed Run per reply | sdks/typescript/test/guard.test.ts | Compatibility |
| 2 | Sequential session replies share one running Run | sdks/typescript/test/guard.test.ts | Lifecycle |
| 3 | Concurrent first replies issue one Run create | sdks/typescript/test/guard.test.ts | Concurrency |
| 4 | Close racing Run creation sends one terminal update after active boundaries | sdks/typescript/test/guard.test.ts | Concurrency |
| 5 | Idle and duplicate close do not create or finish extra Runs | sdks/typescript/test/guard.test.ts | Edge case |
| 6 | LiveKit close evidence maps to completed, failed, or canceled | sdks/typescript/test/guard.test.ts | Adapter contract |
| 7 | Empty lazy external IDs fail without using agentId as fallback | sdks/typescript/test/guard.test.ts | Identity safety |
| 8 | Failed session start retries on a later boundary only | sdks/typescript/test/guard.test.ts | Failure isolation |
| 9 | Agent errors remain primary and do not terminate the session Run | sdks/typescript/test/guard.test.ts | Failure isolation |
| 10 | An explicit `withRun` scope wins inside session mode | sdks/typescript/test/guard.test.ts | Compatibility |
| 11 | Session terminal update failures preserve the result and emit a warning | sdks/typescript/test/guard.test.ts | Failure isolation |
| 12 | Tool-only LiveKit activity creates and reuses the session Run | sdks/typescript/test/tool-discovery.test.ts | Adapter integration |
| 13 | A concurrent first output and first tool share one Run creation | sdks/typescript/test/tool-discovery.test.ts | Concurrency |
| 14 | A nested tool does not retry Run creation inside one failed boundary | sdks/typescript/test/tool-discovery.test.ts | Failure isolation |
| 15 | Run metadata does not copy raw input/output text | sdks/typescript/test/guard.test.ts | Privacy |

## GREEN and refactor evidence

The consolidated validation pass completed without a failed gate:

- `pnpm --filter @trustloopguard/sdk test`: 11 files and 116 tests passed.
  The 18 new session-lifecycle cases cover sequential and concurrent reuse,
  output/tool concurrency, lifecycle status mapping, explicit scope
  precedence, privacy, and failure isolation.
- `pnpm --filter @trustloopguard/sdk typecheck` and `build`: passed with zero
  TypeScript errors.
- Fumadocs generation and the docs typecheck: passed.
- `pnpm --filter @trustloopguard/sdk test:package`: packed 339 files, imported
  `guardAgent` and `liveKitRun`, and typechecked the structural LiveKit
  consumer without installing LiveKit.
- Workspace typecheck, boundary lint, Prettier, production dependency audit,
  and `git diff --check`: passed.
- `pnpm codegen:check`: 319 Rust binding tests passed and generated artifacts
  were in sync.

The refactor retained one async Run-context seam, kept a failed start promise
for the entire nested boundary, and reused one controller across output and
tool decoration. The package smoke test was extended after review because the
plan's validation expectation named `liveKitRun`, while the existing script
only asserted the older `guardAgent` export.

## Coverage boundary

These tests verify client request order and exact wire fields, including
principal.agent_id and principal.run_id. Durable trace persistence and the
existing /runs dashboard remain Rust/server integration responsibilities; no
Rust endpoint or wire contract changes in this feature.
