# Output Boundary Quickstart

This example answers one question: **where does TrustLoopGuard sit in an AI
agent?**

```text
user input -> agent creates draft -> TrustLoopGuard /v1/events -> Rust decision
                                      |                            |
                                      +---- trace is persisted <---+
                                                                   |
                                               application delivers only
                                               the guarded reply
```

The example agent is deterministic, so no model key is needed. It returns
normal support hours for a safe prompt and a synthetic SSN for a risky prompt.
The SSN exists only to make the deny path obvious; it is not real customer data.

## Read The Integration

The complete application boundary is in [`agent.ts`](agent.ts):

```ts
const guardrail = guard({
  agentId: 'cookbook-support-agent',
  failClosed: true,
});

const reply = guardrail.wrap(draftSupportReply);
```

`reply(input)` creates the draft, sends an authenticated `POST /v1/events`
request directly to the Rust API, and returns the permitted draft or a safe
replacement. The application never delivers the raw draft first.

## Prerequisites

- Node.js 22 or newer and pnpm
- the repository dependencies installed with `pnpm install`
- a local `tl-server` or a hosted TrustLoopGuard runtime key

## 1. Start Rust

For the shortest local path, use the in-memory Rust storage mode and local-only
secrets. From the repository root:

```bash
export TL_API_KEY=cookbook-local-key
export TL_GATEWAY_CREDENTIAL_KEY=cookbook-local-gateway-key
cargo run -p tl-server --no-default-features
```

Keep the server running in that terminal. The cookbook uses
`http://127.0.0.1:8080` by default. This mode keeps traces only for the server
lifetime; use the normal Postgres-backed server when you want durable history.

## 2. Register The Example

In another terminal, set the same internal key plus an explicit local workspace,
then register the agent profile and publish its policy:

```bash
export TL_SERVER_URL=http://127.0.0.1:8080
export TL_API_KEY=cookbook-local-key
export TL_WORKSPACE_ID=cookbook-local

curl -X POST http://127.0.0.1:8080/v1/agents \
  -H "authorization: Bearer $TL_API_KEY" \
  -H "x-tlg-workspace-id: $TL_WORKSPACE_ID" \
  -H 'content-type: application/yaml' \
  --data-binary @cookbook/00_quickstart/agent.yaml

curl -X POST http://127.0.0.1:8080/v1/policies \
  -H "authorization: Bearer $TL_API_KEY" \
  -H "x-tlg-workspace-id: $TL_WORKSPACE_ID" \
  -H 'content-type: application/yaml' \
  --data-binary @cookbook/00_quickstart/policy.yaml
```

These operations are idempotent. On a hosted server, create the agent, policy,
and agent-bound runtime key in the dashboard instead. Set `TLG_URL` and
`TLG_API_KEY=tl_live_...` for the runner and leave `TL_WORKSPACE_ID` unset;
Rust derives the workspace from the runtime key.

## 3. Run Both Decisions

Risky prompt, expected `deny`:

```bash
pnpm --filter @trustloopguard/cookbook quickstart
```

Safe prompt, expected `permit`:

```bash
pnpm --filter @trustloopguard/cookbook quickstart -- "When is support available?"
```

The runner labels four stages:

```text
1. User input
2. Agent draft (never deliver this directly)
3. TrustLoopGuard decision and trace ID
4. Delivered reply
```

Open the dashboard trace after either run to inspect the same persisted
decision. Try changing the prompt, the deterministic draft, or
[`policy.yaml`](policy.yaml) to see which boundary behavior changes.

## Verify Without A Server

The contract tests mock only HTTP transport while exercising the real SDK:

```bash
pnpm --filter @trustloopguard/cookbook test
pnpm --filter @trustloopguard/cookbook test:coverage
pnpm --filter @trustloopguard/cookbook typecheck
cargo run -p tl-cli -- policy validate cookbook/00_quickstart/policy.yaml
cargo run -p tl-cli -- agent-lint cookbook/00_quickstart/agent.yaml
```

They prove that permit returns the draft, deny replaces the draft, transport
failure fails closed, and the SDK submits the expected `output.proposed` event.
