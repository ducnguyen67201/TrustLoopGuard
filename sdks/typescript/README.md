# @trustloopguard/sdk

TypeScript SDK for TrustLoopGuard runtime guardrails.

The happy path is intentionally small:

```bash
npm install @trustloopguard/sdk
```

```ts
import { guardAgent } from '@trustloopguard/sdk';

const agent = guardAgent(createAgent(), { agentId: 'support-agent' });

const reply = await agent.reply(userMessage);
sendToUser(reply);
```

`guardAgent(...)` is the decorator. Put it where the agent is created, then
leave every `agent.reply(...)` call site alone.

## Before you start

Create an agent and runtime API key in the TrustLoopGuard dashboard. You need:

- the registered agent ID;
- the runtime API URL;
- a runtime API key.

You do not need to clone this repository, run TrustLoopGuard locally, or
configure a model-provider proxy.

## 1. Install One Package

```bash
npm install @trustloopguard/sdk
```

## 2. Configure

Set the URL and runtime key created in the TrustLoopGuard dashboard:

```bash
export TLG_URL=https://api.gettrustloop.app
export TLG_API_KEY=tl_live_...
```

The SDK reads these variables automatically.

## 3. Decorate the Agent Once

Decorate the agent object once when you create it:

```ts
import { guardAgent } from '@trustloopguard/sdk';

const agent = guardAgent(createAgent(), { agentId: 'support-agent' });

const reply = await agent.reply(userMessage);
sendToUser(reply);
```

`guardAgent(...)` returns the same agent type. Existing `agent.reply(...)` call
sites stay unchanged. The decorator:

- delegates to the original `reply()` method;
- submits the final returned string to `POST /v1/events`;
- returns the original or safely transformed reply on success;
- returns a safe fallback for deny, approval, defer, and SDK failure branches;
- preserves the rest of the agent's public interface and `reply()` arguments.

The decorated agent fails closed on SDK transport errors by default. Set
`failClosed: false` when availability is more important than enforcement.

If your app already has a helper like this:

```ts
async function generateReply(message: string): Promise<string> {
  return await agent.reply(message);
}

const reply = await generateReply(userMessage);
sendToUser(reply);
```

do not add guard checks inside the helper. Decorate the agent once before the
helper sees it:

```ts
const agent = guardAgent(createAgent(), { agentId: 'support-agent' });
```

The helper keeps working because `agent.reply(...)` is still the same method
from the caller's point of view.

## 4. Send a test reply

Keep the rest of your application unchanged:

```ts
const reply = await agent.reply('Can I receive a refund?');
sendToUser(reply);
```

Open the resulting trace in the TrustLoopGuard dashboard to verify the
integration.

## What happens on every wrapped call

1. Your application calls `agent.reply(message)`.
2. The original agent generates a draft string.
3. The SDK sends an authenticated `POST /v1/events` request directly to the
   TrustLoopGuard Rust API.
4. The server evaluates the draft and persists a trace.
5. The decorator returns the permitted draft, a transformed reply, or a safe
   fallback.

The event is equivalent to:

```http
POST /v1/events
Authorization: Bearer <TLG_API_KEY>
Content-Type: application/json
```

```json
{
  "kind": "output.proposed",
  "principal": {
    "workspace_id": "",
    "environment_id": "",
    "agent_id": "support-agent"
  },
  "action": {
    "operation": "output",
    "parameters": {
      "text": "<agent draft>"
    },
    "side_effect": "none"
  },
  "context": {
    "channel": "chat",
    "domain": "customer_support"
  }
}
```

The SDK also adds source and provenance metadata, while the server resolves
workspace and environment scope from the runtime key.

The raw user message is used locally to call the agent and support optional
regeneration, but this output wrapper does not include the raw message text in
the event by default. It guards the final reply boundary, not hidden framework
internals.

| Server effect | What `reply()` returns |
| --- | --- |
| `permit` | The original draft |
| `transform` | The server's safe transformed value |
| `deny` | The configured block message |
| `require_approval` | The configured holding message |
| `defer` | The configured retry-later message |
| Transport failure | A safe block by default for wrapped agents |

### Agent contract

The first `reply()` argument must be the user message string and the method must
return `Promise<string>`:

```ts
interface ReplyAgent {
  reply(message: string, ...args: unknown[]): Promise<string>;
}
```

This root decorator intercepts each invocation crossing `reply()` and guards
the final returned response. Calls hidden inside a framework, such as tools or
payments, still use framework adapters or explicit typed helpers because they
require exact parameters and execution guarantees.

### Function-Only Integrations

If a framework exposes only a function instead of an agent object, the
lower-level wrapper remains available. Use this only when there is no
agent-shaped object to decorate:

```ts
import { guard } from '@trustloopguard/sdk';

const guardedReply = guard({
  agentId: 'support-agent',
}).wrap(generateReply);
```

## Guard an existing draft

When input and draft are already separate values, use the callable guard:

```ts
import { guard } from '@trustloopguard/sdk';

const protect = guard({
  agentId: 'support-agent',
});

const reply = await protect({
  input: userMessage,
  draft: agentDraft,
});
```

## Guard modes

| Mode | Behavior |
|------|----------|
| `strict` | Blocked or transformed output is rejected |
| `rewrite` | Uses the safe transformed output and blocks when none exists |
| `rewrite_or_regenerate` | Uses the transformed output or invokes your regeneration callback |

```ts
import { GuardMode, guardAgent } from '@trustloopguard/sdk';

const agent = guardAgent(createAgent(), {
  agentId: 'support-agent',
  mode: GuardMode.Rewrite,
});
```

Custom branch messages remain optional:

```ts
const agent = guardAgent(createAgent(), {
  agentId: 'support-agent',
  onBlock: "I can't help with that.",
  onRequireApproval: 'A human must approve this response.',
  onDefer: 'I need more verified information before continuing.',
});
```

## Streaming output

Token streams must be buffered before any unguarded output is delivered:

```ts
const protect = guard({ agentId: 'support-agent' });
const reply = await protect.stream({
  input: userMessage,
  draft: modelTokenStream,
});
```

## Explicit action helpers

Tool calls, payments, and other side effects remain explicit because they need
exact parameters, provenance, and at-most-once execution guarantees. Use
`guardToolCall`, `withAuthorizedAction`, or the typed financial helpers for
those boundaries.

## Requirements

- Node.js 22+
- TypeScript 5+ recommended

## Troubleshooting

- No trace appears: check `TLG_URL`, `TLG_API_KEY`, and that `agentId` matches
  the dashboard agent.
- `401 Unauthorized`: create or copy a runtime key for the same workspace and
  environment as the agent.
- Your framework does not expose `reply(): Promise<string>`: use the
  function-only `.wrap()` form or a framework adapter.
- Streaming: buffer the complete response with `protect.stream(...)` before
  sending any tokens to the user.

## License

Apache-2.0
