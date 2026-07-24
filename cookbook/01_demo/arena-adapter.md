# Arena Adapter

## What It Teaches

Expose an existing agent in the two shapes understood by the Attacks tooling:

- OpenAI-compatible `GET /v1/models` and `POST /v1/chat/completions`;
- simple `GET /arena/profile` and `POST /arena/chat`.

The adapter is transport glue. It does not own attacks, policies, decisions,
or traces.

## Fastest Check

```bash
pnpm --filter @trustloopguard/demo arena:check
```

The check starts an ephemeral loopback adapter, calls both protocols, verifies
session propagation and trace metadata, then closes the server.

## Run It In A Demo

The dispute demo creates raw and guarded adapters from the same helper:

```bash
pnpm --filter @trustloopguard/demo dispute:serve:doppler
```

Use `http://127.0.0.1:9201` for the raw target and
`http://127.0.0.1:9202` for the guarded target in the dashboard Attacks page.

## Expected Proof

- An OpenAI-compatible runner can discover the target model and chat with it.
- A simple runner can call `/arena/chat`.
- Guard metadata is returned without becoming a second trace store.

## Read The Code

- [`adapter.ts`](../../demo/arena/adapter.ts) owns the adapter contract.
- [`adapter.check.ts`](../../demo/arena/adapter.check.ts) is the offline check.
- [Agent Breakaway Arena](../../docs/concept/agent-breakaway-arena.md) is the
  canonical concept document.
