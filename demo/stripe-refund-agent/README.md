# Live Stripe refund demo

The marketing route at `/demo` is a UI over this service. It is deliberately not a replay or a
scripted animation. Every request:

1. creates and captures a fresh `$100` PaymentIntent in Stripe test mode;
2. sends the customer's message to an OpenAI tool-calling agent;
3. looks up trusted order evidence in the demo customer backend;
4. submits a typed refund to the TrustLoopGuard Rust API;
5. calls Stripe's refund API only when TrustLoopGuard authorizes execution.

The Stripe and OpenAI keys remain server-side in Doppler. The code refuses `sk_live_` keys and the
public route redacts internal order, payment-intent, and provider-credential fields.

## Run locally

The `trustloopguard/dev_stripe_demo` Doppler config must contain `OPENAI_API_KEY`,
`STRIPE_SECRET_KEY`, `TL_API_KEY`, `TL_SERVER_URL`, `DATABASE_URL`, and
`TL_GATEWAY_CREDENTIAL_KEY`. It must also contain a dedicated, randomly generated
`REFUND_DEMO_PROXY_SECRET` of at least 32 characters. Do not reuse another application key.

Start the Rust API:

```bash
doppler run --project trustloopguard --config dev_stripe_demo -- cargo run -p tl-server
```

In a second terminal, install the refund policy, reusable authorization grant, and local provider connection:

```bash
doppler run --project trustloopguard --config dev_stripe_demo -- \
  pnpm --filter @trustloopguard/demo stripe-refund-agent:setup
```

Start the Stripe provider adapter and refund-agent service:

```bash
pnpm --filter @trustloopguard/demo stripe-refund-agent:live
```

Start the marketing app:

```bash
doppler run --project trustloopguard --config dev_stripe_demo -- \
  pnpm --filter marketing dev
```

Open [http://localhost:3002/demo](http://localhost:3002/demo). Try `$25` for automatic execution,
`$75` for a human-approval hold, and `$125` for a block.

## Deploy

Deploy one refund-agent service on a private server with the same Doppler config, expose only its
authenticated chat endpoint through an HTTPS origin, and set `REFUND_DEMO_SERVICE_URL` plus the same
`REFUND_DEMO_PROXY_SECRET` on the marketing app. The marketing proxy enforces input limits, a bounded
per-visitor throttle, an upstream timeout, and a strict public response schema. The central refund
service independently authenticates every mutation and caps the total number of expensive runs, so
multiple marketing instances share one launch budget. If the refund service itself is scaled beyond
one instance, replace that central in-process budget with a shared durable limiter first.

The marketing edge permits 10 runs per platform-reported client address in a rolling 24-hour window.
That visitor counter is process-local: it resets when the marketing process restarts and is not shared
between replicas. Use a shared rate-limit store before scaling the marketing app beyond one instance
or when an exact daily quota is required. The refund service separately retains its 60-run global
circuit breaker per 10-minute window.

The deployed refund-agent service must read a production-scoped `TL_API_KEY` from Doppler and set
`TL_SERVER_URL=https://api.gettrustloop.app`. The Rust `/v1` API lives on `api.gettrustloop.app`;
`https://app.gettrustloop.app` is the authenticated dashboard and is used only for held-action review
links. Do not copy either key into the marketing app or expose it through a `NEXT_PUBLIC_*` variable.

Railway deploys the service with `demo/stripe-refund-agent/Dockerfile`. Set `PORT=8080`,
`STRIPE_REFUND_AGENT_UI_HOST=0.0.0.0`, a dedicated 32+ character
`STRIPE_REFUND_PROVIDER_API_KEY`, and `STRIPE_REFUND_PROVIDER_BASE_URL` to the service's public HTTPS
origin. The same service exposes the proxy-authenticated `/chat` and `/status/*` routes plus the
separately authenticated `/payments` provider adapter. Run `stripe-refund-agent:setup` once per
environment after deployment so the Rust API stores the hosted provider origin and credential.
