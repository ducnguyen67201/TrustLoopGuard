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
- `NEXT_PUBLIC_TL_SERVER_URL`

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

The web app derives its active workspace from the `workspace` query parameter and sends the
resolved workspace id as `x-tlg-workspace-id` to Rust APIs.

## Acceptance Criteria

- A user can sign up, sign in, and reach the dashboard with Rust-backed credentials.
- Anonymous users cannot access dashboard routes.
- Dashboard policy, agent, trace, and knowledge-source data comes from `tl-server`.
- `apps/web` has no direct DB dependencies, config, schema, or client code.
- `pnpm --filter web typecheck` passes.
