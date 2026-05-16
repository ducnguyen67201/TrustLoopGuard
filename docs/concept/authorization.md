# Authorization

How `tl-server` decides whether a request is allowed in.

> Terms: [Workspace](glossary.md#workspace-member), [Workspace API key](#workspace-api-keys-future). For *user* authentication (NextAuth, signup/signin), see [web-dashboard-authentication.md](web-dashboard-authentication.md).

## Two lanes, one middleware

`tl-server`'s bearer middleware accepts **two kinds of credential**, never more:

```text
                                            ┌─────────────────────────┐
                                            │   tl-server  /v1/*      │
                                            └────────────┬────────────┘
                                                         │
                                            Authorization: Bearer …
                                            ┌────────────┴────────────┐
                                            │                         │
                                       TL_API_KEY              tl_live_<…>
                                       (internal)              (per workspace,
                                            │                   future)
                                            ▼                         ▼
                                   ┌────────────────┐       ┌──────────────────┐
                                   │ Web dashboard  │       │ Customer SDK in  │
                                   │ proxy          │       │ their product    │
                                   │                │       │                  │
                                   │ NextAuth gates │       │ Issued from the  │
                                   │ the user;      │       │ /api-keys page,  │
                                   │ web → Rust is  │       │ scoped to one    │
                                   │ trusted        │       │ workspace        │
                                   │ service-to-    │       │                  │
                                   │ service        │       │                  │
                                   └────────────────┘       └──────────────────┘
```

There is **no per-user JWT lane.** User identity is forwarded by the trusted web proxy as headers, not as auth.

## `TL_API_KEY` — internal / web-to-Rust

A single shared static token configured per deployment.

- **Where it lives**: env var on the server; `Doppler secrets set TL_API_KEY=…` for staging/prod. Unset in local dev → middleware skipped, endpoints open (a warning logs at boot).
- **Who uses it**: the Next.js dashboard's same-origin proxy (`apps/web/app/api/*`), the seed script, and any internal tooling. It is the trust anchor for "this caller is us."
- **Workspace scoping**: there is none. A request with `TL_API_KEY` reads `X-TLG-Workspace-Id` from the headers and trusts it. Safe because only first-party code sets that header.
- **User identity**: when the web proxy forwards on behalf of a signed-in user, it adds `X-TLG-User-Id` and (optionally) `X-TLG-User-Email`. The Rust handler reads them for invite acceptance, audit fields like `invited_by_user_id`, and the auto-bind path in `GET /v1/team/my-workspaces`. Rust **does not verify** these headers — it trusts them because the bearer already established this is the first-party web app.

## Workspace API keys (future)

The schema for per-customer keys is already in place (`workspace_api_keys` table, migration 6):

| column | purpose |
|---|---|
| `id` | opaque row id |
| `workspace_id` | which workspace this key authorizes |
| `name` | human-readable label, shown in `/api-keys` |
| `key_prefix` | first ~8 chars of the plaintext, e.g. `tl_live_abc…`, for lookup hint and UI |
| `key_hash` | SHA-256 of the full key, what we compare against on every request |
| `status` | `active` / `revoked` |
| `created_by_user_id` | audit |
| `created_at` / `last_used_at` / `revoked_at` | lifecycle |

**Not wired yet.** The `/api-keys` page lists rows but the create flow and the middleware-side verification are follow-ups. Target behaviour:

- **Creation**: admin clicks "Create key" → server generates 32 random bytes → returns `tl_live_<base64>` **once** in the response → stores only the SHA-256 hash + the `tl_live_<prefix>` snippet.
- **Verification**: middleware inspects the bearer prefix.
  - Starts with `tl_live_` → SHA-256 the value, look up in `workspace_api_keys`, attach `workspace_id` to the request from the row. **The key decides the workspace, not the header** — `X-TLG-Workspace-Id` is ignored in this lane.
  - Otherwise → compare against `TL_API_KEY`, fall back to the internal lane.
- **Scope enforcement**: every authoring endpoint then reads `workspace_id` from the request extension, not from the header. Cross-workspace access becomes structurally impossible.

## What this model does *not* have

- **No user sessions on Rust.** No JWT, no refresh tokens, no `Authorization: Bearer <user-token>`. NextAuth owns user identity end-to-end on the web; Rust trusts the web because the web holds `TL_API_KEY`.
- **No per-key rate limiting yet.** Possible later via the same row that authorizes the call.
- **No third-party OAuth flows for SDK access.** Customers want a key, not a flow.

## Why this shape

1. **Tiny attack surface on the server.** Two credential formats. One middleware. Lookup is `==` for the internal key and a single hash comparison for the workspace key. No JWT validation, no key rotation, no introspection round-trip.
2. **Web stays the source of truth for user identity.** NextAuth already handles OAuth + sessions + CSRF. Rebuilding that in Rust would duplicate logic that has nothing to do with guardrails.
3. **Customers get the credential model they expect.** "Here's an API key, paste it in" beats "set up an OIDC client" by every measure that matters for SDK adoption.
4. **Per-workspace isolation by construction.** Once the future key-lookup path is in, a customer literally cannot reach another customer's workspace — the key never resolves to one.

## See also

- [web-dashboard-authentication.md](web-dashboard-authentication.md) — NextAuth + signup/signin flow (user *authentication*, complements this doc's *authorization*).
- [team-and-invites.md](team-and-invites.md) — invite acceptance and workspace membership.
- [architecture.md](architecture.md) — request flow + layer ownership.
