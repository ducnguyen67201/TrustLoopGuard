# Procurement Agent

## What It Teaches

Let an agent search public catalog data, but authorize the exact canonical
purchase-order action before the simulated procurement system receives it.
The tool accepts only a quote ID; server-owned fixture data supplies supplier,
category, value, and review facts.

```text
search_catalog -> quote ID -> resolve canonical quote
                             -> withAuthorizedAction()
                             -> Rust tool policies
                             -> execute only on permit
```

## Fastest Check

```bash
pnpm test:procurement-demo
```

This covers the agent, public contract, Marketing route, and deployment shape
without a live model.

## Run It

```bash
pnpm --filter @trustloopguard/demo procurement-agent:setup
OPENAI_API_KEY=<test-key> pnpm marketing:dev
```

Setup requires a configured Rust server, an internal API key, workspace
context, and an approved admin user. Open
`http://localhost:3002/demo/procurement`.

## Expected Proof

- Approved chairs are permitted.
- High-value laptops require approval.
- An unapproved supplier is denied.
- Restricted gift cards are denied.
- The simulated purchase callback runs only for `permit`.

## Read The Code

- [`fixtures.ts`](../../demo/procurement-agent/fixtures.ts) owns canonical facts
  and policy YAML.
- [`agent.ts`](../../demo/procurement-agent/agent.ts) owns tools and guarded
  execution.
- [`hosted.ts`](../../demo/procurement-agent/hosted.ts) owns public isolation.
- [`demo/README.md`](../../demo/README.md#procurement-agent) owns full setup.
