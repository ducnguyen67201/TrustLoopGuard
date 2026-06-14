# Gateway Proxy Runtime Branch Guide

This branch adds gateway mode: a provider-compatible proxy path for customers who do not want to submit `POST /v1/events` manually around every model response.

The canonical product concept is still `docs/concept/gateway.md`. This document is a branch walkthrough: what changed, how requests move through the system, and how to read the implementation.

## One-Sentence Model

TrustLoopGuard becomes the base URL for the customer's OpenAI-compatible or Anthropic SDK, checks the prompt before the provider call, forwards the request to the real provider using a stored provider key, checks the provider output, and returns a provider-shaped response.

It is a man-in-the-middle at the application HTTP layer, not a transparent network proxy. The customer opts in by configuring their SDK `baseURL`.

## SDK Mode vs Gateway Mode

SDK mode:

```text
customer app -> provider SDK -> provider
customer app -> TrustLoopGuard SDK -> POST /v1/events -> Decision
customer app decides whether to send, block, rewrite, or escalate
```

Gateway mode:

```text
customer app -> OpenAI/Anthropic SDK with TrustLoopGuard baseURL
TrustLoopGuard Rust gateway -> input check
TrustLoopGuard Rust gateway -> real OpenAI/Anthropic endpoint
TrustLoopGuard Rust gateway -> output check
TrustLoopGuard Rust gateway -> provider-compatible response
```

The important difference is who applies enforcement. In SDK mode, customer code receives a `Decision`. In gateway mode, the Rust server applies the route's enforcement profile before the customer sees the model response.

## How a Customer Routes Through the Proxy

OpenAI-compatible clients point at the route-specific OpenAI-compatible base URL:

```ts
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: process.env.TLG_API_KEY,
  baseURL: 'https://api.gettrustloop.app/v1/gateway/<route_id>/openai',
});

const response = await openai.chat.completions.create({
  model: 'gpt-4o-mini',
  messages: [{ role: 'user', content: userMessage }],
});
```

That SDK call lands on:

```text
POST /v1/gateway/{route_id}/openai/chat/completions
```

Anthropic clients point at the route-specific Anthropic base URL:

```ts
import Anthropic from '@anthropic-ai/sdk';

const anthropic = new Anthropic({
  authToken: process.env.TLG_API_KEY,
  baseURL: 'https://api.gettrustloop.app/v1/gateway/<route_id>/anthropic',
});
```

That SDK call lands on:

```text
POST /v1/gateway/{route_id}/anthropic/v1/messages
```

The credential in these examples is the TrustLoopGuard runtime API key. The actual OpenAI or Anthropic provider key is stored in the gateway provider connection.

OpenAI's SDK sends the runtime key as a bearer token when configured with `apiKey`. Anthropic's normal `apiKey` path sends `x-api-key`, while the current Rust auth middleware accepts bearer tokens. Use Anthropic's bearer-token configuration, add an explicit `Authorization` default header, or extend the middleware before documenting `apiKey` as the Anthropic gateway auth path.

## Configuration Objects

Gateway mode needs three durable objects:

```text
Provider connection
  Provider kind: openai_compatible or anthropic
  Provider base URL: optional override
  Default model
  Encrypted provider API key

Enforcement profile
  What to do on input matches
  What to do on output matches
  Fail-open or fail-closed behavior
  Retention mode
  Fallback message
  Regeneration budget

Gateway route
  Public route id
  Provider connection id
  Agent id
  Enforcement profile id
```

The route is what the customer puts into the SDK `baseURL`.

## Request Flow in Rust

The core path lives under `crates/tl-server/src/gateway/`.

```text
proxy_openai_chat_completions
or proxy_anthropic_messages
  -> proxy_provider_request
    -> resolve gateway route for workspace + route_id
    -> validate provider kind matches endpoint
    -> parse JSON body and reject streaming
    -> decrypt stored provider API key
    -> extract input text from provider request
    -> check_gateway_content(... gateway_input_check ...)
    -> apply input action:
         allow  -> continue
         block  -> return provider-shaped content_filter fallback
         redact -> rewrite latest user message before forwarding
    -> forward request to real provider
    -> extract output text from provider response
    -> check_gateway_content(... gateway_output_check ...)
    -> apply output action:
         allow    -> return provider response
         block    -> return provider-shaped content_filter fallback
         escalate -> return provider-shaped content_filter fallback with escalated header
         rewrite  -> use safe_output, retry regeneration, or fallback
```

Both input and output checks build `GuardEvent`s and call the same runtime path as `POST /v1/events` through `execute_event_submission`.

## Provider Forwarding

OpenAI-compatible forwarding:

```text
TrustLoopGuard endpoint:
  /v1/gateway/{route_id}/openai/chat/completions

Real provider endpoint:
  {provider_connection.base_url or https://api.openai.com}/v1/chat/completions

Credential sent upstream:
  Authorization: Bearer <stored provider_api_key>
```

Anthropic forwarding:

```text
TrustLoopGuard endpoint:
  /v1/gateway/{route_id}/anthropic/v1/messages

Real provider endpoint:
  {provider_connection.base_url or https://api.anthropic.com}/v1/messages

Credential sent upstream:
  x-api-key: <stored provider_api_key>
  anthropic-version: 2023-06-01
```

If the incoming request has no `model`, the gateway fills in the provider connection's `default_model`.

## What "Man in the Middle" Means Here

Accurate:

- TrustLoopGuard sits between the customer's app and the model provider.
- The customer calls TrustLoopGuard instead of calling OpenAI or Anthropic directly.
- TrustLoopGuard can inspect and modify the request before forwarding.
- TrustLoopGuard can inspect, replace, or suppress the provider response before returning it.

Not accurate:

- It is not a transparent TLS/network proxy.
- It does not intercept traffic without application changes.
- The dashboard web app is not the data-plane proxy.
- Customer runtime checks do not go through `apps/web`.

The data plane is Rust:

```text
customer app -> crates/tl-server gateway endpoint -> provider
```

The dashboard is only for configuration:

```text
browser -> apps/web same-origin API route -> crates/tl-server config API
```

## Response Shape on Enforcement

When the gateway blocks or escalates, it returns a normal-looking provider response with content-filter semantics:

```text
OpenAI-compatible:
  choices[0].finish_reason = "content_filter"

Anthropic:
  stop_reason = "content_filter"
```

It also adds TrustLoopGuard headers:

```text
X-TrustLoopGuard-Verdict: blocked | escalated
X-TrustLoopGuard-Phase: input | output
X-TrustLoopGuard-Trace-Id: <trace id>
X-TrustLoopGuard-Policy-Id: <policy id when available>
```

Clean allowed responses do not get these headers.

## Dashboard Proxy vs Runtime Proxy

There are two different proxy ideas in this branch.

Dashboard proxy:

```text
apps/web/app/api/gateway/*
apps/web/app/api/enforcement-profiles/*
```

These routes exist so the browser can call same-origin Next.js endpoints. They forward dashboard configuration requests to Rust. They do not forward model traffic.

Runtime provider proxy:

```text
crates/tl-server/src/gateway/
POST /v1/gateway/{route_id}/openai/chat/completions
POST /v1/gateway/{route_id}/anthropic/v1/messages
```

These routes are the actual OpenAI/Anthropic-compatible data plane.

## Files to Read in Order

Start here:

1. `docs/concept/gateway.md` for the product concept.
2. `sdks/typescript/README.md` for the customer-facing SDK/baseURL examples.
3. `crates/tl-core/src/gateway.rs` for the wire types.
4. `crates/tl-storage/migrations/00000000000016_gateway/up.sql` for the durable schema.
5. `crates/tl-storage/src/gateway_repo.rs` for persistence.
6. `crates/tl-server/src/gateway/` for the runtime proxy and enforcement flow.
7. `crates/tl-server/tests/gateway.rs` for executable examples of blocked input, output rewrite, provider forwarding, and regeneration.
8. `apps/web/app/gateway/page.tsx` for the dashboard read-only configuration view.
9. `apps/web/lib/server/proxy-helpers.ts` for the thin Next.js dashboard proxy helpers.

## Current Limits

- Streaming is explicitly rejected for now.
- Only OpenAI-compatible chat completions and Anthropic messages are exposed.
- Gateway mode applies provider-compatible fallback responses, not raw `Decision` objects.
- The customer must configure provider connections, enforcement profiles, and routes before model traffic can flow.
- Provider credentials are stored server-side and encrypted with `TL_GATEWAY_CREDENTIAL_KEY`.
