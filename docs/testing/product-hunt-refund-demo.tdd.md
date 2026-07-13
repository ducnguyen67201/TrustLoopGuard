# Product Hunt refund demo: TDD evidence

## User journey

1. A visitor opens `/demo` and asks the support agent for a refund.
2. The demo creates a fresh, captured $100 payment in Stripe test mode.
3. OpenAI selects the order lookup and refund tools from the visitor's request.
4. The refund action is proposed to the TrustLoopGuard Rust API before Stripe is called.
5. The UI shows the guard decision, execution trace, audit identifiers, and Stripe result.

The three suggested amounts exercise the public story:

- $25 is allowed and creates a real Stripe test refund.
- $75 is held for human approval and does not call Stripe's refund endpoint.
- $125 is blocked and does not call Stripe's refund endpoint.

## RED

Commit `4285b6e3` introduced the public contract and live-agent requirements before implementation.
The focused suite failed because the `/demo` contract did not exist and live mode still fell back to
the scripted agent. A separate receipt assertion failed because the Stripe reference was read from
the wrong proof path.

## GREEN

The implementation adds a validated same-origin proxy, public-field redaction, rate limiting,
upstream timeouts, a live-only OpenAI path, fresh Stripe test payments, and the `/demo` interface.

Verification commands:

```text
pnpm test:refund-demo
pnpm --filter @trustloopguard/demo typecheck
pnpm --filter marketing typecheck
pnpm --filter @trustloopguard/demo stripe-refund-agent:check
pnpm --filter marketing build
```

Focused result: 9 tests passed. Contract coverage is 100% line / 90% branch / 100% function.
The proxy route is 82.89% line covered. Aggregate coverage is not representative because importing
the existing demo agent also loads the full SDK/demo graph into Node's coverage denominator.

## Review hardening

The PR review identified concurrency, authentication, error-redaction, upstream-validation, and
throttling gaps. RED checkpoints `9513d702` and `312f0bae` captured the failing reproducers before
production changes. The GREEN suite now contains 16 passing tests and guarantees that:

- concurrent demo seeds use independent SQLite files and Stripe PaymentIntent IDs;
- the upstream service accepts only the exact 32+ character proxy bearer credential;
- every marketing instance shares the central refund-service request budget;
- platform-owned client addresses take precedence over spoofable forwarding values;
- internal run logs are removed from the browser response;
- malformed upstream success payloads return a generic `502`, not a client `400`.

Focused coverage after hardening: proxy route 90.24% lines, public contract 98.85% lines, proxy auth
and central budget 100% lines/branches/functions, and live Stripe seeding 92.5% lines. Aggregate
coverage remains diluted by the imported generated SDK graph and is not used as the focused gate.

The hardened live path was also exercised with two simultaneous requests through separate local
ports while the existing developer stack remained untouched. An unauthenticated direct `/chat`
request returned `401`. The concurrent `$25` and `$75` runs produced distinct action IDs: the `$25`
run executed a real Stripe test refund and returned a 7,500 minor-unit balance, while the `$75` run
remained held with a 10,000 balance and no refund. Neither public response contained `logs`.

## Local repeatability follow-up

As a local demo operator, I can run more than four refund scenarios without waiting for the public
launch throttle to reset. RED checkpoint `8763554b` reproduced the bug with statuses
`200, 200, 200, 200, 429, 429`. The GREEN implementation bypasses only the marketing edge throttle
outside production; production still enforces the per-visitor limit, and the authenticated refund
service retains its central expensive-run budget in every environment.

`pnpm test:refund-demo` passes 17 tests, marketing typecheck and production build pass, and focused
coverage reports 89.60% lines for the proxy route and 92.45% lines across the route plus public
contract.

## Live integration evidence

The local Product Hunt route was exercised through the same-origin marketing proxy with Doppler
config `dev_stripe_demo`, the Rust API, OpenAI, and Stripe test mode:

- $25: `executed`; Stripe returned a real `re_...` refund reference.
- $75: `held`; zero Stripe refunds were created.
- $125: `denied`; zero Stripe refunds were created.

No production Stripe key is accepted by the demo. Secret values are never returned to the browser.

`pnpm secrets:check` passes. `pnpm audit --prod` reports 18 pre-existing low/moderate advisories
under the unrelated `apps/web` and `apps/docs` dependency trees; this change adds no package and
does not introduce those vulnerable paths into the marketing demo.

## Documentation impact

No Rust wire contract or architecture boundary changed. The feature is a marketing/demo client of
the existing public action API, so concept documentation does not require an update.
