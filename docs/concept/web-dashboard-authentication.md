# Web Dashboard And Authentication

## Status

Implemented around a Rust-owned data boundary.

`apps/web` owns the Next.js UI and Auth.js session cookie, but it does not open a database
connection or define database tables. Durable data is owned by `tl-server` and `tl-storage`,
with Postgres access going through Diesel when the Rust server runs with the `postgres` feature.

## Authentication

The web app uses Auth.js for dashboard sessions. Staging and production expose OAuth sign-in only:

- Google, when `AUTH_GOOGLE_ID` and `AUTH_GOOGLE_SECRET` are configured
- GitHub, when `AUTH_GITHUB_ID` and `AUTH_GITHUB_SECRET` are configured

Local development also exposes the Auth.js credentials provider so developers can bootstrap
username/password users without configuring OAuth. That local credentials path delegates to:

- `POST /v1/auth/signup`
- `POST /v1/auth/login`
- `POST /v1/auth/password`

`tl-server` stores username/password accounts in the Rust-owned `users` table. The web session is
JWT-only. For credentials users, `session.user.id` carries the Rust user id returned by `tl-server`.
For OAuth users, Auth.js owns the provider login flow. After Google/GitHub succeeds, the web calls
Rust `POST /v1/identity/oauth-session` with the provider id, provider subject, and email. Rust links
that provider identity through `oauth_identities`, returns the canonical local `users.id`, and the
web stores that id in the Auth.js session.

`POST /v1/identity/oauth-session` is an internal endpoint: Rust accepts only
`Authorization: Bearer <TL_API_KEY>` on this route. User-session JWTs and
workspace runtime keys (`tl_live_...`) are rejected with `401 unauthorized`.

Required web environment variables:

- `AUTH_SECRET`
- `AUTH_URL`
- `NEXT_PUBLIC_TL_SERVER_URL`

The hosted MCP endpoint uses a separate OAuth resource-server lane. Consent
requires the member to choose a workspace and an existing registered agent.
The consent proxy calls `POST /v1/oauth/authorize` with the internal
`TL_API_KEY` and trusted forwarded user/workspace identity; Rust rejects user
JWTs, OAuth access tokens, and workspace runtime keys on that code-issuance
route before reading the forwarded identity.
Access and rotating refresh tokens retain that binding; deleting the agent or
using a legacy unbound token requires reauthorization. Access tokens are minted
for the exact `$TL_PUBLIC_URL/mcp` audience and `mcp:tools` scope. The `/mcp`
middleware rejects dashboard sessions, internal proxy keys, and workspace
runtime keys; generic `/v1` authentication rejects hosted MCP tokens.
Authorization codes and refresh tokens are hashed in durable Rust-owned
storage. See [hosted MCP access gateway](hosted-mcp-access-gateway.md).

`TL_DASHBOARD_URL` is the Rust server's public reference to this same dashboard
origin. OAuth discovery uses it for the employee consent endpoint; in production
it should be `https://app.featherlane.ai` when `AUTH_URL` has that value.

`AUTH_URL` is the canonical dashboard URL used for Auth.js redirects and OAuth callbacks. It must
point at the frontend app (`https://staging3.featherlane.ai` in staging,
`https://app.featherlane.ai` in production), not the Rust API.

Dashboard logout uses the branded `/signout` page. Direct browser visits to Auth.js'
`/api/auth/signout` endpoint are redirected back to `/signout`; POST requests still go through the
Auth.js handler so session cookies are cleared by Auth.js itself.

`NEXT_PUBLIC_TL_SERVER_URL` is the public Rust API URL for browser-safe runtime calls.
`TL_SERVER_URL` is the server-side Rust API URL used by Next API routes and Auth.js credentials
login.

Staging and production must configure at least one OAuth provider. The username/password sign-in UI,
Auth.js credentials provider, `/signup` page, same-origin `/api/signup` proxy, and Rust
`/v1/auth/*` password endpoints are disabled outside local development.

Rust enables password auth only when the environment is explicitly local (`TL_APP_ENV`, `APP_ENV`, or
`NEXT_PUBLIC_APP_ENV` set to `dev`, `development`, or `local`) or when the server is running with no
`DATABASE_URL` and no `TL_API_KEY`, which is the default local memory-only boot path. Staging,
preview, and production environment markers disable password auth.

`DATABASE_URL` belongs to `tl-server`, not `apps/web`.

New users can authenticate but cannot use dashboard-backed protected routes until the Rust-owned
`users.is_approved` field is set to `true`. When Rust returns the approval denial, the web app
routes the user to `/welcome`, where they see the waiting-for-admin approval state. Unapproved users
do not see first-run onboarding or self-service workspace creation. Once approved, a user with no
workspace is sent to first-run onboarding (`/onboarding/workspace`), and a user whose workspace has
no agents yet is sent to `/onboarding/connect`; users with an agent land on the dashboard.

Platform support access is a separate, default-deny capability. The Rust-owned
`users.is_platform_admin` field defaults to `false`. When it is `true`,
`GET /v1/team/my-workspaces` returns every active workspace and
`is_platform_admin: true`; Rust workspace member and admin gates, including workspace analytics,
also recognize the platform administrator. Owner-only destructive operations such as workspace
deletion remain membership-bound. The dashboard shows the cross-workspace switcher state only from
this response. A workspace role such as `owner` or `admin` does not grant platform access.

There is no public or dashboard endpoint that can grant this capability. An operator with write
access to the product database grants or revokes it explicitly:

```sql
UPDATE users
SET is_platform_admin = TRUE, updated_at = NOW()
WHERE LOWER(username) = LOWER('<operator-email>');
```

Use `FALSE` to revoke it. Cross-workspace listing and workspace-admin authorization emit structured
server logs with the acting user, selected workspace when applicable, and action.

## Dashboard Data Boundary

Dashboard pages call Rust API routes for runtime and workspace data. The web app must not import
Drizzle, `postgres`, or a local database client.

Rust-owned tables currently include:

- `users`
- `oauth_identities`
- `agents`
- `policies`
- `traces`
- `escalations`
- `knowledge_sources`
- `knowledge_source_files`
- workspace administration tables created by the Rust migration layer

The web app treats the `workspace` query parameter as a requested workspace, not an authority. Before
server-rendered dashboard pages or `apps/web/app/api/*` proxy routes attach `TL_API_KEY` and
`x-featherlane-ai-workspace-id`, they resolve the signed-in user's memberships through Rust
`GET /v1/team/my-workspaces`. For ordinary users, if the requested workspace is not in that
membership list, the proxy returns 403 instead of forwarding the request. Platform administrators
receive the complete active-workspace list from Rust, so the same check permits their
cross-workspace selection. When no workspace is requested, the first authorized workspace is used.

Membership selection does not grant mutation authority. Server actions that mutate Rust-owned data
resolve the signed-in user and selected membership through Rust, enforce the action's required role,
and only then attach `TL_API_KEY`, `x-featherlane-ai-user-id`, and `x-featherlane-ai-workspace-id` when forwarding the
mutation. Shared workspace configuration, including policy and knowledge-source creation, requires
an Owner/Admin role. Rust repeats the role check before durable storage is called.

## Acceptance Criteria

- A staging or production user can sign in and reach the dashboard with Google or GitHub.
- A hosted-deployment user (any app environment) must have `users.is_approved=true` before using
  protected dashboard routes.
- Username/password sign-in, sign-up, and direct Rust password endpoints are unavailable in staging
  and production.
- A local development user can sign up, sign in, and reach the dashboard with Rust-backed
  credentials.
- Anonymous users cannot access dashboard routes.
- Ordinary authenticated users cannot steer the web proxy into a workspace outside their Rust
  membership list.
- Only users with `users.is_platform_admin=true` receive cross-workspace dashboard access.
- Approved users self-serve workspace creation via first-run onboarding on every deployment.
- Dashboard policy, agent, trace, and knowledge-source data comes from `tl-server`.
- `apps/web` has no direct DB dependencies, config, schema, or client code.
- `pnpm --filter web typecheck` passes.
