# Gateway proxy chat-agent demo

This demo is the gateway version of a normal provider SDK integration.

The application talks to an OpenAI-compatible base URL:

```text
chat agent -> /v1/gateway/<route_id>/openai -> TrustLoopGuard -> mock provider
```

TrustLoopGuard owns the policy checks. The app still receives an
OpenAI-compatible response.

## Run

Start the Rust server:

```sh
TL_API_KEY=dev-admin \
TL_GATEWAY_CREDENTIAL_KEY=local-demo-gateway-secret \
cargo run -p tl-server
```

In another terminal, run the demo:

```sh
TL_API_KEY=dev-admin pnpm demo:proxy
```

## What It Sets Up

The script creates everything needed for a local gateway route:

- workspace
- policy that blocks `unsafe proxy reply`
- workspace runtime key
- OpenAI-compatible provider connection
- enforcement profile
- gateway route
- local mock provider

No paid provider is called.

## What It Shows

The demo runs two chat turns:

1. Clean prompt: the provider response passes through unchanged.
2. Unsafe provider output: TrustLoopGuard returns an OpenAI-shaped
   `content_filter` response with correlation headers.

Expected output looks like:

```text
chat scenario: clean support turn
  finish : stop
  guard  : verdict=none phase=none trace=(none)

chat scenario: unsafe provider output
  finish : content_filter
  guard  : verdict=blocked phase=output trace=...
```

At the end it prints average and p95 gateway latency for the local run.
