# Hosted MCP access gateway

The hosted MCP access gateway gives every workspace one remote MCP endpoint,
`$TL_PUBLIC_URL/mcp`. Employees add that endpoint to an MCP-capable AI client;
OAuth identifies the employee and workspace, durable assignments determine
which tools are visible, and the existing Rust policy runtime decides whether
each proposed call may execute.

![Hosted MCP access gateway](assets/hosted-mcp-access-gateway.svg)

This is an opt-in integration path. `workspaces.is_mcp_gateway_enabled` defaults
to `false`, so existing SDK, `/v1/events`, provider gateway, local MCP server,
and local `apps/mcp-proxy` behavior is unchanged.

## Ownership and boundaries

| Concern | Owner |
|---|---|
| OAuth registrations, authorization codes, refresh tokens | Rust/Postgres in `tl-server` and `tl-storage` |
| Remote server configuration, pinned catalog, assignments | Rust/Postgres in `tl-server` and `tl-storage` |
| Runtime policy decision, approvals, leases, traces | Existing Rust event and authorization services |
| Dashboard rendering and same-origin API proxying | `apps/web` |
| Managed MCP protocol endpoint | `POST /mcp` in `tl-server` |

The web application never stores credentials, catalogs, assignments, OAuth
codes, or refresh tokens. Connection credentials are write-only, AES-GCM
sealed with the existing gateway credential key, and never returned by a read
API. Authorization codes and refresh tokens are stored only as SHA-256 hashes.

## Identity and access

The managed endpoint accepts only an OAuth access token minted for the exact
resource `$TL_PUBLIC_URL/mcp` and scope `mcp:tools`. It requires `iss`, `aud`,
`oauth_client_id`, `workspace_id`, and `scope` claims. Internal service keys,
ordinary dashboard JWTs, and `tl_live_` runtime keys cannot authenticate to
this endpoint. Conversely, the generic `/v1` JWT verifier rejects the
audience-bearing MCP token.

Every MCP request rechecks current workspace membership and the feature flag,
including `initialize`, `ping`, `tools/list`, and `tools/call`. Removing a
member or disabling the feature therefore takes effect without waiting for an
access token to expire. `tools/list` reads only active, assigned catalog rows;
it never contacts an upstream server.

Owner/Admin members can manage servers, synchronize catalogs, classify side
effects, and replace per-tool member assignments under `/v1/mcp-gateway/*`.
Other members can read only connect information. Runtime keys cannot use the
control plane.

OAuth discovery is served by the Rust public origin. Its token and dynamic
registration endpoints remain on `$TL_PUBLIC_URL`, while its authorization
endpoint points to `$TL_DASHBOARD_URL/oauth/authorize`, where the existing
dashboard session renders consent and a human-readable username or email.
Dynamic client registration is limited to 20 requests per minute per server
instance, inserts under one atomic 10,000-client capacity check, and prunes
clients older than 30 days when they have no unexpired authorization code or
refresh token. Production ingress must apply a distributed rate limit as well.

## Safe remote servers

The MVP supports remote MCP Streamable HTTP only. It does not launch commands,
stdio servers, WebSockets, or legacy HTTP+SSE servers. Production endpoints
must use HTTPS. URLs with user info, query credentials, or fragments are
rejected. Every connection resolves DNS again, rejects non-public and metadata
addresses, pins accepted addresses in a no-proxy client, disables redirects
and retries, and applies bounded connect and operation timeouts. The insecure
HTTP development switch accepts only `localhost` and literal loopback hosts;
it cannot send a bearer credential to a public HTTP endpoint.

An administrator-triggered sync pins names, descriptions, annotations, input
and output schemas, and schema hashes. Catalog limits are 500 tools, 4 KiB per
description, and 64 KiB per schema. External `$ref` values and deeply nested or
non-object input schemas are rejected. An absent tool becomes `missing`; a
runtime schema mismatch becomes `schema_changed`. Either status hides the tool
until an administrator synchronizes and accepts the current catalog.
Catalog pagination rejects repeated or empty cursors and stops before retaining
more than 500 tools. HTTP response bodies are byte-bounded while streaming,
before JSON or SSE buffering; catalog traffic has a 72 MiB envelope derived
from the per-tool schema limits.

## Governed execution

For an assigned `tools/call`, Rust validates the arguments against the pinned
schema, prepares a safe catalog connection, verifies the live tool and
schema hash, and submits a server-authored `tool.call.proposed` `GuardEvent` to
the existing event service with authorization principal
`mcp:user:<user UUID>`.

Only `permit` is executable. `deny`, `defer`, and `transform` return an MCP tool
error without an upstream call. `require_approval` closes the prepared peer,
waits for the existing approval record for at most 60 seconds, resubmits the
same action with the approval grant, and requires a current permit and lease.
Immediately before execution the gateway re-reads assignment, side-effect
classification, and connection authority. It opens a separately byte-bounded
execution connection only after authorization. It never automatically retries
an upstream call after execution may have started. Leases are completed as
consumed or canceled, and a completion failure tells the caller not to retry
automatically.

Structured output is checked against a pinned output schema when present. The
HTTP execution response is capped while streaming and the serialized MCP result
is independently capped at 1 MiB.

## Rollout and rollback

Hosted rollout requires `TL_PUBLIC_URL`, `TL_DASHBOARD_URL`, `TL_JWT_SECRET`, and
`TL_GATEWAY_CREDENTIAL_KEY`. Plain HTTP is unavailable unless
`TL_MCP_GATEWAY_ALLOW_INSECURE_HTTP=true`; that switch exists for loopback test
servers only.

Enable a pilot workspace operationally:

```sql
UPDATE workspaces
SET is_mcp_gateway_enabled = TRUE, updated_at = NOW()
WHERE id = '<workspace-id>';
```

Rollback is the reverse update. The dashboard item disappears and every MCP
request fails the live feature check. Connections, catalogs, assignments,
OAuth state, policies, approvals, and traces remain durable for diagnosis or a
later re-enable; no existing integration path changes.
