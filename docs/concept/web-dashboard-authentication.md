# Web dashboard and authentication spec

## Status

Partially implemented.

`apps/web` has the start of the surface area: an Auth.js route, Drizzle auth tables,
middleware that protects `/dashboard`, a `/signin` page, and a `/dashboard` page.
The auth path now supports OAuth-only sign-in with Google and GitHub when the
corresponding provider credentials are configured. The dashboard is still a placeholder:

- `apps/web/app/(dashboard)/dashboard/page.tsx` only displays the current session email.

This spec defines the first complete version of the dashboard around that OAuth-only model.

## Goals

- Let a user sign in to the web app with Google or GitHub.
- Protect dashboard routes from anonymous access.
- Show useful TrustLoopGuard operational state after sign-in.
- Keep `apps/web` as the setup location for the dashboard and auth UI.
- Reuse generated/shared TrustLoopGuard contracts instead of duplicating request and response shapes.

## Non-goals

- Multi-tenant billing, invitations, or organization administration.
- Email/password accounts or local account creation.
- A full decision-log analytics product.
- Editing generated SDK or OpenAPI output by hand.
- Committing local scratch specs outside `docs/concept`.

## Authentication

Use Auth.js in `apps/web/auth.ts` with Drizzle-backed persistence.
Doppler is the source of truth for runtime environment values. Local development
uses project `trustloopguard`, config `dev`, linked by `doppler.yaml`; developers
authenticate to Doppler with GitHub before running `doppler setup`.

Provider requirements:

- Google OAuth when `AUTH_GOOGLE_ID` and `AUTH_GOOGLE_SECRET` are present.
- GitHub OAuth when `AUTH_GITHUB_ID` and `AUTH_GITHUB_SECRET` are present.
- No email/password provider.
- No local account creation flow.
- No anonymous dashboard access.

Required routes:

- `/signin`: provider buttons and form states for configured providers.
- `/api/auth/[...nextauth]`: Auth.js handlers.
- `/dashboard`: protected landing page after sign-in.

Required behavior:

- Unauthenticated requests to `/dashboard/*` redirect to `/signin`.
- Signed-in users can sign out from the dashboard shell.
- Session callbacks expose `session.user.id`.
- Missing provider configuration should fail closed with a clear sign-in page state.

Required environment variables:

- `DATABASE_URL`
- `AUTH_SECRET`
- `AUTH_GOOGLE_ID`, optional
- `AUTH_GOOGLE_SECRET`, optional
- `AUTH_GITHUB_ID`, optional
- `AUTH_GITHUB_SECRET`, optional

These values should be stored in Doppler under `trustloopguard/dev`, not committed
to `.env` files.

The schema may keep `auth_users.password_hash` for now, but the web app must not expose an
email/password registration or login path.

## Dashboard

The first dashboard should answer three questions for an operator:

1. Is the guardrail server reachable?
2. What policies are active?
3. What recent decisions need attention?

Initial dashboard sections:

- System status: `tl-server` URL, health check state, last checked time.
- Decision summary: counts by verdict, recent block/escalate decisions, latency highlights.
- Policy summary: enabled policy count, recently changed policies, validation errors.
- User/account panel: current signed-in user and sign-out action.

The dashboard should link to existing surfaces instead of duplicating them:

- Playground remains the place to submit ad hoc guard checks.
- Policy Manager remains the place to author and validate policies.
- Dashboard summarizes and deep-links into those workflows.

## Shared resources and generated contracts

Do not hand-roll API types in the dashboard. Prefer shared/generated resources:

- Use `@trustloopguard/sdk` from the workspace for client-facing TrustLoopGuard types.
- Use generated OpenAPI/JSON schema artifacts when validating policy or decision payloads.
- Keep generated files generated; if contracts drift, update Rust source types and run codegen.

## Data model

Auth tables live in `apps/web/lib/db/schema/auth.ts` and use the `auth_` prefix.

Dashboard data should come from existing TrustLoopGuard APIs first. Add web-owned tables only for
web-only state, such as user preferences. Decision, policy, and replay data should stay owned by
the runtime/server side.

## Acceptance criteria

- Configured Google and GitHub providers appear on `/signin`.
- A user can sign in and reach `/dashboard`.
- Anonymous users cannot access `/dashboard` or nested dashboard routes.
- The dashboard shows server health and at least one policy/decision summary from live APIs.
- `pnpm --filter web typecheck` passes.
- No generated contract files are edited by hand.
