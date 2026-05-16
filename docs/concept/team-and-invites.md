# Team & invites

How users join a workspace.

> Terms: [Workspace member](glossary.md#workspace-member), [Workspace invite](glossary.md#workspace-invite), [Workspace role](glossary.md#workspace-role).

## Ownership

Rust owns the durable state. Tables, repository, and HTTP endpoints all live in the Rust stack:

- Tables: `workspace_members`, `workspace_invites` (migration `00000000000006_workspace_admin`).
- Repository: `crates/tl-storage/src/team_repo.rs`.
- HTTP handlers: `crates/tl-server/src/team.rs`.
- Wire types: `crates/tl-core/src/team.rs`.

The dashboard is a same-origin proxy. `apps/web/app/api/team/*` and `apps/web/app/api/invites/[id]/lookup` forward to the Rust endpoints with the active workspace's id in `X-TLG-Workspace-Id`. The dashboard never writes to either table directly.

## Roles

`workspace_role` is one of `owner | admin | editor | viewer`. The enum is defined in the migration and mirrored by `WorkspaceRole` in `tl-core`. Membership in a workspace's parent organization is tracked separately in `organization_members` (and an accepted invite atomically inserts that org membership too, so the user can see the workspace at all).

## Invite lifecycle

```text
+----------+         +----------+         +----------+
| (none)   | create  | pending  | accept  | accepted |
|          | ------> |          | ------> |          |
+----------+         +----------+         +----------+
                          |
                          | revoke (admin) | expire (TTL)
                          v
                     +----------+
                     | revoked  |
                     | expired  |
                     +----------+
```

- **Token shape.** The invite `id` doubles as the bearer token: an opaque 32-byte URL-safe random string generated server-side in `team_repo::generate_token`. The dashboard surfaces it inside an `/invite/accept?token=…` URL.
- **TTL.** 7 days, fixed (`INVITE_TTL_DAYS` in `team_repo.rs`). A pending invite past its `expires_at` is treated as expired on lookup and is transitioned to `expired` on the next accept attempt.
- **Uniqueness.** At most one `pending` invite per `(workspace_id, email)`. Re-inviting the same address while one is outstanding returns 409 — the admin should copy the existing link or revoke first.
- **Single use.** Acceptance is transactional: the row flips to `accepted`, a `workspace_members` row is upserted, and the corresponding `organization_members` row is upserted (as `member`) in the same transaction. A repeated accept is rejected with 409.

## Endpoints

All four authoring endpoints sit behind the existing shared-bearer middleware. The lookup endpoint is intentionally public — see "Public lookup" below.

| Method | Path | Body | Returns |
|---|---|---|---|
| `GET`    | `/v1/team/members`         | — | `MemberListResponse` |
| `GET`    | `/v1/team/invites`         | — | `InviteListResponse` (pending only) |
| `POST`   | `/v1/team/invites`         | `{ email, role }` | `CreateInviteResponse { invite, accept_path }` |
| `DELETE` | `/v1/team/invites/:id`     | — | 204 |
| `GET`    | `/v1/team/my-workspaces`   | — | `MyWorkspacesResponse` *(user-scoped, auto-binds pending invites)* |
| `GET`    | `/v1/invites/:id/lookup`   | — | `InviteLookupResponse` *(public)* |

Workspace context is always read from `X-TLG-Workspace-Id`. The optional `X-TLG-User-Id` header (UUID) is captured on `POST /v1/team/invites` and persisted to `invited_by_user_id` so the audit trail survives.

`GET /v1/team/my-workspaces` is user-scoped instead — it reads `X-TLG-User-Id` (required, UUID) plus `X-TLG-User-Email`. When the email is present, the server bulk-accepts any pending invite addressed to it *before* querying memberships. This is the auto-bind mechanism: a user invited after they've already signed up sees the new workspace on their next page load without clicking the accept link.

## Enforcement

The dashboard refuses to render when the signed-in user has zero memberships.

- **Server-side**: `getDashboardShell` (in `apps/web/lib/server/dashboard-data.ts`) calls `/v1/team/my-workspaces` first, and `redirect('/welcome')` if the list is empty.
- **Middleware** (`apps/web/middleware.ts`): handles auth presence (unauthenticated → `/signin`) and lets `/welcome`, `/signin`, `/signup`, and `/invite/accept` through without a session.
- **`/welcome`**: re-queries `getMyWorkspaces` on every render. If a workspace has appeared since the last visit (via auto-bind), the page redirects to it immediately; otherwise it shows the user's email and a Refresh button.

The combined effect: a new user who self-signs up lands on `/welcome` → an admin invites them → next time `/welcome` (or any dashboard page) is loaded, the auto-bind picks up the pending invite and the user is in.

## Acceptance flow (Option A)

The MVP ships with signup-with-token. Phase B (per-user JWT) is deferred.

```text
+---------------+        public lookup       +---------------+
| invite email  | -- GET /v1/invites/:id  -> | accept page   |
+---------------+        /lookup             | renders shell |
                                             +-------+-------+
                                                     |
                              user already exists?   |
                                                     v
                              +----------------------+----------------------+
                              |                                             |
                              | yes                                         | no
                              v                                             v
                  +-----------+-----------+               +-----------------+----------------+
                  | "Sign in first"       |               | SignupForm pre-filled with email |
                  | message + signin link |               | + invite_token hidden field      |
                  +-----------------------+               +-----------------+----------------+
                                                                            |
                                                                            v
                                                            POST /v1/auth/signup
                                                            { username, password, invite_token }
                                                                            |
                                                                            v
                                                            +---------------+---------------+
                                                            | create account                |
                                                            | accept_invite(token, user_id) |
                                                            | atomic in two steps           |
                                                            +---------------+---------------+
                                                                            |
                                                                            v
                                                                       201 + auto-login
                                                                       redirect to /
```

Why two steps and not one transaction across account creation + acceptance: `users` and `workspace_invites` live in separate writes here so the in-memory test path can run without the team store. If a valid `invite_token` accompanies a signup but acceptance fails (revoked, expired, already accepted), the API returns 422 and the new account survives — the user can ask their admin for a fresh invite without re-registering.

### Why signup-with-token, not full JWT

`tl-server`'s bearer middleware accepts a single shared API key (`TL_API_KEY`). It does not yet mint per-user JWTs. Building Option B (proper per-user auth across every `/v1/*` admin route) would double the blast radius of this change. Option A unblocks invites for the dashboard's signup-driven path without touching the middleware. Phase B is tracked separately — see `auth_user.rs` module docs.

### Existing users

When `lookup_invite` returns `user_exists: true`, the accept page does **not** render the signup form. It shows "you already have an account for `<email>` — sign in" and points the user at `/signin`. Binding an existing user's account to an invite requires per-user auth (Option B) and is intentionally out of scope for this PR.

## Public lookup

`GET /v1/invites/:id/lookup` is unauthenticated. It returns:

- the invited `email`
- the `role`
- the workspace's `name` + `slug`
- the invite `status` and `expires_at`
- a `user_exists` boolean

Nothing else. This is information the invite recipient already knows or can derive — there's no traversal across workspaces, no member list, no policy data. The token is the capability; possessing it is what the lookup checks.

If the token is malformed or unknown, lookup returns 404 — same shape for both cases so we don't help token-guessers distinguish.

## Memory mode

The non-postgres build wires a `MemoryTeamStore` so the no-DB boot path and integration tests keep running. The memory implementation deliberately drops workspace-slug lookups (it returns a placeholder `"Workspace"` / `"workspace"`) — the accept-page UX is only meaningful against a real database. Production deployments must run with the `postgres` feature.

## What this PR does *not* do

- **Email delivery.** No SMTP/Resend integration. The dashboard surfaces "Copy invite link" so admins share the URL out-of-band. A follow-up PR can layer email on top of `POST /v1/team/invites`.
- **Per-user JWT / Option B.** See above.
- **Bulk invite / CSV upload.** Out of scope.
- **Member role edits / remove member.** The members table is read-only in the UI today; mutations land in a follow-up.

## See also

- [architecture.md](architecture.md) — request flow + layer ownership.
- [web-dashboard-authentication.md](web-dashboard-authentication.md) — signup + signin flow that the accept page hooks into.
- [glossary.md](glossary.md) — canonical term definitions.
