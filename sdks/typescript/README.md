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

const client = new Client({
  baseUrl: process.env.TLG_URL ?? 'http://127.0.0.1:8080',
  apiKey: process.env.TLG_API_KEY,
});

const decision = await client.check({
  agent_id: 'my-agent',
  input: userMessage,
  draft: agentDraft,
});
```

## Gateway mode

The SDK keeps full control in your code. Gateway mode is the proxy path:
configure a provider connection, enforcement profile, and route in the
dashboard, then point provider traffic at TrustLoopGuard.

OpenAI-compatible example:

```ts
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: process.env.TLG_API_KEY,
  baseURL: 'https://api.trustloopguard.com/v1/gateway/<route_id>/openai',
});

const response = await openai.chat.completions.create({
  model: 'gpt-4o-mini',
  messages: [{ role: 'user', content: userMessage }],
});
```

Anthropic example:

```ts
import Anthropic from '@anthropic-ai/sdk';

const anthropic = new Anthropic({
  authToken: process.env.TLG_API_KEY,
  baseURL: 'https://api.trustloopguard.com/v1/gateway/<route_id>/anthropic',
});
```

SDK mode returns a decision for your code to handle. Gateway mode applies
the dashboard enforcement profile before returning a provider-compatible
response.

Gateway configuration types such as `GatewayRoute`, `EnforcementProfile`, and
`GatewayProviderConnection` are exported from this package.

Streaming gateway requests are not supported yet.

## Requirements

- Node.js 18+
- TypeScript 5+ (optional but recommended)

## License

Apache-2.0
