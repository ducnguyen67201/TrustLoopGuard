# SDK-Driven Development at TrustLoopGuard

TrustLoopGuard is open-source. Adoption happens when a stranger drops one of
our SDKs into their agent runtime and ships. That makes the SDK surface — not
the engine internals — the contract with users.

This doc captures the rules we hold ourselves to so that contract stays honest.

## The three rules

The public runtime calls are `POST /v1/events` with `GuardEvent` for generic
guard decisions and `/v1/financial/*` with typed financial actions for
money-bearing authorization. Event-engine vocabulary such as labels,
provenance, tool metadata, and optional `Decision` evidence follows the same
SDK-driven rule as financial action vocabulary such as money, mandates,
approvals, receipts, and outcomes: define it in `tl-core`, regenerate OpenAPI
and SDK types, then expose behavior only when the SDK call site is clear.

### 1. Engine-only PRs aren't done

If a feature changes user-visible behavior, it lands across the full surface in
the same PR:

- `tl-core` types updated (these are the source of truth for the wire format)
- `cargo run -p tl-codegen` regenerated `docs/openapi.yaml`,
  `policies/*.schema.json`, and `sdks/typescript/src/generated/*`
- `crates/tl-sdk-rust` exposes the new method or field
- `sdks/python/src/trustloopguard/client.py` exposes the new method or field
- `sdks/typescript/src/client.ts` exposes the new method or field
- `docs/INTEGRATION.md` shows the new surface in at least one SDK quickstart

A PR that adds an engine capability without exposing it through every SDK is
half-shipped. Half-shipped features don't merge.

### 2. No internal imports in `demo/`

Demos use *only* what a stranger could install from crates.io / PyPI / npm.
That means they import from `tl-sdk-rust`, `trustloopguard` (Python), and
`@trustloopguard/sdk` (TS) — not from `tl-core`, `tl-engine`, `tl-policy`,
`tl-server`, or any other internal crate.

If a demo needs an internal type, the SDK is missing something. Add it to the
SDK first; then update the demo.

This rule is enforced by the `boundary lint` gate.

### 3. Cross-cutting concerns live in the SDK, once

Retries, auth, error mapping, tracing, timeouts, rate-limit handling — solved
in `tl-sdk-rust` / `trustloopguard` / `@trustloopguard/sdk`, never re-solved
per example or per caller.

If you catch yourself writing the same try/except, the same backoff loop, the
same auth header in two demos: it belongs in the SDK.

## Why this is harder than it sounds

Each rule is mechanical. The hard part is *refusing* to merge work that
violates them — especially under deadline pressure when "we'll add the Python
binding next sprint" feels reasonable. It isn't. Next sprint never comes, and
the SDK falls behind the engine until adoption stalls.

The discipline is the refusal.

## How features are built (the loop)

For every user-visible change:

1. **Write the call site first.** In a scratch file, write the line of code
   you wish a stranger could write to use this feature. If it feels awkward,
   the API is wrong — fix it on paper before writing real code.
2. **Update the Rust types in `tl-core`.** These flow downstream into OpenAPI,
   JSON Schemas, TS types, and (via `datamodel-code-generator`) Pydantic
   types.
3. **Run `cargo run -p tl-codegen`.** The scratch snippet from step 1 should
   compile-check against the regenerated SDK types. If it doesn't, the wire
   contract is wrong; back to step 1.
4. **Write the integration snippet before the implementation.** In
   `docs/INTEGRATION.md` (and a demo when it fits), write the SDK call as a
   stranger would. It's the executable form of "what does success look like."
5. **Implement engine-side.** Make that snippet's call resolve. Engine
   internals can be ugly; strangers don't see them.
6. **Wire the SDK surface.** Thin pass-through in each language. Cross-cutting
   helpers should already exist; you're only exposing the new method.
7. **Mirror to Python and TypeScript.** Mostly mechanical because of codegen.
   Hand-write only the ergonomic wrapper.
8. **Exercise the new surface from all three SDKs.** Same input → same
   decision (the parity tests). If they diverge, the SDK is leaking
   implementation details.
9. **Run `make verify-contract` locally** after changing Rust wire types,
   server route annotations, or generated SDK models.
10. **Tick the PR template checklist.** CI runs `codegen-check`, `sdk build`,
    and the internal-import lint as required gates.

## Reviewer checklist

When reviewing, the question to ask is **not** "is this engine code clean?".
It's:

> Could a stranger use this feature from the SDK docs alone?

If you have to read `tl-engine` to understand how to call the new method, the
docs are wrong. If a demo imports an internal crate to make the new feature
work, the SDK is wrong.

## What this kills

- Half-shipped features (engine without bindings)
- Doc rot (the SDK surface is regenerated from `tl-core` and drift-checked in CI)
- API archaeology (new users read the SDK docs, not the engine)
- Bikeshedding internal abstractions (engine internals stop being a review
  battleground; the SDK is the contract)

## What this costs

- Each PR is ~30% bigger (engine + 3 SDKs + doc updates).
- The first feature after adopting the discipline is slow because helpers
  (errors, retry, auth) didn't exist before. Subsequent features are fast.

## Required CI gates

These workflows run on every PR. They are **required status checks** on
`main` — see [`MERGE_GATES.md`](MERGE_GATES.md) for the
branch-protection settings.

| Gate                     | Workflow                                       | What it checks |
|--------------------------|------------------------------------------------|----------------|
| `codegen drift`          | `.github/workflows/codegen-check.yml`          | `tl-core` source-of-truth matches the generated OpenAPI / JSON Schemas / TS types / Pydantic models on disk |
| `sdk build`              | `.github/workflows/sdk-build.yml`              | All three SDKs compile and pass tests |
| `boundary lint`          | `.github/workflows/lint-sdk-boundary.yml`      | `demo/` only imports the published SDK surface; public API DTOs live in `tl-core` |

Local equivalents:

| Gate                     | Local command                          |
|--------------------------|----------------------------------------|
| `codegen drift`          | `make verify-contract`                 |
| `sdk build`              | `make sdk-all`                         |
| `boundary lint`          | `make ci-lint`                         |
| All three                | `make ci`                              |

## Out of scope

This doc is about discipline at the SDK boundary. It does **not** govern:

- Engine internal architecture (`tl-engine`, `tl-fuzzy`, `tl-storage`, …)
- Server transport details (`tl-server`)
- Internal CLI ergonomics (`tl-cli` is for operators, not third-party
  integrators; it has its own UX bar)

## Direct event submission

SDKs submit a full `GuardEvent` — operation, parameters, sources, and
parameter-to-source provenance — for runtime guard decisions:

```ts
const decision = await client.submitEvent({
  kind: "tool.call.proposed",
  principal: { workspace_id: "ws", environment_id: "production", agent_id: "support-agent" },
  action: { operation: "send_email", parameters: { recipient, body } },
  sources: [
    { id: "user:msg-1", origin: "user", labels: {} },
    { id: "web:page-7", origin: "web", labels: {} },
  ],
  provenance: { recipient: ["web:page-7"], body: ["user:msg-1", "web:page-7"] },
  context: null,
});
```

Rust: `client.submit_event(&event)`. Python: `client.submit_event(event)`
(also on `AsyncClient`). The server resolves workspace/environment identity,
runs the GuardEvent pipeline, loads enabled workspace policies, evaluates them
against the event, and returns one composed `Decision`.

## Typed financial authorization

SDKs submit money-bearing work as `FinancialAction` requests, not as generic
guard events. The financial surface owns authorization state, approvals,
ledger-derived spend, provider execution, receipts, and outcomes:

```ts
const mandate = await client.createMandate({
  principal_id: "refund-bot",
  scope: { action_kinds: ["refund"], max_amount_minor: 10000, currency: "USD" },
  metadata: { source: "customer_backend" },
});

const action = await client.guardPayment({
  idempotency_key: "refund-order-123",
  execute: false,
  action: {
    kind: "refund",
    principal_id: "refund-bot",
    amount: { amount_minor: 7500, currency: "USD" },
    counterparty: { id: "cust_456", kind: "customer", metadata: {} },
    rail: "card",
    mandate: { id: mandate.id, version: mandate.version },
    metadata: { order_id: "order_123", reason: "damaged_item" },
  },
  evidence: [{ source: "customer_backend", source_id: "elig_789", kind: "refund_eligibility", metadata: {} }],
});

const approved = action.status === "held" ? await client.approveAction(action.id) : action;
const executed = await client.executeAction(approved.id);
const receipt = await client.getReceipt(executed.id);
```

Generic guard events remain the right contract for document, tool-call, output,
memory, or database safety checks. Financial actions are the right contract
when the product must prove authorization before execution and produce
ledger-backed proof afterward.

## Run grouping helper

SDKs expose runs as the grouping layer above individual event decisions. Prefer the scoped helper when an agent runtime owns the whole execution:

```ts
await client.withRun(
  { agentId: "support-agent", kind: "chat_session", externalId: conversation.id },
  async (run) => {
    await run.withEvent({ kind: "user_turn", metadata: {} }, async () => {
      await guard({ client, agentId: "support-agent", input: userMessage, draft });
    });

    await client.guardToolCall({
      agentId: "support-agent",
      operation: "issue_refund",
      parameters: { orderId },
      sideEffect: "api_mutation",
      sources: [{ id: "input", origin: "user", labels: {} }],
      provenance: { orderId: ["input"] },
    });
  },
);
```

The same model exists in Python as `with client.run(...)` / `async with client.run(...)` and in Rust as `client.with_run(...)`. Scoped helpers merge active run ids into `GuardEvent.principal` at submission time; explicit `run_id`, `run_event_id`, and `session_id` fields still win.

Manual run wiring remains available:

```ts
const run = await client.startRun({
  agent_id: "support-agent",
  kind: "chat_session",
});
const runEvent = await client.createRunEvent(run.id, {
  kind: "assistant_turn",
  label: "Turn 1",
  input_summary: "Customer asks about a refund",
  output_summary: "Agent drafts refund answer",
});
await client.submitEvent({
  kind: "output.proposed",
  principal: {
    workspace_id: "",
    environment_id: "",
    agent_id: "support-agent",
    run_id: run.id,
    run_event_id: runEvent.id,
  },
  action: { operation: "output", parameters: { text: proposedOutput }, side_effect: "none" },
  sources: [{ id: "input", origin: "user", labels: {} }],
  provenance: { text: ["input"] },
  context: { channel: "chat", domain: "customer_support" },
});

await client.finishRun(run.id);
```

The same manual shape exists in Python and Rust as `start_run`, `submit_event`, and
`finish_run`. `createRunEvent` / `create_run_event` remain available for
timeline moments that do not need an immediate guardrail decision. Runtime
events link to runs through `GuardEvent.principal.run_id` and
`run_event_id`.

## MCP adapter

The local MCP server in `apps/mcp-server` follows the same SDK-first rule. It
does not define new wire contracts or own guardrail storage; each MCP tool maps
to an existing TypeScript SDK method, which then calls the Rust `/v1/*` API.

Use it for assistant-facing setup and inspection workflows:

- submit a `GuardEvent` for a decision,
- create or inspect runs and run events,
- list traces,
- register agents,
- validate or upsert policies,
- register tool metadata.

If an MCP workflow needs a new product capability, add the Rust endpoint and
shared `tl-core` type first, regenerate the SDK surface, then expose a thin MCP
tool.

## Publishing

The TypeScript SDK release process is tag-driven. See
[`docs/concept/sdk-publishing.md`](concept/sdk-publishing.md) for the canonical
process, including version selection, environment approval, and npm failure
modes.

For the engine roadmap, see the 21-PR plan in the repo issues.
