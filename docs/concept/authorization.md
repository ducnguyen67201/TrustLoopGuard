# Authorization

How `tl-server` decides whether a request is allowed in.

> Terms: [Workspace](glossary.md#workspace-member), [Workspace API key](#workspace-api-keys). For *user* authentication (NextAuth, signup/signin), see [web-dashboard-authentication.md](web-dashboard-authentication.md).

## Three lanes, one middleware

`tl-server`'s bearer middleware accepts three credential formats. Every request takes exactly one lane:

```text
                          ┌─────────────────────────────────────┐
                          │       tl-server  /v1/*              │
                          └──────────────────┬──────────────────┘
                                             │
                              Authorization: Bearer <token>
                          ┌──────────────────┼──────────────────┐
                          │                  │                  │
                     TL_API_KEY        user JWT          tl_live_<…>
                     (internal)        (HS256)           (per workspace,
                          │                │              future)
                          ▼                ▼                  ▼
                ┌──────────────────┐ ┌────────────────┐ ┌──────────────────┐
                │ Web dashboard    │ │ Signed-in dash │ │ Customer SDK in  │
                │ proxy (service)  │ │ user, credentials│ │ their product    │
                │                  │ │ sign-in        │ │                  │
                │ Workspace context│ │ Issued by Rust │ │ Issued from the  │
                │ from headers,    │ │ on signup /    │ │ /api-keys page,  │
                │ trusted          │ │ login          │ │ scoped to one    │
                │                  │ │                │ │ workspace        │
                └──────────────────┘ └────────────────┘ └──────────────────┘
```

The middleware tries them in order: API-key first (cheapest — const-time byte compare), then JWT verification, then workspace-key hash lookup. First match wins; failure on all three returns `401`.

## `TL_API_KEY` — internal / web-to-Rust

A single shared static token configured per deployment.

- **Where it lives**: env var on the server; `Doppler secrets set TL_API_KEY=…` for staging/prod. Unset in local dev → middleware skipped, endpoints open (a warning logs at boot).
- **Who uses it**: the Next.js dashboard's same-origin proxy (`apps/web/app/api/*`), the seed script, and any internal tooling. It is the trust anchor for "this caller is us."
- **Workspace scoping**: there is none. A request with `TL_API_KEY` reads `X-TLG-Workspace-Id` from the headers and trusts it. Safe because only first-party code sets that header.
- **User identity**: when the web proxy forwards on behalf of a signed-in user *without* a Rust JWT (OAuth users, today), it adds `X-TLG-User-Id` + optionally `X-TLG-User-Email` as fallback identity headers. Rust handlers read them — but only because the bearer already established this is the first-party web app.

## User-session JWT — HS256, minted by Rust

`POST /v1/auth/signup` and `POST /v1/auth/login` return a freshly-minted JWT in the response body's `jwt` field. The web stashes it in the NextAuth session (cookie-backed JWT, signed with `AUTH_SECRET`, HttpOnly). On every Rust API call made on behalf of a credentials user, the web proxy forwards it as `Authorization: Bearer <jwt>`. The dashboard exposes this username/password path only in local development; staging and production require OAuth.

- **Algorithm**: HS256.
- **Secret**: `TL_JWT_SECRET` (env). Should be ≥32 random bytes; a short value logs a warning at boot. Unset → no JWT is minted and `AuthResponse.jwt` is omitted; the web falls back to header-forwarded identity.
- **Claims**: `sub` (UUID), `username`, `iat`, `exp`. No roles, no scopes. Authorization for workspace data still lives at the membership layer.
- **TTL**: 7 days. No refresh flow — when the JWT expires the user signs in again. The NextAuth cookie has its own lifetime managed by NextAuth.
- **Verification path**: middleware reads `Authorization: Bearer <token>`, attempts `JwtSigner::verify`, and on success attaches a `UserContext { user_id, username }` to the request extension. Handlers that need user identity read the extension instead of trusting raw headers.
- **No refresh endpoint, no revocation list**: stateless verification only. If a JWT is compromised the only mitigation today is rotating `TL_JWT_SECRET`, which invalidates every session. Add a `jti` denylist if that ever matters.

### OAuth users (Google / GitHub)

OAuth users currently **do not** get a Rust JWT — they sign in through NextAuth without ever hitting `/v1/auth/login`. Their requests fall back to the internal `TL_API_KEY` + `X-TLG-User-Id` header lane. Wiring OAuth onto the JWT path requires a `POST /v1/auth/oauth-ensure`-style endpoint that creates/finds a Rust user for the OAuth identity and returns a JWT. Tracked separately; not in this milestone.

## Workspace API keys

Per-customer SDK/runtime keys live in `workspace_api_keys`:

| column | purpose |
|---|---|
| `id` | opaque row id |
| `workspace_id` | which workspace this key authorizes |
| `name` | human-readable label, shown in `/api-keys` |
| `key_prefix` | leading plaintext snippet, e.g. `tl_live_abc...`, for lookup hint and UI |
| `key_hash` | SHA-256 of the full key, what we compare against on every request |
| `status` | `active` / `revoked` |
| `created_by_user_id` | audit |
| `created_at` / `last_used_at` / `revoked_at` | lifecycle |

The `/api-keys` dashboard page creates and lists these keys through Rust:

- **Creation**: `POST /v1/api-keys` generates 32 random bytes, returns `tl_live_<base64url>` **once**, and stores only the SHA-256 hash plus a prefix snippet.
- **Listing**: `GET /v1/api-keys` returns metadata only. The plaintext secret is never returned after creation.
- **Revocation**: `PATCH /v1/api-keys/batch/revoke` marks selected workspace keys as `revoked` and sets `revoked_at`.
- **Verification**: middleware inspects the bearer prefix. Starts with `tl_live_` → SHA-256 the value, look up an active `workspace_api_keys` row, attach that row's `workspace_id`, and update `last_used_at`.
- **Scope enforcement**: the key decides the workspace. Middleware overwrites `X-TLG-Workspace-Id` with the stored workspace before handlers run, so caller-provided workspace headers or `/v1/check.workspace_id` cannot steer the request into another workspace.

## What this model does *not* have

- **No refresh tokens, no revocation list.** Stateless JWT verification only. The TTL is the only expiry.
- **No per-key rate limiting yet.** Possible later via the same row that authorizes the call.
- **No third-party OAuth flows for SDK access.** Customers want a key, not a flow.

## Why this shape

1. **Tiny attack surface on the server.** Three credential formats, one middleware, no introspection round-trips. Internal key is `==`; JWT is signature-verify; workspace key is a single hash lookup.
2. **NextAuth keeps the cookie / OAuth surface.** Rust doesn't need to know about cookies, CSRF, or OAuth callback flows. It signs a bearer token; the web is responsible for transporting it.
3. **Customers get the credential model they expect.** "Here's an API key, paste it in" beats "set up an OIDC client" by every measure that matters for SDK adoption.
4. **Per-workspace isolation by construction.** A customer cannot reach another customer's workspace with a runtime key because the key resolves to exactly one stored workspace.

## See also

- [web-dashboard-authentication.md](web-dashboard-authentication.md) — NextAuth + signup/signin flow (user *authentication*, complements this doc's *authorization*).
- [team-and-invites.md](team-and-invites.md) — invite acceptance and workspace membership.
- [architecture.md](architecture.md) — request flow + layer ownership.
