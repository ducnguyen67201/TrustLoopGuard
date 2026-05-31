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
Customer app -> /v1/gateway/... -> run -> input check -> provider -> output check -> safe response
```

Both modes use the same policy engine. The difference is who handles the verdict.

## Configuration

Provider connections store customer-owned provider credentials for OpenAI-compatible and Anthropic requests. The API never returns plaintext provider keys after creation.

Gateway routes bind a public route id to:

- a provider connection
- an agent id
- an enforcement profile

Enforcement profiles define what the proxy does after a policy match: input action, output action, fail mode, retention mode, fallback message, and regeneration budget.

Dashboard/user credentials manage this configuration. Workspace runtime API keys (`tl_live_...`) may call the provider-compatible gateway data plane, but they cannot create or update provider connections, routes, enforcement profiles, or other runtime keys.

Provider credentials are encrypted with `TL_GATEWAY_CREDENTIAL_KEY`. Development can fall back to `TL_API_KEY`; if neither secret is configured the server refuses to seal gateway credentials unless `TL_GATEWAY_ALLOW_INSECURE_DEV_KEY` is explicitly enabled for local-only use.

The dashboard setup flow mirrors this ownership model:

1. Create a provider connection.
2. Create an enforcement profile.
3. Create a route that binds provider, agent, and profile.
4. Create a workspace runtime API key as a signed-in workspace owner/admin if the route will be called outside the dashboard.

Once the route is ready, OpenAI-compatible clients use:

```text
baseURL = https://<server>/v1/gateway/<route_id>/openai
```

Anthropic clients use:

```text
baseURL = https://<server>/v1/gateway/<route_id>/anthropic
```

OpenAI-compatible SDKs usually send the runtime key as `Authorization: Bearer ...` when configured with `apiKey`. Anthropic SDK examples must use bearer-token auth, such as `authToken`, because the gateway authenticates runtime calls through the Rust bearer middleware.

## Enforcement Response Signal

When the gateway blocks or escalates a request, it returns a response the agent framework can distinguish from a legitimate provider reply:

- **`finish_reason: "content_filter"`** (OpenAI) / **`stop_reason: "content_filter"`** (Anthropic) — the industry-standard signal for policy-blocked content. Agent frameworks built on these SDKs already handle this case correctly and will not loop.
- **`X-TrustLoopGuard-Verdict`** — `"blocked"` or `"escalated"`. Blocked means the response was suppressed; escalated means it was suppressed and a human review was triggered.
- **`X-TrustLoopGuard-Phase`** — `"input"` or `"output"`, indicating which check fired.
- **`X-TrustLoopGuard-Trace-Id`** — the trace UUID for correlation in the dashboard.
- **`X-TrustLoopGuard-Policy-Id`** — the first triggered policy ID, if any.

Clean responses (allow or successful rewrite) carry none of these headers, so the agent treats them as normal provider replies.

## Observability

Each accepted gateway model request creates a `chat_session` run before provider credentials are decrypted or policies are checked. The gateway attaches its input and output checks to that run by passing `run_id` into the same runtime check path used by SDK integrations. For each provider-compatible request inside a grouped session, the gateway also creates a `user_turn` run event with the latest user message as the input summary and links the input/output checks to that event.

By default, the run's `external_id` is the gateway request id. Callers may send `X-TLG-Run-External-Id` to correlate provider-compatible requests that belong to the same upstream session, such as a LiveKit room or customer chat id. When a matching run already exists for the route's agent and external id, the gateway reuses it so the dashboard groups all input/output checks under one run.

Run metadata records the integration mode, route id, gateway request id, provider kind, and enforcement profile id. Successful provider-shaped responses, including blocked or escalated policy responses, complete the run. Provider failures and internal check failures mark the run as failed.

For OpenAI-compatible chat requests, the input check evaluates the latest `user` message only. Agent-owned `system` instructions are not treated as user input, so policies do not fire on the product's own safety prompt every turn, and earlier turns do not get concatenated into the current turn. Output checks still evaluate the provider's assistant reply.

## Self-Healing with `max_regenerations`

When `output_action` is `rewrite` and the engine does not return a pre-computed `safe_output`, the gateway can attempt to self-correct by re-sending the request to the provider with corrective feedback injected into the message history.

The enforcement profile's `max_regenerations` field caps the number of retry attempts (default 0 = no retries). On each attempt the gateway appends the failed assistant turn and a system-level correction message, then re-checks the new output. If any attempt passes, the clean response is returned to the caller with no enforcement headers. If all attempts are exhausted, the fallback message is returned with the standard enforcement headers.

This allows many borderline policy violations to resolve transparently without the caller's agent knowing a block was attempted.

## Retention

Gateway checks always evaluate the real prompt and output so policy enforcement is not weakened by retention settings. Gateway traces include route, provider, phase, action, and retention metadata. Raw prompt/output storage is controlled by the enforcement profile:

- `metadata_only` stores no raw body text in check payloads.
- `redacted_body` stores a placeholder.
- `full_body` stores bounded `checked_input_excerpt` and `checked_output_excerpt` fields on the persisted `Decision` payload so the run detail view can show what the policy evaluated.

## Provider Support

Supported surfaces:

- `POST /v1/gateway/{route_id}/openai/chat/completions`
- `POST /v1/gateway/{route_id}/anthropic/v1/messages`

Each enforcement profile carries a `response_mode` (`regular` | `streaming`). In `regular`
mode (the default) a `stream: true` request is rejected with `400`. In `streaming` mode the
gateway buffers the full upstream reply, runs the output guard, and then emits the guarded
result as provider-native Server-Sent Events — it never forwards unguarded tokens. This is
buffered emission, not token-by-token streaming; incremental mid-stream interruption is
future work.
