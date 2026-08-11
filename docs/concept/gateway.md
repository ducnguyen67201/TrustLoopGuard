# Gateway

Gateway is Featherlane AI's provider-proxy integration. It lets an application send OpenAI-compatible
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
- a reliability mode and an ordered list of same-kind fallback connections

Every enabled policy for the active workspace environment and route agent applies automatically.
Routes do not select or override policies.

Dashboard/user credentials manage provider connections and routes. Workspace runtime keys
(`tl_live_...`) can call Gateway model endpoints but cannot manage Gateway configuration or other
runtime keys.

Provider credentials are encrypted with `TL_GATEWAY_CREDENTIAL_KEY`. Development may fall back to
`TL_API_KEY`; without either secret, credential sealing requires the explicit local-only
`TL_GATEWAY_ALLOW_INSECURE_DEV_KEY` escape hatch.

The guided production-loop activation reconciles the provider, agent, route, deterministic starter
evaluations, email rule, and workspace privacy choice through one Rust-owned control-plane action.
Stable ids are reused only when the existing provider, agent, route, and starter-policy semantics
are compatible; otherwise activation returns a conflict with `activation_step` and the ids already
ready. It reports each readiness check independently so an interrupted activation can repeat the
same request without creating a parallel source of truth. Provider and runtime key plaintext are
one-time values and are never returned by readiness APIs.

Activation accepts one exact `verification_session_id` (or generates one) before configuration is
returned. Readiness is `ready` only after traffic with that external id reaches the activated route,
the matching Run is terminal through the finalization boundary, and its non-empty deterministic
manifest reaches a terminal result other than `not_configured`. Configuration rows alone never
satisfy those traffic checks. `GET /v1/gateway/routes/{id}/production-readiness` accepts that value
as `external_id`; the dashboard proxy forwards it unchanged along with workspace/environment
context.

An operator may explicitly set `alerts_deferred`. That skips rule creation but deliberately leaves
the email-rule and transport checks in `needs_attention`; omission of an email without that explicit
choice is invalid. The guided form offers one same-protocol fallback for the bounded standard plan;
the route contract retains an ordered fallback list for control-plane clients.

The dashboard setup flow is:

1. Connect a provider and create or select an agent.
2. Activate the route, optional fallback, starter evaluations, alert rule (or explicitly defer it),
   and privacy mode.
3. Create a workspace runtime API key if none is active.
4. Copy the route-specific provider configuration and send a harmless verification request.
5. Confirm the exact Run, evaluation, and notification readiness checks.

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
`Denied by Featherlane AI.` and the provider's normal content-filter shape. Gateway does not call
the provider again to regenerate a response.

Provider failures are availability failures, not policy decisions. They return a sanitized
`502 Bad Gateway` and mark the Gateway run failed.

## Provider reliability

Existing routes default to `none`: Gateway makes exactly one call to the primary connection.
`standard` uses a bounded plan: one primary call, one retry of that primary only for a retryable
transport/408/429/5xx failure, then one same-kind fallback. `Retry-After` is bounded by the total
request deadline. Authentication, authorization, client, and provider response-shape errors do not
amplify calls. Payment HTTP connections are never LLM fallbacks.

Each attempt has its own budget reservation identity and durable evidence record. Run detail shows
attempt order, provider connection, model, latency, usage, cost, and sanitized failure code. A
fallback success completes the Run; a provider-terminal notification is queued only when the
bounded plan is exhausted. Notification delivery semantics live in
[notifications.md](notifications.md).

## Response signals

Gateway responses include `X-Featherlane-Run-Id` and `X-Featherlane-Session-State` so callers can
correlate the automatically captured Run. Non-permit responses also include:

- provider-native `content_filter` finish/stop reason
- `X-Featherlane AI-Effect`
- `X-Featherlane AI-Phase`
- `X-Featherlane AI-Trace-Id`
- `X-Featherlane AI-Policy-Id` when a policy id is available

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

Run metadata records integration mode, route id, Gateway request id, and provider kind. With no
session header, every request is a one-request Run and finalizes after its guarded response. A caller
groups turns by sending a stable `X-Featherlane-Session-Id`; the legacy
`X-FEATHERLANE-AI-Run-External-Id` header remains compatible. When both are present they must match.
`X-Featherlane-Session-End: true` finalizes after that response. Otherwise a Rust worker finalizes
idle sessions and times out sessions that exceed the configured maximum duration. Concurrent first
turns for the same route agent and session converge on one active Run. A later request with the same
session ID creates a new Run after the previous one is terminal.

Session IDs are opaque correlation metadata, not authorization inputs. They are length-bounded and
must not contain secrets or personal data. Every terminal path uses the same transactional Run
finalization boundary, which in turn closes capture and schedules post-Run evaluation.

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
the final billing authority; Featherlane AI's per-request cost is a price-snapshot estimate.

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
- `POST /v1/gateway/activations`
- `GET /v1/gateway/routes/{id}/production-readiness`
- `POST /v1/gateway/{route_id}/openai/chat/completions`
- `POST /v1/gateway/{route_id}/anthropic/v1/messages`

Payment-provider forwarding is a separate financial path and is not configured through a Gateway
route.
