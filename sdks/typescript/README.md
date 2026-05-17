# @trustloopguard/sdk

TypeScript SDK for [TrustLoopGuard](https://trustloopguard.dev) — real-time guardrails for AI agents.

## Installation

```bash
npm install @trustloopguard/sdk
```

## Quick start

```ts
import { guard } from '@trustloopguard/sdk';

const guardrail = guard({ agentId: 'my-agent', apiKey: process.env.TLG_API_KEY });

const reply = await guardrail({ input: userMessage, draft: agentDraft });
await sendToUser(reply);
```

## Guard modes

| Mode | Behavior |
|------|----------|
| `strict` | Blocked or rewritten output is always rejected |
| `rewrite` | Uses `safeOutput` when available, blocks otherwise |
| `rewrite_or_regenerate` | Uses `safeOutput`, or triggers a regeneration loop |

```ts
import { guard, GuardMode } from '@trustloopguard/sdk';

const guardrail = guard({
  agentId: 'my-agent',
  apiKey: process.env.TLG_API_KEY,
  mode: GuardMode.Rewrite,
});
```

## Custom handlers

```ts
import { guard } from '@trustloopguard/sdk';

const guardrail = guard({
  agentId: 'my-agent',
  apiKey: process.env.TLG_API_KEY,
  onBlock:    () => "I can't help with that.",
  onEscalate: () => { humanQueue.push(draft); return 'A human will follow up.'; },
});
```

## Low-level client

```ts
import { Client } from '@trustloopguard/sdk';

const client = new Client({ apiKey: process.env.TLG_API_KEY });

const decision = await client.check({
  agent_id: 'my-agent',
  input: userMessage,
  draft: agentDraft,
});
```

## Requirements

- Node.js 18+
- TypeScript 5+ (optional but recommended)

## License

Apache-2.0
