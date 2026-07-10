# @trustloopguard/sdk

TypeScript SDK for [TrustLoopGuard](https://trustloopguard.dev) — real-time guardrails for AI agents.

## Installation

```bash
npm install @trustloopguard/sdk
```

## Quick start

```ts
import { Client, guard } from '@trustloopguard/sdk';

const client = new Client({
  baseUrl: process.env.TLG_URL ?? 'http://127.0.0.1:8080',
  apiKey: process.env.TLG_API_KEY,
});

const reply = await client.withRun({ agentId: 'my-agent', kind: 'chat_session' }, async (run) => {
  return run.withEvent({ kind: 'user_turn', metadata: {} }, () =>
    guard({
      client,
      agentId: 'my-agent',
      input: userMessage,
      draft: agentDraft,
      onBlock: () => "I can't help with that.",
      onEscalate: () => 'A human will follow up.',
    }),
  );
});
await sendToUser(reply);
```

`withRun` groups `guard()`, `submitEvent()`, and `guardToolCall()` calls under
the active run. Explicit `runId` / `runEventId` fields still win.

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

const decision = await client.submitEvent({
  kind: 'output.proposed',
  principal: { workspace_id: '', environment_id: '', agent_id: 'my-agent' },
  action: { operation: 'output', parameters: { text: agentDraft }, side_effect: 'none' },
  sources: [{ id: 'input', origin: 'user', labels: {} }],
  provenance: { text: ['input'] },
  context: { channel: 'chat', domain: 'customer_support' },
});
```

Tool-call events can use the thin helper:

```ts
await client.guardToolCall({
  agentId: 'my-agent',
  operation: 'issue_refund',
  parameters: { orderId },
  sideEffect: 'api_mutation',
  sources: [{ id: 'input', origin: 'user', labels: {} }],
  provenance: { orderId: ['input'] },
});
```

## Gateway mode

The SDK keeps full control in your code. Gateway mode is the proxy path:
configure a provider connection and an agent route in the dashboard, then point
provider traffic at TrustLoopGuard. Enabled policies apply automatically.

OpenAI-compatible example:

```ts
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: process.env.TLG_API_KEY,
  baseURL: 'https://api.gettrustloop.app/v1/gateway/<route_id>/openai',
});

const response = await openai.chat.completions.create({
  model: 'gpt-4o-mini',
  messages: [{ role: 'user', content: userMessage }],
  max_tokens: 512,
});
```

Anthropic example:

```ts
import Anthropic from '@anthropic-ai/sdk';

const anthropic = new Anthropic({
  authToken: process.env.TLG_API_KEY,
  baseURL: 'https://api.gettrustloop.app/v1/gateway/<route_id>/anthropic',
});
```

SDK mode returns a decision for your code to handle. Gateway mode applies the
same policy decision before returning a provider-compatible response.

Gateway configuration types such as `GatewayRoute` and
`GatewayProviderConnection` are exported from this package.

Streaming requests are buffered, checked, and returned as provider-native SSE.

## Requirements

- Node.js 22+
- TypeScript 5+ (optional but recommended)

## License

Apache-2.0
