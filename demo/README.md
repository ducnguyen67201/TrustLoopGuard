# TrustLoopGuard demos

These demos exercise the public SDKs:

1. The agent drafts output.
2. The demo calls a runtime guard or typed financial authorization helper.
3. TrustLoopGuard returns a decision, action status, trace, receipt, or proof.
4. The demo executes only after authorization allows it.

## Agent visibility

This deterministic TypeScript agent demonstrates the one-line agent decorator.
It exposes a Mastra-shaped `book_appointment` tool, calls that tool from
`agent.reply()`, and uses `Client.withRun()` so the dashboard receives one
completed run with both the automatic `tool.call.proposed` trace and the final
output trace.

```sh
TL_SERVER_URL=http://127.0.0.1:8080 \
TL_WORKSPACE_ID=<workspace-id> \
pnpm --filter @trustloopguard/demo agent-visibility
```

The script registers the agent profile and discovered tool metadata, executes
one appointment, waits for both traces to persist, and prints the run id to open
on the dashboard.

Start the Rust server first:

```sh
cargo run -p tl-server
```

Optional environment:

| Var | Default | Purpose |
| --- | --- | --- |
| `TL_SERVER_URL` | `http://127.0.0.1:8080` | TrustLoopGuard server URL |
| `TL_API_KEY` | unset | Bearer token when the server requires auth |
| `TL_AGENT_ID` | `demo-acme-support` | Demo agent profile id |
| `TL_WORKSPACE_ID` | unset | Optional local workspace header override, e.g. `ws_test` |
| `TL_ADMIN_USER_ID` | unset | Optional owner/admin identity header for policy setup operations |
| `TL_HEALTHCARE_DEMO_API_KEY` | unset | Workspace-scoped runtime key created by `healthcare-agent:setup`; required by the hosted healthcare demo |
| `OPENAI_API_KEY` | unset | Enables real OpenAI-backed replies |
| `OPENAI_MODEL` | `gpt-4.1-mini` | OpenAI model for LLM-backed replies |
| `TL_USER_ID` | unset | Workspace owner/admin UUID — lets `dispute:setup` arm `enforce` checker modes |
| `STRIPE_SECRET_KEY` | unset | **Test-mode only** (`sk_test_…`). When set, an allowed payment makes one real Stripe test-mode call; otherwise payments are simulated. A live key is refused. |
| `STRIPE_REFUND_AGENT_DB` | `demo/.data/stripe-refund-agent.sqlite` | SQLite DB used by the refund-agent demo as the customer order backend |
| `STRIPE_PAYMENT_INTENT_ID` | seeded demo id | Optional Stripe test PaymentIntent id for the refund-agent order |
| `STRIPE_REFUND_PROVIDER_PORT` | `9303` | Local provider sidecar port for Stripe refund execution |
| `STRIPE_REFUND_PROVIDER_API_KEY` | local demo token | Bearer token TrustLoopGuard uses when calling the provider sidecar |

## Procurement agent

The Marketing demo at `http://localhost:3002/demo/procurement` is a live
OpenAI Agents SDK workflow protected by TrustLoopGuard. The agent can search a
fixed catalog, but its simulated procurement system receives a purchase order
only after the TypeScript SDK submits the exact canonical action to Rust and
receives `permit`.

Tools exposed to the agent:

| Tool                    | What it does                                                                                                                 |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `search_catalog`        | Reads public fields from four immutable demo quotes                                                                          |
| `submit_purchase_order` | Accepts only a quote id, resolves server-owned facts, and wraps the simulated side effect in `Client.withAuthorizedAction()` |

Three Rust `family: tool` policies protect `submit_purchase_order`:

| Policy                              | Match                            | Effect             |
| ----------------------------------- | -------------------------------- | ------------------ |
| `procurement-approved-suppliers`    | `/supplier_status == unapproved` | `deny`             |
| `procurement-high-value-review`     | `/review_tier == high_value`     | `require_approval` |
| `procurement-restricted-categories` | `/category` is restricted        | `deny`             |

The public policy switches never edit shared policies. The validated selection
maps to one of eight pre-provisioned principals, from `procurement-demo-000` to
`procurement-demo-111`. Each policy is scoped to the four principals where its
bit is active, plus the purchase operation, side-effect class, and tool
identity. Use a dedicated Marketing demo workspace/environment and keep every
policy in it explicitly scoped to demo principals so curated outcomes remain
deterministic.

Start Rust, configure the internal Marketing-to-Rust lane, and provision the
three policies once:

```sh
cargo run -p tl-server

TL_SERVER_URL=http://127.0.0.1:8080 \
TL_API_KEY=<internal-api-key> \
TL_WORKSPACE_ID=procurement-demo-local \
TL_ADMIN_USER_ID=<owner-or-admin-id> \
pnpm --filter @trustloopguard/demo procurement-agent:setup
```

`TL_ADMIN_USER_ID` is needed when the local server requires an owner/admin
identity for policy writes. The Marketing service must use the internal
`TL_API_KEY` lane: a runtime key bound to one principal cannot select among the
eight fixed profiles. Never expose that key to the browser.

Then start the live Marketing demo:

```sh
OPENAI_API_KEY=<test-key> \
OPENAI_MODEL=gpt-4.1-mini \
pnpm marketing:dev
```

Try the four curated paths: approved chairs permit, high-value laptops require
approval, an unapproved supplier is denied, and gift cards are denied. Turning
off the matching policy selects another fixed principal and lets the remaining
Rust policies determine the result.

The purchase-order provider and its state are simulated and request-local.
Approval is displayed but cannot be resumed from the public page. OpenAI Agents
SDK tracing is disabled for public prompts; the Rust `/v1/events` trace remains
the product proof. The central model budget and per-visitor limit are
process-local and assume the existing single Marketing replica.

Code map:

| File                            | Start here for                                                                     |
| ------------------------------- | ---------------------------------------------------------------------------------- |
| `procurement-agent/fixtures.ts` | Catalog facts, public policy metadata, fixed principals, and generated policy YAML |
| `procurement-agent/agent.ts`    | OpenAI tools and the guarded purchase-order callback                               |
| `procurement-agent/hosted.ts`   | Request isolation, model budget, safe logs, and public workflow result             |
| `procurement-agent/setup.ts`    | Admin-only policy provisioning                                                     |

Concept impact: none. This demo uses the existing `/v1/events`, tool-policy,
SDK guarded-execution, policy-registry, and Rust trace contracts without
changing their behavior or ownership.

## Stripe refund agent

This is the live financial-authorization demo. You ask a support agent for a
refund; the agent searches a seeded order, prepares a typed TrustLoopGuard
refund action, and executes only through the vaulted `payment_http` provider
path. The agent process does not need `STRIPE_SECRET_KEY`.

SQLite is the demo customer backend. `search_order` queries `orders` and
`refunds` from `demo/.data/stripe-refund-agent.sqlite`, then returns trusted
eligibility evidence to TrustLoopGuard. TrustLoopGuard still owns financial
authorization state, ledger entries, approvals, and receipts.

Tools exposed to the agent:

| Tool | What it does |
| --- | --- |
| `search_order` | Read-only lookup for order/payment/refundable-balance evidence |
| `prepare_refund` | Calls the SDK `financialOperation("issue_refund")` wrapper, which submits a typed refund `FinancialAction` |
| `execute_refund` | Calls TrustLoopGuard `executeAction` after authorization |

Run the local stack first:

```sh
make local
```

Then set up the demo workspace:

```sh
pnpm --filter @trustloopguard/demo stripe-refund-agent:db
pnpm --filter @trustloopguard/demo stripe-refund-agent:setup
```

For the browser demo, start the refund provider and chat UI together:

```sh
cd demo
pnpm run dev
```

Open `http://127.0.0.1:9310`. The page shows the agent chat, tool trace, and
SQLite order/refund state. `pnpm run dev` starts both the simulated payment
provider on `127.0.0.1:9303` and the chat UI on `127.0.0.1:9310`; use
`doppler run -- pnpm run dev` if you want Doppler env vars available to both.

For a terminal-only refund:

```sh
pnpm --filter @trustloopguard/demo stripe-refund-agent:provider
pnpm --filter @trustloopguard/demo stripe-refund-agent \
  'Refund order ord_demo_1001 for $75 because damaged item.'
```

With no Stripe key, the provider returns a simulated refund id. With
`STRIPE_SECRET_KEY=sk_test_...`, the provider creates a real Stripe sandbox
refund. Live keys are refused.

Offline smoke:

```sh
pnpm --filter @trustloopguard/demo stripe-refund-agent:check
```

Code map:

| File | Start here for |
| --- | --- |
| `stripe-refund-agent/agent.ts` | Choosing OpenAI mode or deterministic scripted mode |
| `stripe-refund-agent/scripted-agent.ts` | The easiest-to-read refund flow |
| `stripe-refund-agent/tool-runner.ts` | The three agent tools and their outputs |
| `stripe-refund-agent/core.ts` | TrustLoopGuard financial action preparation/execution |
| `stripe-refund-agent/order-db.ts` | SQLite customer-backend order/refund state |
| `stripe-refund-agent/ui.ts` | Local chat UI for the demo agent |

## Agentic refund authorization

This is the financial-authorization wedge demo for support or fintech ops. It
uses the typed `guardPayment` flow instead of converting generic guard events
into finance. The demo is offline-safe by default: a mock SDK-shaped financial
client creates a grant, submits refund actions, applies cap/approval/grant
logic, executes only authorized actions, exports receipts, records outcomes, and
proves duplicate idempotency does not execute twice.

| Scenario | Initial status | Final status | Provider calls |
| --- | --- | --- | --- |
| refund $40 under approval threshold | `executed` | `executed` | 1 |
| refund $75 held, approved, then executed | `held` | `executed` | 1 |
| refund $80 held, denied | `held` | `denied` | 0 |
| duplicate retry | `executed` | `executed` | 1 total |
| missing grant | `denied` | `denied` | 0 |

```sh
pnpm --filter @trustloopguard/demo financial-refund
pnpm --filter @trustloopguard/demo financial-refund:check
```

## Money agent — guarded scenarios (flagship)

An AI agent that moves money, guarded. One run sends a fixed set of money-move
attempts through the guard; **each scenario trips exactly one control**, and a
payment fires **only** when the effect is `permit`. Amounts are integer cents.

| Scenario | Effect | Control |
| --- | --- | --- |
| legit refund $50 | `permit` → payment fires | — |
| over-cap refund $750 | `deny` | `value_limit` (amount cap) |
| refund to an injected account | `deny` | `parameter_auth` (destination source) |
| ambiguous (non-integer) amount | `defer` | `value_limit` (unverifiable) |
| wire transfer | `require_approval` | `approval` (human sign-off) |

```sh
make server                                                   # 1. run the guard
TL_USER_ID=<owner-uuid> pnpm --filter @trustloopguard/demo dispute:setup   # 2. register tools + arm enforce modes
pnpm --filter @trustloopguard/demo dispute:scenarios          # 3. simulated payments
STRIPE_SECRET_KEY=sk_test_… pnpm --filter @trustloopguard/demo dispute:scenarios   # or real test-mode payments
```

The runner prints a effect table and fails loudly if every scenario was
allowed (which means the workspace's `param`/`approval` checkers are still
`off` — set `TL_USER_ID` so setup can arm them, or enable them in Settings).

Offline smoke (no server, no keys):

```sh
pnpm --filter @trustloopguard/demo dispute:scenarios:check
```

## Bring your own agent

Gate your own money tool on a effect — the whole integration is: register your
tool's controls once, then ask the guard before you execute and honor the
effect. See [`dispute/byo.example.ts`](dispute/byo.example.ts):

```sh
make server
TL_USER_ID=<owner-uuid> pnpm --filter @trustloopguard/demo dispute:setup
pnpm --filter @trustloopguard/demo dispute:byo
```

```ts
const decision = await client.submitEvent(yourToolEvent);
if (decision.effect === 'permit') await yourRealPaymentApi(...); // otherwise, money never moves
```

Prefer not to wrap each call? Point your OpenAI/Anthropic client's `baseURL` at
the gateway proxy instead — same effects, no per-call code.

## NorthPay dispute

This is the smallest useful demo: an OpenAI SDK chat agent with one tool,
`issue_refund(amount, account, reason)`.

The smoke test is local-only: it forces `useOpenAI: false` and checks the
fallback parser plus local refund ledger behavior.

```sh
pnpm --filter @trustloopguard/demo dispute:check
```

For the OpenAI-backed Attacks tab flow, the demo exposes the same agent in two
modes:

- Raw target root: `http://127.0.0.1:9201`
- Guarded target root: `http://127.0.0.1:9202`

Use the root URL in the Attacks page and in saved agent config. The arena
adapter exposes both protocols:

- HackAgent/OpenAI-compatible chat: `/v1/models` and `/v1/chat/completions`
- Simple runner/manual chat: `/arena/chat`

HackAgent can initiate chat through `/v1/chat/completions`; manual curl can use
`/arena/chat`.

Set up the refund tool metadata once with the Rust server running:

```sh
pnpm --filter @trustloopguard/demo dispute:setup:doppler
```

Start the dispute adapters:

```sh
pnpm --filter @trustloopguard/demo dispute:serve:doppler
```

Open `http://localhost:3000/attacks`, then run against each root target:

1. `http://127.0.0.1:9201` should show the raw dispute agent issuing the
   attacker-directed refund.
2. `http://127.0.0.1:9202` should show the same proposed refund blocked by the
   guard when the workspace has the dispute tool metadata enabled.

## Healthcare scheduling agent

The Marketing app exposes a live, synthetic healthcare scheduling demo at
`http://localhost:3002/demo/healthcare`. It demonstrates a customer-style
OpenAI integration protected at two boundaries:

1. The current visitor message is submitted to the Rust `/v1/events` pipeline
   with the `healthcare_input` domain.
2. Only a permitted input reaches one stateless OpenAI Responses API call.
3. The model draft is submitted to the same Rust pipeline with the
   `healthcare_output` domain before any reply reaches the browser.
4. The policy monitor reads enabled policy summaries from the Rust registry;
   the templates in this directory are setup input, not a runtime policy store.

Build the TypeScript SDK and start a configured Rust server first. Run setup
with the internal management credential and an approved owner/admin user. Setup
finds or creates one Rust-owned workspace named `Healthcare Demo`; no workspace
id is configured or hard-coded:

```sh
pnpm --filter @trustloopguard/sdk build

export TL_SERVER_URL=http://127.0.0.1:8080
export TL_API_KEY=<internal-management-key>
export TL_ADMIN_USER_ID=<approved-owner-or-admin-user-id>

pnpm --filter @trustloopguard/demo healthcare-agent:setup
```

On the first run, setup prints `TL_HEALTHCARE_DEMO_API_KEY` exactly once. Put
that workspace runtime key in the Marketing server environment, remove the
setup-only management credentials from that deployment, and start the app:

```sh
export TL_SERVER_URL=http://127.0.0.1:8080
export TL_HEALTHCARE_DEMO_API_KEY=<tl_live_key_printed_by_setup>
export OPENAI_API_KEY=<openai-key>
export OPENAI_MODEL=gpt-4.1-mini

pnpm marketing:dev
```

Run the setup command again safely whenever the templates change. It upserts
the same workspace, `healthcare-demo-agent` profile, and six policy IDs,
validates their YAML through Rust, and explicitly re-enables them in the
workspace's default environment. It reuses existing runtime-key metadata and
never rotates a secret implicitly. Manage the agent, policies, environments,
and API keys by switching to `Healthcare Demo` in the dashboard. The templates
in this directory are bootstrap desired state only; the running demo reads the
Rust registry exclusively. If that registry is unavailable, policy inventory
is shown as unavailable rather than falling back to hard-coded policy cards.
Store production values in the deployment secret manager and never expose them
through `NEXT_PUBLIC_*` variables. Pin and evaluate the production model through
the existing `OPENAI_MODEL` setting before a customer deployment.

Expected synthetic presets:

| Preset | Expected result |
| --- | --- |
| Schedule a visit | Input permit, one OpenAI draft, output check, guarded reply |
| Emergency symptoms | Input deny, emergency guidance, OpenAI skipped |
| Medication advice | Input deny, clinician handoff, OpenAI skipped |
| Another patient | Input deny, privacy-safe refusal, OpenAI skipped |

Use synthetic text only. This demo does not integrate an EHR, verify identity,
create or modify an appointment, provide diagnosis or treatment, or establish
HIPAA compliance. Clinical and emergency requests are directed to people. The
demo consumes existing SDK, policy, event, and trace contracts, so it does not
change system ownership or introduce a new concept-layer contract.

## LiveKit

The LiveKit demo is Python because it follows the LiveKit Agents runtime. SDK
mode guards the draft right before the agent speaks:

```sh
pip install -e sdks/python
pip install "livekit-agents[openai,silero]" python-dotenv

TL_SERVER_URL=http://127.0.0.1:8080 \
TL_AGENT_ID=demo-healthcare-livekit \
python demo/livekit/guarded_healthcare_agent.py dev
```

For voice, the demo configures a 250 ms guardrail budget and one SDK attempt:
the runtime either returns guarded output within the realtime budget or follows
the SDK's configured failure behavior.

Gateway mode points LiveKit's OpenAI-compatible LLM at TrustLoopGuard instead of
calling the provider directly:

```sh
python demo/livekit/proxy_healthcare_agent.py dev
```

Set `TLG_API_KEY` and `TL_GATEWAY_ROUTE_ID` in `demo/livekit/.env` first. See
`livekit/README.md` for the full setup.
