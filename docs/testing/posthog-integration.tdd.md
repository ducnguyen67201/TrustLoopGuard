# PostHog integration TDD evidence

## Source and journeys

No implementation plan file was supplied. The journeys were derived from the requested PostHog setup:

- As a marketing visitor, I want page and funnel interactions captured so product interest can be measured.
- As a dashboard user, I want my activity joined to my authenticated identity so usage can be understood across sessions.
- As an operator, I want analytics to remain optional so CI and unconfigured deployments continue to work.
- As a signed-out user, I want the previous account identity cleared so events are never attributed to the wrong account.

## Task report

### Client initialization

- RED: `pnpm exec tsx --test apps/marketing/lib/posthog.test.ts` failed with `ERR_MODULE_NOT_FOUND` for `apps/marketing/lib/posthog`.
- RED: `pnpm --filter web test -- lib/posthog.test.ts` ran the existing suite and failed to resolve `apps/web/lib/posthog.ts`.
- GREEN: `pnpm exec tsx --test apps/marketing/lib/posthog.test.ts` passed 5 tests.
- GREEN: `pnpm --filter web exec vitest run lib/posthog.test.ts` passed 7 tests.
- Guarantee: each app initializes with the configured token/host and PostHog's `2026-05-30` defaults, registers its `app_surface`, and remains disabled without a token.

### Marketing dual dispatch

- RED: the integration test failed with `TypeError: client.capture is not a function`, proving `trackMarketingEvent` had not dispatched through the supplied PostHog client.
- GREEN: the same test passed after the typed event entry point dispatched to both GTM and PostHog.
- Guarantee: existing marketing event names and properties reach both analytics destinations once per interaction.

### Disabled marketing path

- RED: the unloaded-client test observed an unexpected `landing_cta_click` PostHog capture.
- GREEN: the same test passed after capture became conditional on the PostHog singleton being loaded.
- Guarantee: GTM continues working when PostHog is unconfigured, without calling an uninitialized SDK.

### Dashboard identity lifecycle

- RED: the uninitialized-client tests observed calls to `get_distinct_id` and `reset` when PostHog had not loaded.
- GREEN: focused Vitest coverage verifies identify, same-user deduplication, reset behavior, and no-op behavior when the SDK is uninitialized.
- Guarantee: dashboard users are identified by stable user ID with name/email properties, repeated page rendering does not emit redundant identify calls for the same ID, sign-out resets browser identity, and unconfigured deployments do not use identity APIs.

## Test specification

| # | What is guaranteed | Test target | Type | Result | Evidence |
|---|---|---|---|---|---|
| 1 | Marketing initializes with current PostHog defaults and `app_surface=marketing` | `apps/marketing/lib/posthog.test.ts` | Unit | PASS | `pnpm exec tsx --test apps/marketing/lib/posthog.test.ts` |
| 2 | Missing marketing token disables SDK initialization | `apps/marketing/lib/posthog.test.ts` | Unit | PASS | Same command, 5/5 tests |
| 3 | One marketing interaction reaches GTM and PostHog | `apps/marketing/lib/posthog.test.ts` | Integration | PASS | Same command, 5/5 tests |
| 4 | An unloaded PostHog client is not captured to | `apps/marketing/lib/posthog.test.ts` | Unit | PASS | Same command, 5/5 tests |
| 5 | Dashboard initializes with current defaults and `app_surface=dashboard` | `apps/web/lib/posthog.test.ts` | Unit | PASS | `pnpm --filter web exec vitest run lib/posthog.test.ts` |
| 6 | Dashboard identifies a new user and deduplicates the current user | `apps/web/lib/posthog.test.ts` | Unit | PASS | Same command, 7/7 tests |
| 7 | Dashboard identity resets on sign-out and remains unused when uninitialized | `apps/web/lib/posthog.test.ts` | Unit | PASS | Same command, 7/7 tests |
| 8 | Both Next.js apps compile with the local PostHog configuration | Next.js production builds | Integration | PASS | `pnpm --filter marketing build`; `pnpm --filter web exec next build` |
| 9 | The supplied project token is accepted by PostHog ingestion | PostHog capture API | Integration | PASS | `posthog_installation_verified` returned `{"status":"Ok"}`, HTTP 200 |

## Coverage and regression evidence

- Marketing: `pnpm exec tsx --test --experimental-test-coverage apps/marketing/lib/posthog.test.ts` reported 100% lines/functions and 83.33% branches across `gtm.ts` and `posthog.ts`.
- Dashboard: `pnpm --filter web exec vitest run --coverage lib/posthog.test.ts` reported 100% statements/branches/functions/lines for `apps/web/lib/posthog.ts`. The repository-wide percentage shown by this focused command is not meaningful because the web coverage configuration includes every source file even when only one test target is loaded.
- Dashboard regression: `pnpm --filter web test` passed 55 files and 237 tests.
- Marketing regression: `pnpm test:marketing-use-cases` passed 11 tests; the PostHog suite passed 5 tests.
- Type checks: `pnpm --filter marketing typecheck` and `SKIP_ENV_VALIDATION=true pnpm --filter web typecheck` passed.
- Builds: marketing generated 20 routes; web generated 65 routes. Both compiled and completed TypeScript/page generation successfully.

## Known gaps and merge evidence

- A real-browser network assertion could not run because the local Chrome profile does not have the Playwright extension. PostHog ingestion was verified directly with a non-PII event and HTTP 200 response instead.
- The public project token can ingest events but cannot create or query PostHog dashboards. The dashboard recipe is documented in `docs/concept/product-analytics.md`; applying it remotely requires a separately authorized PostHog management credential.
- RED checkpoints are preserved in commits `39dc9955`, `6db6f533`, `1d65871d`, and `345a2de8`. The GREEN implementation commit follows these checkpoints on the same task branch.
