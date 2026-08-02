# SDK package-first integration TDD evidence

## Source

The user journeys were derived from direct integration feedback: customers
should install one published SDK package and decorate the agent object once,
without cloning Featherlane AI, changing every reply call site, or adding
monitoring scaffolding.

## User journeys

- As a TypeScript developer, I can install `@featherlane-ai/sdk`, decorate an
  agent at construction, and keep calling `agent.reply(...)` normally.
- As a developer, I retain the decorated agent's other properties, methods, and
  typed `reply()` arguments.
- As a Python developer, I can apply one decorator to an async reply function.
- As a new customer, I see package-first onboarding instead of local repository,
  proxy, run, and callback setup.

## RED and GREEN report

| Behavior | RED evidence | GREEN evidence |
| --- | --- | --- |
| TypeScript function wrapper | `pnpm test -- guard.test.ts` ran the new tests and failed five cases with `protect.wrap is not a function`. | The lower-level wrapper remains compatible in the full SDK suite. |
| TypeScript agent decorator | The new agent-object tests failed four cases with `guardAgent is not a function`. A later private-field regression test failed because untouched methods received the proxy as `this`. | `pnpm test` passed 77 tests, including unchanged `agent.reply(...)` calls, private-backed interface preservation, transformed output, and fail-closed behavior. |
| Environment-only SDK configuration | The new test reached `http://127.0.0.1:8080/v1/events` instead of `FEATHERLANE_AI_URL`, then showed the documented `FEATHERLANE_AI_API_KEY` was not loaded. | The TypeScript SDK tests passed with both environment variables applied to the request. |
| Python decorator | `PYTHONPATH=src python3 -m pytest tests/test_guard.py -q` failed collection because `guarded` was not exported. | The full Python SDK suite passed 70 tests. |
| Concise dashboard onboarding | The first package-first assertions failed because the snippet still imported `Client`, used `withRun`, and required branch callbacks. The structural-decorator assertions later failed three cases while onboarding still showed `.wrap()`. | The web suite passed 225 tests, including the `guardAgent(...)` onboarding unit and component contracts. |
| Installable npm artifact | Importing the packed artifact failed with `ERR_MODULE_NOT_FOUND` because emitted ESM used extensionless relative imports. The first dry run also contained 969 files, including source and generated runtime modules. | `test:package` now packs and imports the exact artifact in Node, compiles a TypeScript consumer, verifies `guardAgent`, excludes source/generated runtime JavaScript, and passes with 329 files. |

The worktree was on a detached HEAD, so RED/GREEN checkpoint commits were not
created. The evidence is preserved here instead.

## Test specification

| # | What is guaranteed | Test target | Type | Result |
| --- | --- | --- | --- | --- |
| 1 | `guardAgent(agent, options)` returns the same agent type and preserves `agent.reply(...)` call sites | `sdks/typescript/test/guard.test.ts` and SDK typecheck | Unit/compile-time | PASS |
| 2 | The decorated `reply()` delegates to the original agent and submits its returned string | `sdks/typescript/test/guard.test.ts` | Unit/integration | PASS |
| 3 | Other agent members and additional reply arguments remain usable | `sdks/typescript/test/guard.test.ts` | Unit | PASS |
| 4 | Featherlane AI transformed output is returned through the same `reply()` method | `sdks/typescript/test/guard.test.ts` | Unit | PASS |
| 5 | The root agent decorator fails closed on SDK transport errors by default | `sdks/typescript/test/guard.test.ts` | Unit | PASS |
| 6 | Existing explicit `guard()` and lower-level `.wrap()` behavior remain compatible | Full TypeScript and Python SDK suites | Regression | PASS |
| 7 | `FEATHERLANE_AI_URL` and `FEATHERLANE_AI_API_KEY` configure the factory without constructing a client | `sdks/typescript/test/guard.test.ts` | Unit | PASS |
| 8 | Dashboard snippets lead with `guardAgent(...)`, retain `agent.reply(...)`, and omit `Client`/`withRun` | Web onboarding tests | Component/unit | PASS |
| 9 | The exact npm tarball imports in Node and exports `guardAgent` | `sdks/typescript/scripts/package-smoke.mjs` | Package integration | PASS |
| 10 | Published files exclude TypeScript source and generated runtime JavaScript | `sdks/typescript/scripts/package-smoke.mjs` | Package contract | PASS |
| 11 | The output wrapper submits the draft but does not include the raw user message text by default | `sdks/typescript/test/guard.test.ts` | Wire contract | PASS |
| 12 | A TypeScript consumer can resolve the package declaration graph from the tarball | `sdks/typescript/scripts/package-smoke.mjs` | Package integration | PASS |

## Verification

- `pnpm test && pnpm typecheck && pnpm build` in `sdks/typescript`: PASS, 77 tests.
- `PYTHONPATH=src python3 -m pytest -q` in `sdks/python`: PASS, 70 tests.
- `pnpm test -- lib/onboarding.test.ts components/onboarding/ConnectAgentStep.test.tsx && pnpm typecheck` in `apps/web`: PASS, 225 tests.
- `pnpm typecheck && pnpm build` in `apps/marketing`: PASS.
- `pnpm typecheck && pnpm build` in `apps/docs`: PASS.
- `pnpm --filter @featherlane-ai/sdk test:package`: PASS, 329 packed files,
  205052 unpacked bytes.
- `cargo test -p tl-codegen`: PASS, including TypeScript import-normalization
  tests.

## Coverage and known gaps

The repository does not currently install `@vitest/coverage-v8` or
`pytest-cov`, so coverage commands could not run. The attempted commands failed
only because those coverage plugins were absent. Runtime tests, type checks,
and production builds passed.

The original TypeScript root decorator targeted agents exposing
`reply(message: string, ...args): Promise<string>`. It now also discovers
supported local tool registries; provider-hosted and hidden execution still
requires a host adapter or explicit typed helper.

The Python decorator intentionally supports async reply functions only. Sync
callers retain the existing explicit `guard()` API.

The currently published npm release predates these package fixes. The package
is prepared as `0.0.7`; an `sdk-v0.0.7` release is required before customers
receive the working `guardAgent` artifact.
