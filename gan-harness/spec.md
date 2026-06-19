# gan-design spec — TrustLoopGuard dashboard, from-scratch UI redesign

> The brief IS the spec (gan-design has no planner). This file is the contract the
> Generator builds against and the Evaluator scores against. It is harness
> scaffolding, not a `docs/concept/` doc.

## Goal

Redesign the entire `apps/web` dashboard so it is **meaningful to use, super clean,
and has good flow**. Reinvent the *visual language, layout, and page composition*.
Do **not** invent a second backend or change any data contract.

## Hard constraints (a violation = automatic fail, regardless of visual score)

1. **Rust is the source of truth.** Pages keep calling the same `apps/web/app/api/*`
   proxy routes through `lib/http.ts`. No new Drizzle tables, no guardrail logic in
   web routes, no changed request/response shapes. Presentation changes only.
2. **Preserve shared contracts.** `DataTable`, `BatchActionBar`, and `http.ts` keep
   their public APIs (see `docs/concept/web-ui-conventions.md`). Restyle internals,
   do not break call sites.
3. **Verdict semantics are sacred.** `--color-allow / --color-rewrite / --color-block
   / --color-escalate` keep their meaning and stay legible in light and dark.
4. **Type-safe.** No `any`, no `unknown` leaks, no `as unknown as`, no untyped mocks.
   `tsc --noEmit` must pass. The production `next build` must pass.
5. **Surgical per page.** A page agent edits only its own route/component files and
   may add page-local components. It must NOT edit `app/globals.css`,
   `components/ui/*`, or the app shell — those are frozen by Phase 1.
6. **Accessibility & motion.** Semantic HTML, visible focus states, AA contrast,
   keyboard-navigable. Motion is transform/opacity only and respects
   `prefers-reduced-motion`.

## Design direction — "Instrument"

A calm, high-signal control surface for AI-guardrail operators. Keep the product's
technical soul (orange brand, mono as a *data* face) but fix the harshness
(0-radius everywhere, mono-as-body, flat neutrals, no designed states).

See `gan-harness/design-direction.md` for the full art direction. In one line:
**signal over decoration, two-face typography, structured depth, designed states,
clear wayfinding, functional motion.**

## Definition of done (per page)

- Uses the Phase-1 foundation only (tokens, `Button/Card/Badge`, `PageHeader`,
  `DataTable`, `EmptyState`, skeletons) — no bespoke one-off styling tokens.
- Clear hierarchy: one primary action, an eyebrow+title+description header,
  breadcrumbs on nested routes.
- Loading, empty, and error states are all present and intentional.
- Reads identically to the rest of the app (same header rhythm, spacing, surfaces).
- Weighted evaluator score ≥ 7.5 with no hard-constraint violation.
