# Contextual Outbound Agent

## What It Teaches

Reuse one guarded agent runtime across company-neutral workflow pages at
`/demo/{workflow-category}`. Marketing supplies bounded public presentation
context, while Rust owns the agent, policy pack, runtime key, decisions, and
traces.

```text
browser -> Marketing route -> input /v1/events check
                           -> one OpenAI draft
                           -> output /v1/events check
                           -> guarded reply
```

## Fastest Check

```bash
pnpm test:contextual-demo
```

This suite covers the agent, workspace provisioning, public route contract,
profile validation, and single-service deployment boundary without a live
provider.

## Run It

Provision the shared runtime with an internal management credential:

```bash
pnpm --filter @trustloopguard/demo contextual-agent:setup
```

Store the printed `TL_CONTEXTUAL_DEMO_API_KEY` only in the Marketing server
environment, set `OPENAI_API_KEY`, then run:

```bash
pnpm marketing:dev
```

Open `http://localhost:3002/demo/{workflow-category}` using a configured
profile category.

## Expected Proof

- Read-only requests can pass.
- Shared changes are held for human review.
- Authorization bypass and secret disclosure are denied.
- False execution claims are transformed before reaching the browser.

## Read The Code

- [`agent.ts`](../../demo/contextual-agent/agent.ts) owns both guard boundaries.
- [`hosted.ts`](../../demo/contextual-agent/hosted.ts) owns request isolation.
- [`setup.ts`](../../demo/contextual-agent/setup.ts) provisions Rust state.
- [`demo/README.md`](../../demo/README.md#contextual-outbound-demo-agent)
  contains the full environment contract.
