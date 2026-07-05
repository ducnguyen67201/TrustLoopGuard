# Team & invites

How users join a workspace.

> Terms: [Workspace member](glossary.md#workspace-member), [Workspace invite](glossary.md#workspace-invite), [Workspace role](glossary.md#workspace-role).

## Ownership

Rust owns the durable state. Tables, repository, and HTTP endpoints all live in the Rust stack:

- Tables: `workspace_members`, `workspace_invites` (migration `00000000000006_workspace_admin`).
- Repository: `crates/tl-storage/src/team_repo.rs`.
- HTTP handlers: `crates/tl-server/src/team.rs`.
- Wire types: `crates/tl-core/src/team.rs`.

The dashboard is a same-origin proxy. `apps/web/app/api/team/*` forwards to the Rust endpoints with the active workspace's id in `X-TLG-Workspace-Id`. The dashboard never writes to either table directly.

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

- **Id shape.** The invite `id` is an opaque 32-byte URL-safe random string generated server-side in `team_repo::generate_token`. It identifies the pending row for admin revoke operations; it is not a public accept token.
- **TTL.** 7 days, fixed (`INVITE_TTL_DAYS` in `team_repo.rs`). Expired pending invites are excluded from admin lists and auto-bind lookups.
- **Uniqueness.** At most one active pending invite per `(workspace_id, email)`. Re-inviting the same address while one is outstanding returns 409; the admin should revoke first.
- **Single use.** Acceptance is transactional: the row flips to `accepted`, a `workspace_members` row is upserted, and the corresponding `organization_members` row is upserted (as `member`) in the same transaction. A repeated accept is rejected with 409.

## Endpoints

All team endpoints sit behind the existing shared-bearer middleware.

| Method | Path | Body | Returns |
|---|---|---|---|
| `GET`    | `/v1/team/members`         | — | `MemberListResponse` |
| `GET`    | `/v1/team/invites`         | — | `InviteListResponse` (pending only) |
| `POST`   | `/v1/team/invites`         | `{ email, role }` | `CreateInviteResponse` tagged by `kind` (`invited` with `invite`, or `added` with `member`) |
| `DELETE` | `/v1/team/invites/:id`     | — | 204 |
| `GET`    | `/v1/team/my-workspaces`   | — | `MyWorkspacesResponse` *(user-scoped, auto-binds pending invites)* |
| `POST`   | `/v1/team/my-workspaces`   | `{ name }` | `MyWorkspace` *(self-service bootstrap for approved users)* |

Workspace context is always read from `X-TLG-Workspace-Id`. The optional `X-TLG-User-Id` header (UUID) is captured on `POST /v1/team/invites` and persisted to `invited_by_user_id` so the audit trail survives.

`GET /v1/team/my-workspaces` is user-scoped instead: it reads `X-TLG-User-Id` (required, UUID) plus `X-TLG-User-Email`. When the email is present, the server bulk-accepts any pending invite addressed to it *before* querying memberships. This is the auto-bind mechanism: a user invited before or after signup sees the workspace on their next page load without clicking an accept link.

## Enforcement

The dashboard refuses to render when the signed-in user has zero memberships.

- **Server-side**: `getDashboardShell` (in `apps/web/lib/server/dashboard-data.ts`) calls `/v1/team/my-workspaces` first, and `redirect('/welcome')` if the list is empty.
- **`/welcome`**: re-queries `getMyWorkspaces` on every render. If a workspace has appeared since the last visit (via auto-bind), the page redirects to it immediately; otherwise it shows the user's email and a Refresh button.

The combined effect: a new user who self-signs up lands on `/welcome` → an admin invites them → next time `/welcome` (or any dashboard page) is loaded, the auto-bind picks up the pending invite and the user is in.

Unapproved users remain on `/welcome` until an operator approves their user row — approval is the
only gate. Once approved, users self-serve through first-run onboarding: the
`POST /v1/team/my-workspaces` bootstrap path is open to every approved user.

## Acceptance flow

There is one invite-consumption mechanism: email-based auto-bind on the next workspace lookup.

```text
+---------------+       POST /v1/team/invites       +----------------+
| workspace     | ---------------------------------> | existing user? |
| admin         |                                    +-------+--------+
+---------------+                                            |
                                                             |
                                      yes: add member now     | no: pending invite
                                                             v
                                                    +-------------------+
                                                    | workspace_invites |
                                                    +---------+---------+
                                                              |
                                                              | user signs in or signs up
                                                              v
                                                    +-------------------+
                                                    | GET /v1/team/     |
                                                    | my-workspaces     |
                                                    +---------+---------+
                                                              |
                                                              | X-TLG-User-Email matches invite
                                                              v
                                                    +-------------------+
                                                    | accept pending    |
                                                    | invites + return  |
                                                    | memberships       |
                                                    +-------------------+
```

`POST /v1/team/invites` is a smart add path. If the email already belongs to a user, Rust inserts the organization and workspace membership immediately and returns `kind: "added"`. If no user exists yet, Rust records a pending invite and returns `kind: "invited"`.

When the invitee later signs in or signs up with that email, the dashboard's first workspace lookup calls `GET /v1/team/my-workspaces` with `X-TLG-User-Email`. Rust accepts every unexpired pending invite for that email, then returns the updated membership list in the same response.

## Authorization model

The web dashboard is a trusted first-party service: its same-origin proxy calls Rust with either the
user's Rust JWT or `TL_API_KEY` plus `X-TLG-User-Id` and `X-TLG-User-Email` headers. See
[authorization.md](authorization.md) for the full bearer model and approval gate.

## Memory mode

The non-postgres build wires a `MemoryTeamStore` so the no-DB boot path and integration tests keep running. Memory mode cannot check the `users` table during `POST /v1/team/invites`, so it always records a pending invite; the same email-based auto-bind path consumes it later.

## What this PR does *not* do

- **Email delivery.** No SMTP/Resend integration. The dashboard records pending invites but does not send emails.
- **Bulk invite / CSV upload.** Out of scope.
- **Member role edits / remove member.** The members table is read-only in the UI today; mutations land in a follow-up.

## See also

- [architecture.md](architecture.md) — request flow + layer ownership.
- [authorization.md](authorization.md) — the two-key auth model the dashboard and SDKs use.
- [web-dashboard-authentication.md](web-dashboard-authentication.md) — NextAuth signup + signin flow.
- [glossary.md](glossary.md) — canonical term definitions.
