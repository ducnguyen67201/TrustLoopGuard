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

## External approval synchronization

As a demo viewer, I can leave a `$75` refund held, approve that exact action in the TrustLoopGuard
dashboard, and watch the original `/demo` page advance to executed with its receipt and Stripe
reference. RED checkpoint `800566f2` captured the missing authenticated status reader, redacted
proxy, browser merge model, and polling hook.

The GREEN path polls only the UUID returned by the original run, refuses non-demo financial actions,
does not consume the expensive-run budget for status reads, and stops polling on a terminal state.
`pnpm test:refund-demo` passes 23 tests; both demo and marketing typechecks pass; the production
marketing build passes. Focused coverage is 95.45% lines overall: the status reader is 100%, status
model 85.71%, proxy route 90.68%, and public contract 99.15%.

Live verification read executed action `019f5d63-f8ca-77c3-ae7f-07b122daa7b3` through the new
same-origin status route and returned its real Stripe test reference
`re_3TsrRG730BwJXVLd0d71BxUd` without exposing internal action proof.

## Exact approval targeting

Multiple held demo runs made it possible to approve an older ledger row while the current `/demo`
continued waiting. RED checkpoint `8a113420` reproduces the ambiguity. The GREEN flow gives each
held run a direct dashboard link containing its exact action ID, and the financial ledger filters
to that one action before presenting Approve.

The refund-demo suite passes 24 tests, the web suite passes 226 tests, and both app typechecks pass.
Focused coverage is 91.94% lines across the public demo contract, review URL, and status model; the
financial ledger component is 95.5% lines and 80.55% branches.
Live verification moved action `019f5d6a-f57d-7c23-ada2-acc821b332ea` from held to authorized to
executed and returned Stripe test refund `re_3TsrYd730BwJXVLd00yeIDwD` through the public status
route.

## Daily visitor throttle

As an anonymous demo visitor, I can run at most 10 expensive refund workflows in a rolling 24-hour
window, limiting launch cost without requiring an account. The platform-provided client address is
the visitor identity; spoofable forwarding values do not override a platform-owned address.

RED checkpoint `44880ec7` changed the contract to expect 10 successful responses followed by a
`429`. Against the old implementation, requests 5 through 11 returned `429`, proving the previous
4-per-10-minute limit was still active. GREEN checkpoint `07e5a925` changes the edge window to 24
hours and the visitor maximum to 10. `pnpm test:refund-demo` passes all 24 tests and verifies that the
eleventh request is blocked, a request succeeds after the 24-hour boundary, and spoofed forwarded
addresses cannot create extra quota behind a trusted platform header.

Focused Node coverage for the proxy route and public contract is 94.62% lines, 72.34% branches, and
87.5% functions; `route.ts` itself is 91.3% line covered.

The visitor counter is intentionally process-local for the single-instance launch deployment. It
resets on process restart and is not an exact cross-replica quota; a shared limiter is required before
multi-instance deployment. The authenticated refund service retains its separate 60-run global
circuit breaker per 10-minute window.

## Hosted Railway service

As a launch operator, I can deploy one refund-demo service whose chat/status surface and payment
adapter use separate bearer credentials. Railway can bind the service on `0.0.0.0:$PORT`, while the
Rust API registers only a validated HTTPS provider origin.

RED checkpoint `6bca808d` failed because the merged demo exported no hosted server factory and had no
Railway network contract. GREEN checkpoint `49748821` adds the authenticated `/payments` adapter,
strong production provider-key validation, hosted-origin validation, Railway host/port configuration,
and a dedicated allowlisted Docker build context.

`pnpm test:refund-demo` passes 28 tests, `pnpm --filter @trustloopguard/demo typecheck` passes, the
Docker image builds, and a running container returns `401` for an invalid payment bearer credential.
The container does not expose secret values in its image or startup output.

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
