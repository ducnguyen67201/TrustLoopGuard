# Marketing demo header link TDD evidence

## Source and journey

No implementation plan was supplied. The journey was derived from the requested marketing-header update:

- As a visitor, I want a Demo link in the marketing header so I can open the live refund demo from any marketing page.

## Task report

- RED: `pnpm test:marketing-nav` ran the new navigation contract and failed because neither header layout linked to `/demo`.
- GREEN: the same command passed after adding the desktop navigation link and the compact header action.
- Responsive RED: a 1000px Chrome capture showed the six-item desktop navigation wrapping after it activated at 940px.
- Responsive GREEN: the full navigation now activates at 1100px; the 1000px layout shows one uncluttered compact Demo action.
- Guarantee: the full header labels `/demo` as `Demo`, while the compact action remains available below the full-navigation breakpoint.

## Test specification

| # | What is guaranteed | Test target | Type | Result | Evidence |
|---|---|---|---|---|---|
| 1 | The full marketing navigation links `Demo` to `/demo` | `apps/marketing/components/nav.test.ts` | Component contract | PASS | `pnpm test:marketing-nav` |
| 2 | Compact header layouts retain a `/demo` action below 1100px | `apps/marketing/components/nav.test.ts` | Responsive component contract | PASS | `pnpm test:marketing-nav` and 1000px Chrome capture |
| 3 | Existing use-case navigation behavior remains intact | `apps/marketing/app/use-cases/use-cases.test.ts` | Regression | PASS | `pnpm test:marketing-use-cases` (11/11) |
| 4 | The marketing application type-checks and produces a production build | Marketing package | Integration | PASS | `pnpm --filter marketing typecheck`; `pnpm --filter marketing build` |

## Coverage and known gaps

- The change adds declarative navigation markup and no executable branch logic. The focused contract covers both responsive render paths introduced by the change.
- The marketing package has no committed browser E2E harness. Headless Chrome at 1000px supplied focused visual evidence for the responsive breakpoint.

## Merge evidence

- RED checkpoint: commit `8bb72c15`.
- The GREEN implementation commit follows that checkpoint on `codex/add-demo-header-link`.
- Responsive breakpoint RED checkpoint: commit `647f92d8`.
