# Runtime authentication

How `tl-server` authenticates callers and scopes their access. The shared approval, grant, and lease lifecycle is documented separately in [authorization-kernel.md](authorization-kernel.md).

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

- **Where it lives**: env var on the server; `Doppler secrets set TL_API_KEY=…` for staging/prod. When unset for local development, middleware is skipped only on a loopback listener. `tl-server` refuses an unauthenticated non-loopback bind.
- **Who uses it**: the Next.js dashboard's same-origin proxy (`apps/web/app/api/*`), the seed script, and any internal tooling. It is the trust anchor for "this caller is us."
- **Workspace scoping**: there is none. A request with `TL_API_KEY` reads `X-TLG-Workspace-Id` from the headers and trusts it. Safe because only first-party code sets that header.
- **User identity**: the web uses this lane for first-party service calls, including
  `POST /v1/identity/oauth-session`, which maps an already-authenticated Google/GitHub
  account to a local TrustLoopGuard user record.
- **User approval gate**: requests that carry `X-TLG-User-Id` are admitted only if the Rust-owned
  `users` row has `is_approved=true`. Internal calls without user context and customer `tl_live_`
  workspace keys are not affected.

## User-session JWT — HS256, minted by Rust

`POST /v1/auth/signup` and `POST /v1/auth/login` return a freshly-minted JWT in the response body's `jwt` field. The web stashes it in the NextAuth session (cookie-backed JWT, signed with `AUTH_SECRET`, HttpOnly). On every Rust API call made on behalf of a credentials user, the web proxy forwards it as `Authorization: Bearer <jwt>`. The dashboard and Rust server expose this username/password path only in local development; staging and production require OAuth.

- **Algorithm**: HS256.
- **Secret**: `TL_JWT_SECRET` (env). Should be ≥32 random bytes; a short value logs a warning at boot. Unset → no JWT is minted and `AuthResponse.jwt` is omitted; the web falls back to header-forwarded identity.
- **Claims**: `sub` (UUID), `username`, `iat`, `exp`. No roles, no scopes. Authorization for workspace data still lives at the membership layer.
- **TTL**: 7 days. No refresh flow — when the JWT expires the user signs in again. The NextAuth cookie has its own lifetime managed by NextAuth.
- **Verification path**: middleware reads `Authorization: Bearer <token>`, attempts `JwtSigner::verify`, and on success attaches a `UserContext { user_id, username }` to the request extension. Handlers that need user identity read the extension instead of trusting raw headers.
- **User approval gate**: JWT verification is followed by a `users.is_approved` check. Unapproved
  users receive `403 forbidden` before protected handlers run — there is no environment bypass.
- **Environment gate**: Rust returns 404 from `/v1/auth/signup`, `/v1/auth/login`, and `/v1/auth/password` unless the server is in local-development mode. A configured server with `DATABASE_URL` or `TL_API_KEY` and no local environment marker defaults password auth off.
- **No refresh endpoint, no revocation list**: stateless verification only. If a JWT is compromised the only mitigation today is rotating `TL_JWT_SECRET`, which invalidates every session. Add a `jti` denylist if that ever matters.

### OAuth users (Google / GitHub)

Google and GitHub authenticate the browser user through Auth.js. Rust does not verify provider
passwords, hold provider refresh tokens, or act as the OAuth provider.

After Auth.js completes the provider flow, the web calls:

```text
POST /v1/identity/oauth-session
Authorization: Bearer <TL_API_KEY>
```

This route is intentionally **internal-lane only**: bearer auth on
`/v1/identity/oauth-session` accepts `TL_API_KEY` and rejects user JWT and
`tl_live_` workspace runtime keys with `401`.

The request carries the provider id (`google` or `github`), the provider's stable account subject,
and the provider email. Rust resolves that identity to one local TrustLoopGuard `users.id`:

1. Find an existing row in `oauth_identities` by `(provider, provider_subject)`.
2. Otherwise find an existing `users` row by email/username and link the provider identity to it.
3. Otherwise create a new local `users` row and link the provider identity.

The response is the same `AuthResponse` shape as credentials login: canonical local `user_id`,
username/email, and a Rust JWT when `TL_JWT_SECRET` is configured. The web stores that local user id
and JWT in the Auth.js session so workspace membership checks never depend on provider-specific ids.

New OAuth-created `users` rows default to `is_approved=false`. An operator must approve the user
row before that user can access dashboard-backed protected routes.

## Workspace API keys

Per-customer SDK/runtime keys live in `workspace_api_keys`:

| column | purpose |
|---|---|
| `id` | opaque row id |
| `workspace_id` | which workspace this key authorizes |
| `environment_id` | which environment runtime calls from this key use |
| `name` | human-readable label, shown in `/api-keys` |
| `key_prefix` | leading plaintext snippet, e.g. `tl_live_abc...`, for lookup hint and UI |
| `key_hash` | SHA-256 of the full key, what we compare against on every request |
| `status` | `active` / `revoked` |
| `created_by_user_id` | audit |
| `created_at` / `last_used_at` / `revoked_at` | lifecycle |

The `/api-keys` dashboard page creates and lists these keys through Rust:

- **Creation**: `POST /v1/api-keys` generates 32 random bytes, returns `tl_live_<base64url>` **once**, and stores only the SHA-256 hash plus a prefix snippet. The request selects an environment, defaulting to the workspace default environment.
- **Listing**: `GET /v1/api-keys` returns metadata only, including environment. The plaintext secret is never returned after creation.
- **Revocation**: `PATCH /v1/api-keys/batch/revoke` marks selected workspace keys as `revoked` and sets `revoked_at`.
- **Management authorization**: API key create/list/revoke requires an authenticated dashboard user who is an owner or admin of the workspace. The caller may authenticate with a user JWT or through the internal dashboard service lane with forwarded user context. Workspace id alone is never authority.
- **Control-plane separation**: workspace runtime keys cannot mutate agent profiles or source-label policies. Agent deletion is rejected before its owned-policy cascade runs.
- **Verification**: middleware inspects the bearer prefix. Starts with `tl_live_` -> SHA-256 the value, look up an active `workspace_api_keys` row, attach that row's `workspace_id` and `environment_id`, and update `last_used_at`.
- **Scope enforcement**: the key decides the workspace and environment. Middleware overwrites `X-TLG-Workspace-Id` and `X-TLG-Environment-Id` with the stored values before handlers run, so caller-provided workspace or environment context cannot steer the request into another scope.
- **Runtime-only surface**: workspace keys are for SDK and gateway model traffic. They cannot list, create, or revoke API keys, and gateway configuration endpoints reject this lane. Dashboard/user credentials must manage provider connections, routes, and keys.
- **Deployment boundary**: hosted SDK integrations use a workspace key at runtime and keep internal/dashboard credentials in setup or control-plane processes only. Runtime services do not select tenancy with caller-supplied workspace headers; the stored key scope selects it.

## Hosted MCP OAuth lane

`/mcp` is a separate resource-server lane, not a fourth credential accepted by
generic `/v1` middleware. Its audience-bound access token carries signed
workspace, member, OAuth client, scope, and registered-agent identity. The
server revalidates membership, agent existence, and the feature flag on every
MCP request; identity supplied in tool arguments or MCP metadata is never
trusted. Exact member-and-agent entitlement and the two policy checkpoints are
defined in [hosted-mcp-access-gateway.md](hosted-mcp-access-gateway.md).

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
