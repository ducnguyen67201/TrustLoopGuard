# Web Dashboard And Authentication

## Status

Implemented around a Rust-owned data boundary.

`apps/web` owns the Next.js UI and Auth.js session cookie, but it does not open a database
connection or define database tables. Durable data is owned by `tl-server` and `tl-storage`,
with Postgres access going through Diesel when the Rust server runs with the `postgres` feature.

## Authentication

The web app uses the Auth.js credentials provider only. Sign-up and login are delegated to:

- `POST /v1/auth/signup`
- `POST /v1/auth/login`
- `POST /v1/auth/password`

`tl-server` stores username/password accounts in the Rust-owned `users` table. The web session is
JWT-only, and `session.user.id` carries the Rust user id returned by `tl-server`.

Required web environment variables:

- `AUTH_SECRET`
- `AUTH_URL`
- `NEXT_PUBLIC_TL_SERVER_URL`

`AUTH_URL` is the canonical dashboard URL used for Auth.js redirects and OAuth callbacks. It must
point at the frontend app (`https://staging3.gettrustloop.app` in staging,
`https://app.gettrustloop.app` in production), not the Rust API.

`NEXT_PUBLIC_TL_SERVER_URL` is the public Rust API URL for browser-safe runtime calls.
`TL_SERVER_URL` is the server-side Rust API URL used by Next API routes and Auth.js credentials
login.

`DATABASE_URL` belongs to `tl-server`, not `apps/web`.

## Dashboard Data Boundary

Dashboard pages call Rust API routes for runtime and workspace data. The web app must not import
Drizzle, `postgres`, or a local database client.

Rust-owned tables currently include:

- `users`
- `agents`
- `policies`
- `traces`
- `escalations`
- `knowledge_sources`
- `knowledge_source_files`
- workspace administration tables created by the Rust migration layer

The web app treats the `workspace` query parameter as a requested workspace, not an authority. Before
server-rendered dashboard pages or `apps/web/app/api/*` proxy routes attach `TL_API_KEY` and
`x-tlg-workspace-id`, they resolve the signed-in user's memberships through Rust
`GET /v1/team/my-workspaces`. If the requested workspace is not in that membership list, the proxy
returns 403 instead of forwarding the request. When no workspace is requested, the first membership
is used.

## Acceptance Criteria

- A user can sign up, sign in, and reach the dashboard with Rust-backed credentials.
- Anonymous users cannot access dashboard routes.
- Authenticated users cannot steer the web proxy into a workspace outside their Rust membership list.
- Dashboard policy, agent, trace, and knowledge-source data comes from `tl-server`.
- `apps/web` has no direct DB dependencies, config, schema, or client code.
- `pnpm --filter web typecheck` passes.
