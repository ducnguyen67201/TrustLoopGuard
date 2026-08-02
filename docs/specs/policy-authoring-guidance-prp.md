# PRP Plan: Policy Authoring Guidance

## Goal

Help first-time dashboard users understand how policies work, what each creation
field controls, and where to find the full policy-authoring guide.

## Assumptions

- “The popup” means both policy-family dialogs opened from **Policies → New
  policy**.
- `docs/policies/README.md` remains the canonical README; creating a second
  policy guide would duplicate the source of truth.
- This is a dashboard guidance and documentation change only. Rust continues to
  own policy contracts, validation, persistence, and runtime evaluation.
- The dashboard should link to the same-origin policy-authoring docs route so
  hosted and local users have one stable destination.

## Scope

- Expand the canonical policy README with a dashboard workflow and field guide
  for protection and financial policies.
- Add a discoverable policy-guide link to the policy creation flow.
- Add a concise information tooltip for every field or grouped control in the
  financial policy dialog so guidance does not make rows uneven.
- Preserve and verify the existing protection-rule field help.
- Add component tests for the guide link and the new financial field guidance.

## Out of Scope

- Policy API, Rust wire-contract, storage, parser, or runtime behavior changes.
- New policy families, fields, validation rules, or AI-generation behavior.
- Reworking the form layout or introducing a new shared UI primitive.
- Duplicating the canonical policy guide under `docs/concept/`.

## Codebase Findings

| Area | Files | Pattern to Follow |
| --- | --- | --- |
| Policy create entry point | `apps/web/components/workspace/PolicyCreateDialog.tsx` | Keep one family chooser and route each family to its existing editor. |
| Protection policy editor | `apps/web/components/policies/PolicyEditorDialog.tsx` | Preserve its existing `Field` helper, hint copy, and `InfoHint` terminology. |
| Financial policy editor | `apps/web/components/workspace/FinancialSpendingControlsCard.tsx` | Extend the local `Field`, `MoneyField`, and `ActionField` helpers with the shared `InfoHint` primitive rather than adding a form framework. |
| Policy UI tests | `apps/web/components/workspace/PolicyCreateDialog.test.tsx`, `apps/web/components/workspace/FinancialSpendingControlsCard.test.tsx` | Assert user-visible labels, descriptions, links, and behavior with Testing Library. |
| Canonical user guide | `docs/policies/README.md` | Extend the existing quick-start guide; link to detailed references rather than duplicating runtime concepts. |
| Docs-site bridge | `apps/docs/content/docs/guides/policy-authoring.mdx` | Keep the same-origin guide route and point it to the canonical README. |
| Shared help convention | `apps/web/components/ui/info-hint.tsx`, `docs/concept/web-ui-conventions.md` | Use the accessible hover/focus tooltip for compact field guidance in this dense domain form. |

## Implementation Steps

1. Add component assertions for the policy-guide link and representative
   field-by-field financial guidance, then run the focused tests and confirm
   they fail for the missing UI.
2. Add the guide link and financial information tooltips with the smallest
   changes to the existing dialog components, then rerun the same tests until
   they pass.
3. Expand `docs/policies/README.md` with the dashboard workflow, protection
   fields, financial fields, safe rollout advice, and links to canonical
   references.
4. Run focused component tests, the web typecheck, frontend coverage, formatting
   checks for changed files, and inspect the final diff.
5. Commit the scoped changes, push the feature branch, and open a PR to `main`
   using `.github/pull_request_template.md`.

## Files Likely To Change

| File | Action | Reason |
| --- | --- | --- |
| `docs/specs/policy-authoring-guidance-prp.md` | Add | Record the implementation-ready PRP requested by the user. |
| `docs/policies/README.md` | Update | Add the canonical dashboard policy-authoring field guide. |
| `apps/docs/content/docs/guides/policy-authoring.mdx` | Update | Make the same-origin docs route link to the canonical README. |
| `apps/web/components/workspace/PolicyCreateDialog.tsx` | Update | Surface the full guide from the creation flow. |
| `apps/web/components/workspace/PolicyCreateDialog.test.tsx` | Update | Cover the guide link as a user-visible contract. |
| `apps/web/components/workspace/FinancialSpendingControlsCard.tsx` | Update | Explain every financial policy field or grouped control. |
| `apps/web/components/workspace/FinancialSpendingControlsCard.test.tsx` | Update | Cover guidance for action and LLM-usage variants. |

## Tests And Verification

- `pnpm --filter web test -- PolicyCreateDialog.test.tsx FinancialSpendingControlsCard.test.tsx`
- `pnpm --filter web typecheck`
- `pnpm --filter web test:coverage`
- `pnpm exec prettier --check docs/specs/policy-authoring-guidance-prp.md docs/policies/README.md apps/docs/content/docs/guides/policy-authoring.mdx apps/web/components/workspace/PolicyCreateDialog.tsx apps/web/components/workspace/PolicyCreateDialog.test.tsx apps/web/components/workspace/FinancialSpendingControlsCard.tsx apps/web/components/workspace/FinancialSpendingControlsCard.test.tsx`
- `git diff --check`

## Risks

- Too much copy can make a dense form harder to scan. Keep tooltips short and
  put longer explanations in the README.
- Financial terms can drift from runtime behavior. Base copy on
  `docs/concept/financial-authorization.md` and the current form payload.
- A new guide could duplicate existing docs. Extend `docs/policies/README.md`
  and link to canonical concept/reference docs instead.

## Ready To Implement

Yes. The affected UI, documentation source of truth, test harness, and ownership
boundaries are identified, and no backend contract change is required.
