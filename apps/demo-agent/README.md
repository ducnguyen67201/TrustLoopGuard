# `@trustloopguard/demo-agent`

A scripted demo of the TrustLoopGuard request lifecycle:

1. Register an agent profile (`POST /v1/agents`)
2. For each scripted scenario, call `guard()` with all four callback branches wired in
3. Print the verdict + the string the agent should actually send

The scenarios are engineered to fire each verdict **deterministically** using v0 universal detectors — no LLM round-trips needed:

| Scenario | Verdict | Why |
|---|---|---|
| benign question | `allow` | nothing matches, no PII, no injection |
| PII in draft (US phone) | `block` | `universal:pii.phone` fires on `proposed_output` |
| prompt injection in input | `escalate` | `universal:prompt_injection.*` fires on `input` |

## Prerequisites

A local `tl-server` listening on `127.0.0.1:8080`:

```sh
cargo run -p tl-server
```

Optional env vars:

| Var | Default | What it does |
|---|---|---|
| `TL_SERVER_URL` | `http://127.0.0.1:8080` | server base URL |
| `TL_API_KEY` | _unset_ | bearer token (only needed if server has one configured) |

## Run

From repo root:

```sh
pnpm --filter @trustloopguard/demo-agent demo
```

Exit code: **0** if every scenario got its expected verdict, **1** if any
didn't, **2** on a setup error (e.g. server unreachable).

## Authoring your own agent profile

The profile YAML next to this README (`agents/acme-support-v3.yaml`) is a working example. For a field-by-field reference — what each section does, which Tier 3 judge consumes it, validation rules, common authoring mistakes — see [`docs/AGENT_PROFILE.md`](../../docs/AGENT_PROFILE.md).

## Wire it into your agent

The dispatch is the same shape your real agent loop should use:

```ts
const reply = await guard({
  client,
  agentId: 'my-bot',
  input,
  draft,
  onBlock:    () => myCannedSafeReply,
  onEscalate: () => myHumanQueue.push(...) ?? myHoldMessage,
});
await sendToCustomer(reply);
```

That's it — no `switch (verdict)`, no per-branch glue. Override `onAllow` / `onRevise` / `onError` only if you want different default behaviour. See `sdks/typescript/src/guard.ts` for the full options surface.
