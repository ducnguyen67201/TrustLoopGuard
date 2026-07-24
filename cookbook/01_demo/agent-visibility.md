# Agent Visibility

## What It Teaches

Decorate an existing agent once with `guardAgent(...)`, discover its
Mastra-shaped `book_appointment` tool, and group the tool proposal plus final
output under one Rust-owned Run.

```text
agent.reply()
  -> tool.call.proposed
  -> permitted tool execution
  -> output.proposed
  -> one completed dashboard Run
```

## Fastest Check

This demo needs Rust for runtime behavior. Its compile-time contract is covered
by the demo package typecheck:

```bash
pnpm --filter @trustloopguard/demo typecheck
```

## Run It

Start `tl-server`, then run:

```bash
TL_SERVER_URL=http://127.0.0.1:8080 \
TL_WORKSPACE_ID=<workspace-id> \
pnpm --filter @trustloopguard/demo agent-visibility
```

The script registers its agent profile and discovered tool metadata, books one
synthetic appointment, waits for both traces, and prints the Run ID.

## Expected Proof

- The booking callback executes once.
- The Run contains `tool.call.proposed` and `output.proposed` traces.
- The printed Run ID opens the same grouped activity in the dashboard.

## Read The Code

- [`guarded-agent.ts`](../../demo/agent-visibility/guarded-agent.ts) contains
  the complete agent, tool, decorator, Run, and trace check.
- [`demo/README.md`](../../demo/README.md#agent-visibility) owns detailed
  environment setup.
