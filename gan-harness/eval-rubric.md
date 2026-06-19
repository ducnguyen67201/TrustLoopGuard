# gan-design eval rubric — reframed for refinement, not originality

The stock gan-design rubric optimizes "would this win a design award" (originality
0.30). That fights this repo's Surgical / Simplicity / one-design-system rules and
the user's actual goal. We reweight toward **clarity, consistency, and flow** and
add a hard gate.

## Hard gate (binary — fail ⇒ score capped at 3, sent back)

- [ ] No data-contract / proxy-route change; still calls `apps/web/app/api/*`.
- [ ] `DataTable` / `BatchActionBar` / `http.ts` public APIs intact.
- [ ] Verdict colors keep their meaning; legible in light + dark.
- [ ] No `any` / `unknown` leak / double assertion; `tsc --noEmit` clean for the file.
- [ ] Page edited only its own files; shared foundation untouched.

## Weighted score (1–10 each)

### Clarity & meaning — weight 0.30
Does the page make its purpose and the user's next action obvious? Is hierarchy
driven by scale/weight/color, not uniform emphasis? Is copy specific (real empty
states, real labels) rather than placeholder?

### Consistency with the system — weight 0.30
Header rhythm, spacing scale, surfaces, radius, typography (Inter for UI, mono for
data) match the foundation and every other page. No bespoke spacing/color. A user
moving between pages should feel one product.

### Flow — weight 0.25
Can the user start→finish the page's core task without dead ends? Wayfinding
(breadcrumbs, back paths), loading/empty/error states, primary action placement,
form feedback. Nested routes can get back to their parent.

### Craft — weight 0.15
Designed hover/focus/active states, aligned grids, stable numerics (`tabular-nums`),
smooth compositor-only motion, responsive at 360/768/1024/1440, no overflow.

## Scoring

- weighted = Σ(score × weight); pass threshold **7.5**.
- If a finding is "nice to have," it does not block; if it breaks flow or
  consistency, it does.
- Evaluator critiques only — it never edits. Generator fixes and resubmits.
