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
`TL_GATEWAY_CREDENTIAL_KEY`.

Start the Rust API:

```bash
doppler run --project trustloopguard --config dev_stripe_demo -- cargo run -p tl-server
```

In a second terminal, install the refund policy, mandate, and local provider connection:

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
pnpm --filter marketing dev
```

Open [http://localhost:3002/demo](http://localhost:3002/demo). Try `$25` for automatic execution,
`$75` for a human-approval hold, and `$125` for a block.

## Deploy

Deploy the refund-agent service on a private server with the same Doppler config, expose only its
chat endpoint through an HTTPS origin, and set `REFUND_DEMO_SERVICE_URL` on the marketing app. The
marketing proxy enforces input limits, a small process-local rate limit, an upstream timeout, and a
strict public response schema. Use a shared rate limiter before sending significant launch traffic
across multiple marketing instances.
