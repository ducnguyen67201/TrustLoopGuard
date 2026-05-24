# Gateway proxy chat-agent demo

This demo is the gateway version of a normal provider SDK integration.

The proxy agent talks to an OpenAI-compatible base URL:

```text
agent breaker -> proxy agent -> /v1/gateway/<route_id>/openai -> TrustLoopGuard -> mock provider
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

In another terminal, run the one-shot smoke test:

```sh
TL_API_KEY=dev-admin pnpm demo:proxy
```

For the networked demo, keep the proxy agent running in one terminal:

```sh
TL_API_KEY=dev-admin pnpm demo:proxy:agent
```

For the side-by-side arena, also run a raw vulnerable agent:

```sh
pnpm demo:raw-agent
```

Then open `http://localhost:3000/arena` from the web app and compare:

```text
Raw agent URL:     http://127.0.0.1:8787
Guarded agent URL: http://127.0.0.1:8788
```

For a terminal-only breaker run, attack the guarded agent from another terminal:

```sh
pnpm demo:agent-breaker
```

The proxy agent listens on `http://127.0.0.1:8788` by default. Override it with
`PROXY_AGENT_HOST` and `PROXY_AGENT_PORT`. Point the breaker at another agent
with `PROXY_AGENT_URL`.

## What It Sets Up

The proxy demo creates everything needed for a local gateway route:

- workspace
- policy that blocks `unsafe proxy reply`
- workspace runtime key
- OpenAI-compatible provider connection
- enforcement profile
- gateway route
- local mock provider

No paid provider is called.

The support agent prompt lives in `demo/proxy/agent-profile.ts`. In network
mode, the raw and guarded servers use `createArenaAdapter()` from
`demo/arena/adapter.ts` to expose that profile at `GET /arena/profile`; the
arena and CLI breaker use it to generate adversarial chat attempts and send them
to `POST /arena/chat`.

## What It Shows

The demo runs the generated chat breaker turns:

1. Clean prompt: the provider response passes through unchanged.
2. Breaker prompts: TrustLoopGuard returns an OpenAI-shaped
   `content_filter` response with correlation headers.

Expected output looks like:

```text
chat breaker: clean support turn
  finish : stop
  guard  : verdict=none phase=none trace=(none)

chat breaker: direct secret extraction
  finish : content_filter
  guard  : verdict=blocked phase=output trace=...
```

At the end it prints average and p95 gateway latency for the local run.
