# Marketing demo header link TDD evidence

## Source and journey

No implementation plan was supplied. The journey was derived from the requested marketing-header update:

- As a visitor, I want a Demo link in the marketing header so I can open the live refund demo from any marketing page.

## Task report

- RED: `pnpm test:marketing-nav` ran the new navigation contract and failed because neither header layout linked to `/demo`.
- GREEN: the same command passed after adding the desktop navigation link and the compact header action.
- Guarantee: the full header labels `/demo` as `Demo`, while the compact action remains available below the full-navigation breakpoint.

## Test specification

| # | What is guaranteed | Test target | Type | Result | Evidence |
|---|---|---|---|---|---|
| 1 | The full marketing navigation links `Demo` to `/demo` | `apps/marketing/components/nav.test.ts` | Component contract | PASS | `pnpm test:marketing-nav` |
| 2 | Compact header layouts retain a `/demo` action below 940px | `apps/marketing/components/nav.test.ts` | Responsive component contract | PASS | `pnpm test:marketing-nav` |
| 3 | Existing use-case navigation behavior remains intact | `apps/marketing/app/use-cases/use-cases.test.ts` | Regression | PASS | `pnpm test:marketing-use-cases` (11/11) |
| 4 | The marketing application type-checks and produces a production build | Marketing package | Integration | PASS | `pnpm --filter marketing typecheck`; `pnpm --filter marketing build` |

## Coverage and known gaps

- The change adds declarative navigation markup and no executable branch logic. The focused contract covers both responsive render paths introduced by the change.
- No browser E2E was added because the repository's marketing package does not have a component/E2E harness; the Next.js production build verifies route and component integration.

## Merge evidence

- RED checkpoint: commit `8bb72c15`.
- The GREEN implementation commit follows that checkpoint on `codex/add-demo-header-link`.
