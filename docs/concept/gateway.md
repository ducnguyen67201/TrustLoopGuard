# Gateway

Gateway is TrustLoopGuard's provider-proxy integration. It lets an application send OpenAI-compatible
or Anthropic traffic through the Rust runtime instead of applying a returned `AuthorizationDecision` itself.

## Ownership

- `tl-server` owns the provider-compatible data plane and configuration APIs.
- `tl-storage` persists provider connections and routes.
- `tl-core` owns the public Gateway wire types.
- `apps/web` proxies dashboard configuration requests and renders the setup UI.

The Next.js app never forwards model traffic or evaluates policies.

## SDK and Gateway modes

SDK mode returns a decision to customer code:

```text
Customer app -> SDK -> POST /v1/events -> AuthorizationDecision -> customer applies effect
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

Provider connections can be edited without replacing the stored credential; supplying a new key
rotates it, while omitting the key preserves the existing encrypted value. Deletion is a permanent
removal of the provider row and encrypted credential. A provider referenced by any Gateway route
cannot be deleted; the route must be moved to another provider first. Provider kind and stable id
do not change during edits.

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

## Authorization effects

Gateway applies the `AuthorizationDecision` from the shared event service directly:

| Effect | Input | Output |
|---|---|---|
| `permit` | Forward unchanged | Return unchanged |
| `transform` | Replace the latest user message with `transformed_value` | Replace provider content with `transformed_value` |
| `deny` | Do not call the provider | Suppress provider content |
| `require_approval` | Do not call the provider until authority is supplied | Suppress provider content |
| `defer` | Do not call the provider until evidence/system state changes | Suppress provider content |

A transform without `transformed_value` fails closed. Denied responses use the stable message
`Denied by TrustLoopGuard.` and the provider's normal content-filter shape. Gateway does not call
the provider again to regenerate a response.

Provider failures are availability failures, not policy decisions. They return a sanitized
`502 Bad Gateway` and mark the Gateway run failed.

## Response signals

Non-permit responses include:

- provider-native `content_filter` finish/stop reason
- `X-TrustLoopGuard-Effect`
- `X-TrustLoopGuard-Phase`
- `X-TrustLoopGuard-Trace-Id`
- `X-TrustLoopGuard-Policy-Id` when a policy id is available

Transforms include the effect and correlation headers. Permitted responses carry no enforcement
headers.

## Budgets and metering

OpenAI-compatible chat completions retain one Gateway-specific deterministic stage. Before any
provider spend, Gateway evaluates enabled financial policies with `meter: llm_usage` for the
runtime-key principal. It calculates a conservative input-token ceiling from the serialized request,
uses `max_tokens` or `max_completion_tokens` as the output ceiling, and atomically reserves that
maximum cost against the tightest daily, weekly, and monthly caps. Postgres serializes bounded
reservations per workspace/principal, so concurrent replicas cannot spend the same remaining budget.

When neither output bound is present, Gateway uses soft admission: it allows the request while
committed spend remains below every matching cap, settles the provider's reported actual usage,
then denies later requests after the cap is reached. This preserves compatibility with clients that
omit output bounds, but one unbounded request—or multiple concurrent unbounded requests—can
overshoot a cap. Runs label this path `soft_admitted` and `soft_settled`; it is not a hard-cap
guarantee.

After a successful provider response, the reservation is settled to the provider's actual usage.
Unused reserved budget becomes available immediately. Provider failure releases the reservation;
missing usage keeps a bounded request's maximum reservation active. An unbounded response without
usage remains unknown and cannot provide a hard-cap guarantee. Usage events retain USD-nano
precision and expose those exact values as decimal strings alongside legacy rounded minor-unit
fields. Each recorded price snapshot therefore remains auditable even after a model's configured
price changes.

This is still the unified policy registry, but it is not part of generic `/v1/events` evaluation:
authoritative token usage and price are known only around the provider call. At or over a matching
cap, Gateway returns HTTP 429 before calling the provider. A budgeted request fails closed when its
model has no trusted price. Bounded requests receive a hard admission boundary assuming the
upstream provider honors its token bound and reports authoritative usage; unbounded requests use
the soft behavior above.

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

Each accepted Gateway request creates or reuses a `chat_session` run for the route agent. The
checked input attaches to a `user_turn`; a successful provider response creates a separate
`assistant_turn`, and the output check links to that assistant event. Gateway checks produce the
same persisted traces as SDK events, using `gateway_input` and `gateway_output` domains so the run
UI places them in the correct guard phase.

Run metadata records integration mode, route id, Gateway request id, and provider kind. Callers may
send `X-TLG-Run-External-Id` to group multiple provider calls into one upstream session. Policy-
shaped responses complete the run; provider and internal check failures mark it failed.

`GET /v1/runs/{run_id}` joins the run timeline with typed Gateway evidence:

- provider/model, provider response id, status, latency, prompt/completion/total tokens, the price
  snapshot, and estimated customer-inference cost;
- the latest deterministic LLM budget decision, including every configured window's committed,
  reserved, cap, and remaining amounts;
- one guardrail-overhead record per semantic judge invocation, kept separate from customer spend.

The durable usage ledger distinguishes `customer_inference` from `guardrail`. Only customer
inference participates in `meter: llm_usage` hard-cap admission. Semantic policy candidates remain
deterministically prefiltered and are judged in one batched call per event, so the number of enabled
policies does not create an LLM-call loop. Failed or timed-out judge calls show unknown usage and
cost when the provider did not report tokens; they are never displayed as zero-cost calls.

The dashboard's **Usage & budgets** page is the operator home for model prices, aggregate customer
usage, guardrail overhead, `meter: llm_usage` policies, and LLM-scoped alerts. Gateway route setup
only reports whether those spend controls are ready and links to that page. Provider invoices remain
the final billing authority; TrustLoopGuard's per-request cost is a price-snapshot estimate.

OpenAI-compatible input checks evaluate the latest user message. Anthropic input checks also include
the top-level system prompt. Output checks evaluate the provider's assistant response.

## Supported endpoints

- `GET /v1/gateway/provider-connections`
- `POST /v1/gateway/provider-connections`
- `PATCH /v1/gateway/provider-connections/{id}`
- `DELETE /v1/gateway/provider-connections/{id}`
- `GET /v1/gateway/routes`
- `POST /v1/gateway/routes`
- `PATCH /v1/gateway/routes/{id}`
- `POST /v1/gateway/{route_id}/openai/chat/completions`
- `POST /v1/gateway/{route_id}/anthropic/v1/messages`

Payment-provider forwarding is a separate financial path and is not configured through a Gateway
route.
