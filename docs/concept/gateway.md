# Gateway

Gateway is TrustLoopGuard's provider-proxy integration. It lets an application send OpenAI-compatible
or Anthropic traffic through the Rust runtime instead of applying a returned `Decision` itself.

## Ownership

- `tl-server` owns the provider-compatible data plane and configuration APIs.
- `tl-storage` persists provider connections and routes.
- `tl-core` owns the public Gateway wire types.
- `apps/web` proxies dashboard configuration requests and renders the setup UI.

The Next.js app never forwards model traffic or evaluates policies.

## SDK and Gateway modes

SDK mode returns a decision to customer code:

```text
Customer app -> SDK -> POST /v1/events -> Decision -> customer applies verdict
```

Gateway mode applies that same decision to provider traffic:

```text
Customer app
  -> Gateway route
  -> input GuardEvent
  -> execute_event_submission
  -> provider
  -> output GuardEvent
  -> execute_event_submission
  -> guarded provider response
```

Both paths use `execute_event_submission`, including built-in checkers, enabled policies, trace
persistence, and escalation delivery. Gateway does not have a second rule or enforcement-profile
layer. The canonical event pipeline is documented in [event-engine.md](event-engine.md).

## Configuration

A provider connection stores the customer-owned provider credential, provider kind, base URL, and
default model. Plaintext provider credentials are never returned after creation.

A route binds a stable public route id to:

- one provider connection
- one agent id

Every enabled policy for the active workspace environment and route agent applies automatically.
Routes do not select or override policies.

Dashboard/user credentials manage provider connections and routes. Workspace runtime keys
(`tl_live_...`) can call Gateway model endpoints but cannot manage Gateway configuration or other
runtime keys.

Provider credentials are encrypted with `TL_GATEWAY_CREDENTIAL_KEY`. Development may fall back to
`TL_API_KEY`; without either secret, credential sealing requires the explicit local-only
`TL_GATEWAY_ALLOW_INSECURE_DEV_KEY` escape hatch.

The dashboard setup flow is:

1. Connect a provider.
2. Create or select an agent.
3. Create a route binding the provider and agent.
4. Create a workspace runtime API key.
5. Copy the route-specific client configuration shown under Routes.

OpenAI-compatible clients use:

```text
baseURL = https://<server>/v1/gateway/<route_id>/openai
```

Anthropic clients use:

```text
baseURL = https://<server>/v1/gateway/<route_id>/anthropic
```

## Policy verdicts

Gateway applies the `Decision` from the shared event service directly:

| Verdict | Input | Output |
|---|---|---|
| `allow` | Forward unchanged | Return unchanged |
| `rewrite` | Replace the latest user message with `safe_output` | Replace provider content with `safe_output` |
| `block` | Do not call the provider | Suppress provider content |
| `escalate` | Do not call the provider; escalation is queued | Suppress provider content; escalation is queued |

A rewrite without `safe_output` fails closed. Blocked responses use the stable message
`Blocked by TrustLoopGuard.` and the provider's normal content-filter shape. Gateway does not call
the provider again to regenerate a response.

Provider failures are availability failures, not policy decisions. They return a sanitized
`502 Bad Gateway` and mark the Gateway run failed.

## Response signals

Blocked and escalated responses include:

- provider-native `content_filter` finish/stop reason
- `X-TrustLoopGuard-Verdict`
- `X-TrustLoopGuard-Phase`
- `X-TrustLoopGuard-Trace-Id`
- `X-TrustLoopGuard-Policy-Id` when a policy id is available

Rewrites include the verdict and correlation headers. Allowed responses carry no enforcement
headers.

## Budgets and metering

OpenAI-compatible chat completions retain one Gateway-specific deterministic stage. Before any
provider spend, Gateway evaluates enabled financial policies with `meter: llm_usage` for the
runtime-key principal. It calculates a conservative input-token ceiling from the serialized request,
uses `max_tokens` or `max_completion_tokens` as the output ceiling, and atomically reserves that
maximum cost against the tightest daily, weekly, and monthly caps. Postgres serializes reservations
per workspace/principal, so concurrent replicas cannot spend the same remaining budget.

After a successful provider response, the reservation is settled to the provider's actual usage.
Unused reserved budget becomes available immediately. Provider failure releases the reservation;
missing usage keeps it active so unmeasured spend cannot reopen the cap. Usage events retain
USD-nano precision internally while public totals remain denominated in USD minor units.

This is still the unified policy registry, but it is not part of generic `/v1/events` evaluation:
authoritative token usage and price are known only around the provider call. At or over a matching
cap, Gateway returns HTTP 429 before calling the provider. A budgeted request fails closed when its
model has no trusted price or has no positive output-token bound. This is a hard admission boundary
assuming the upstream provider honors its token bound and reports authoritative usage.

OpenAI-compatible providers such as DigitalOcean can use their normal base URL, for example
`https://inference.do-ai.run`, with the desired model configured as the provider default.

## Streaming

`stream: true` needs no route-level setting. Gateway removes upstream streaming fields, buffers the
complete provider response, evaluates output policies, then emits the guarded result as
provider-native Server-Sent Events. This prevents unguarded tokens from reaching the caller. It is
buffered emission, not token-by-token upstream streaming.

## Data handling

Gateway-created `GuardEvent`s use the same workspace data-handling gate as `/v1/events`. Gateway
does not own a route-level retention setting and must not claim to redact or omit raw event text
independently of that shared workspace behavior.

## Observability

Each accepted Gateway request creates or reuses a `chat_session` run for the route agent. Input and
output checks attach to a `user_turn` event and produce the same persisted traces as SDK events.

Run metadata records integration mode, route id, Gateway request id, and provider kind. Callers may
send `X-TLG-Run-External-Id` to group multiple provider calls into one upstream session. Policy-
shaped responses complete the run; provider and internal check failures mark it failed.

OpenAI-compatible input checks evaluate the latest user message. Anthropic input checks also include
the top-level system prompt. Output checks evaluate the provider's assistant response.

## Supported endpoints

- `POST /v1/gateway/{route_id}/openai/chat/completions`
- `POST /v1/gateway/{route_id}/anthropic/v1/messages`

Payment-provider forwarding is a separate financial path and is not configured through a Gateway
route.
