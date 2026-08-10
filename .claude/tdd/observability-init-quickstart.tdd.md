# Observability init quickstart

## Goal

Give a Node.js agent one obvious integration boundary: initialize observability once, run one
complete session inside `observed.run(...)`, and let the SDK correlate and flush telemetry before
the server starts assigned post-run evaluations.

## RED evidence

- `pnpm --filter @featherlane-ai/sdk exec vitest run test/observability.test.ts` failed because
  `src/observability` did not exist. Commit: `5695fcd9`.
- The Cookbook contract test failed because `runObservedAgent` did not exist. Commit in the
  Cookbook repository: `6533cc3`.

## GREEN evidence

- The focused SDK contract test passed after adding the Node-only observability entry point.
  Commit: `03ad0908`.
- The Cookbook contract test and its focused strict TypeScript check passed after replacing manual
  OpenTelemetry setup with the new entry point. Commit in the Cookbook repository: `69630ff`.
- The complete TypeScript SDK suite passed: 12 files and 125 tests.
- The SDK typecheck, build, and packed-consumer smoke test passed, including the
  `@featherlane-ai/sdk/observability` export.
- A local end-to-end run (`019fea6f-9f02-7662-958b-9e4e4337adb0`) completed capture with the Run,
  user-turn, tool-call, assistant-turn, and flush spans. No span was marked as late evidence, and
  the assigned evaluation completed on its first attempt with a passing verdict.

## Behavior covered

| Behavior | Evidence |
| --- | --- |
| Environment-based default configuration | Focused SDK test |
| Run and event correlation | Focused SDK test and local end-to-end run |
| Guard call inside an observed event | Cookbook test and local end-to-end run |
| Flush before Run finalization | Focused SDK test and captured flush span |
| Idempotent shutdown | Focused SDK test |
| Published subpath usability | Packed-consumer smoke test |

## Coverage note

The SDK package does not currently install a Vitest coverage provider or expose a coverage script,
so no percentage is claimed. Regression confidence comes from the focused public-contract test,
the full SDK suite, the packed-consumer test, and the live database-backed end-to-end run.
