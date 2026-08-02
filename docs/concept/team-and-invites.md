# Team & invites

How users join a workspace.

> Terms: [Workspace member](glossary.md#workspace-member), [Workspace invite](glossary.md#workspace-invite), [Workspace role](glossary.md#workspace-role).

## Ownership

Rust owns the durable state. Tables, repository, and HTTP endpoints all live in the Rust stack:

- Tables: `workspaces`, `workspace_members`, `workspace_invites`, and `workspace_api_keys` (created by migration `00000000000006_workspace_admin`; workspace feature flags added by `00000000000044_workspace_feature_flags`). Platform-wide support access is the default-false `users.is_platform_admin` flag.
- Repository: `crates/tl-storage/src/team_repo.rs`.
- HTTP handlers: `crates/tl-server/src/team.rs`.
- Wire types: `crates/tl-core/src/team.rs`.

The dashboard is a same-origin proxy. `apps/web/app/api/team/*` forwards active-workspace operations with `X-FEATHERLANE-AI-Workspace-Id`; `apps/web/app/api/me/workspaces/*` forwards signed-in user workspace operations. The dashboard never writes durable team or workspace lifecycle state directly.

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
| `GET`    | `/v1/team/my-workspaces`   | — | `MyWorkspacesResponse` *(user-scoped, auto-binds pending invites, returns `is_platform_admin`)* |
| `POST`   | `/v1/team/my-workspaces`   | `{ name }` | `MyWorkspace` *(self-service bootstrap for approved users)* |
| `DELETE` | `/v1/team/my-workspaces/{id}` | — | 204 *(owner-only soft delete)* |

Active-workspace team operations read context from `X-FEATHERLANE-AI-Workspace-Id`. The optional `X-FEATHERLANE-AI-User-Id` header (UUID) is captured on `POST /v1/team/invites` and persisted to `invited_by_user_id` so the audit trail survives.

The `GET`, `POST`, and `DELETE` operations under `/v1/team/my-workspaces` are user-scoped instead. They derive the signed-in user from the Rust JWT context or the trusted dashboard-forwarded user id; they do not authorize from the currently selected workspace. `GET` also reads `X-FEATHERLANE-AI-User-Email` when present and bulk-accepts pending invites addressed to it *before* querying memberships. This is the auto-bind mechanism: a user invited before or after signup sees the workspace on their next page load without clicking an accept link.

For an ordinary user, `GET /v1/team/my-workspaces` returns only active workspace memberships and
`is_platform_admin: false`. When `users.is_platform_admin` is true, it returns every active
workspace with an effective `admin` role and `is_platform_admin: true`. Platform administrators
therefore pass the same server-side workspace and owner/admin gates as a workspace administrator.
They do not receive an inserted membership row, and owner-only workspace deletion remains
membership-bound.

Each returned `MyWorkspace` also includes `is_knowledge_base_enabled` and `is_attacks_enabled`. Both columns are `NOT NULL DEFAULT false` on `workspaces`, so those dashboard features remain unavailable until a workspace is explicitly enrolled. The dashboard maps them to camel-case shell fields, omits the corresponding navigation items, and returns not found for direct page requests when disabled. These are product-availability flags; they do not replace authorization on any Rust endpoint.

## Workspace deletion lifecycle

Only an active workspace owner authenticated through a signed user context or the trusted dashboard
service may call `DELETE /v1/team/my-workspaces/{id}`. Workspace runtime keys are rejected before
Rust reads a forwarded user id, so a runtime caller cannot impersonate an owner. The dashboard hides
the action from other roles and requires the owner to type the exact, case-sensitive workspace name,
but Rust membership is the authorization boundary.

Deletion is one PostgreSQL transaction. Rust locks the active workspace and caller membership, changes pending invites to `revoked`, changes active runtime API keys to `revoked` with `revoked_at` set, and timestamps `workspaces.deleted_at`. It does not delete the workspace, organization, memberships, environments, policies, traces, decisions, or other historical records. Active workspace and member queries exclude deleted workspaces, so retained membership rows cannot continue to authorize access.

The first serialized deletion returns 204. A repeated or concurrent request that reaches the row after deletion returns 404. In memory mode the same team contract is represented by a workspace tombstone and revoked pending invites; memory mode has no runtime API-key store.

After success, deleting an inactive workspace retains the current selection. Deleting the active workspace selects another accessible workspace, or sends the owner to `/onboarding/workspace` when none remain.

## Enforcement

The dashboard refuses to render when the signed-in user has zero authorized workspaces.

- **Server-side**: `getDashboardShell` (in `apps/web/lib/server/dashboard-data.ts`) calls `/v1/team/my-workspaces` first, and redirects to `/onboarding/workspace` if the list is empty.
- **`/welcome`**: re-queries `getMyWorkspaces` on every render. If a workspace has appeared since the last visit (via auto-bind), the page redirects to it immediately; otherwise it shows the user's email and a Refresh button.

The combined effect: a new ordinary user without memberships enters workspace onboarding. If an
admin invites them, their next workspace lookup auto-binds the pending invite and the dashboard can
open that workspace. A platform administrator can instead open any active workspace returned by
Rust.

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
                                                              | X-FEATHERLANE-AI-User-Email matches invite
                                                              v
                                                    +-------------------+
                                                    | accept pending    |
                                                    | invites + return  |
                                                    | memberships       |
                                                    +-------------------+
```

`POST /v1/team/invites` is a smart add path. If the email already belongs to a user, Rust inserts the organization and workspace membership immediately and returns `kind: "added"`. If no user exists yet, Rust records a pending invite and returns `kind: "invited"`.

When the invitee later signs in or signs up with that email, the dashboard's first workspace lookup calls `GET /v1/team/my-workspaces` with `X-FEATHERLANE-AI-User-Email`. Rust accepts every unexpired pending invite for that email, then returns the updated membership list in the same response.

## Authorization model

The web dashboard is a trusted first-party service: its same-origin proxy calls Rust with the
user's Rust JWT or with `TL_API_KEY` plus trusted `X-FEATHERLANE-AI-User-Id` and `X-FEATHERLANE-AI-User-Email` headers.
Rust derives invite attribution from that authenticated user identity rather than accepting an
untrusted caller-supplied user id. See [authorization.md](authorization.md) for the full bearer model
and approval gate.

Listing, creating, and revoking pending invites requires an authenticated workspace Owner or Admin
(or a platform administrator). Workspace runtime keys are rejected before the invite store is
called, even if they send a forged user-id header. The invite API accepts `admin`, `editor`, and
`viewer`; it rejects `owner` because ownership transfer requires a dedicated lifecycle rather than
an invite role assignment.

## Memory mode

The non-postgres build wires a `MemoryTeamStore` so the no-DB boot path and integration tests keep running. Memory mode cannot check the `users` table during `POST /v1/team/invites`, so it always records a pending invite; the same email-based auto-bind path consumes it later. Workspace deletion tombstones the workspace and revokes pending invites in memory, but runtime API-key revocation is a PostgreSQL-only durable concern.

## What this PR does *not* do

- **Email delivery.** No SMTP/Resend integration. The dashboard records pending invites but does not send emails.
- **Bulk invite / CSV upload.** Out of scope.
- **Member role edits / remove member.** The members table is read-only in the UI today; mutations land in a follow-up.
- **Restore or permanent deletion.** Deleted workspace history is retained; there is no restore or hard-delete flow.
- **Organization deletion, ownership transfer, or export.** Workspace deletion does not add these lifecycle operations.

## See also

- [architecture.md](architecture.md) — request flow + layer ownership.
- [authorization.md](authorization.md) — the two-key auth model the dashboard and SDKs use.
- [web-dashboard-authentication.md](web-dashboard-authentication.md) — NextAuth signup + signin flow.
- [glossary.md](glossary.md) — canonical term definitions.
