# Gateway

The gateway is TrustLoopGuard's proxy integration path. It lets a customer route provider traffic through the Rust runtime instead of calling `Guard.check` from application code.

## Ownership

Rust owns the gateway data plane and durable configuration:

- `tl-server` exposes provider-compatible proxy endpoints and dashboard configuration APIs.
- `tl-storage` persists provider connections, gateway routes, and enforcement profiles.
- `tl-core` owns the public wire types for gateway configuration.
- `apps/web` only proxies dashboard requests and renders configuration.

The Next.js app must not forward model traffic or evaluate policies.

## SDK vs Gateway

SDK mode returns a `Decision` to customer code:

```text
Customer app -> SDK -> /v1/check -> Decision -> customer handles action
```

Gateway mode applies the action inside TrustLoopGuard:

```text
Customer app -> /v1/gateway/... -> input check -> provider -> output check -> safe response
```

Both modes use the same policy engine. The difference is who handles the verdict.

## Configuration

Provider connections store customer-owned provider credentials for OpenAI-compatible and Anthropic requests. The API never returns plaintext provider keys after creation.

Gateway routes bind a public route id to:

- a provider connection
- an agent id
- an enforcement profile

Enforcement profiles define what the proxy does after a policy match: input action, output action, fail mode, retention mode, fallback message, and regeneration budget.

## Retention

Gateway traces include route, provider, phase, action, and retention metadata. Raw prompt/output storage is controlled by the enforcement profile:

- `metadata_only` stores no raw body text in check payloads.
- `redacted_body` stores a placeholder.
- `full_body` stores the content sent to the checker.

## Provider Support

The first gateway surface is non-streaming:

- `POST /v1/gateway/{route_id}/openai/chat/completions`
- `POST /v1/gateway/{route_id}/anthropic/v1/messages`

Streaming requests return an explicit unsupported error until chunk buffering and interruption semantics are implemented.
