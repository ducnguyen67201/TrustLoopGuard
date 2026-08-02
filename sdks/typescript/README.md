# @featherlane-ai/sdk

TypeScript SDK for Featherlane AI runtime guardrails.

The happy path is intentionally small:

```bash
npm install @featherlane-ai/sdk
```

```ts
import { guardAgent } from '@featherlane-ai/sdk';

const agent = guardAgent(createAgent(), { agentId: 'support-agent' });

const reply = await agent.reply(userMessage);
sendToUser(reply);
```

`guardAgent(...)` is the decorator. Put it where the agent is created, then
leave every `agent.reply(...)` and local tool call site alone.

## Before you start

Create an agent and runtime API key in the Featherlane AI dashboard. You need:

- the registered agent ID;
- the runtime API URL;
- a runtime API key.

You do not need to clone this repository, run Featherlane AI locally, or
configure a model-provider proxy.

## 1. Install One Package

```bash
npm install @featherlane-ai/sdk
```

## 2. Configure

Set the URL and runtime key created in the Featherlane AI dashboard:

```bash
export FEATHERLANE_AI_URL=https://api.featherlane.ai
export FEATHERLANE_AI_API_KEY=tl_live_...
```

The SDK reads these variables automatically.

## 3. Decorate the Agent Once

Decorate the agent object once when you create it:

```ts
import { guardAgent } from '@featherlane-ai/sdk';

const agent = guardAgent(createAgent(), { agentId: 'support-agent' });

const reply = await agent.reply(userMessage);
sendToUser(reply);
```

`guardAgent(...)` returns the same agent type. Existing `agent.reply(...)` call
sites stay unchanged. The decorator:

- discovers supported local tools from OpenAI Agents JS `agent.tools`, LiveKit
  `agent.toolCtx`, Mastra `getToolsForExecution()`, and compatible object maps;
- wraps each local tool `execute()` through `POST /v1/events` before the real
  side effect runs;
- sends the exact tool name, proposed parameters, framework identity, and a
  stable schema identity;
- creates one `chat_session` Run for each `reply()` when no Run is already
  active, or reuses one session Run when a framework lifecycle is configured;
- records the input as a `user_turn` Run event without evaluating it;
- links every guarded tool and output trace in that boundary to the Run;
- delegates to the original `reply()` method;
- records the proposed reply as an `assistant_turn` Run event;
- submits the final returned string to `POST /v1/events`;
- completes the automatic Run, or marks it failed when the agent throws;
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

Open the resulting Run and trace in the Featherlane AI dashboard to verify the
integration.

## What happens on every wrapped call

1. Your application calls `agent.reply(message)`.
2. The SDK starts a `chat_session` Run for the configured `agentId` unless the
   call is already inside `client.withRun(...)`.
3. The SDK records a `user_turn` event, then the original agent generates a draft string.
4. The SDK records an `assistant_turn` event and sends an authenticated `POST /v1/events` request directly to the
   Featherlane AI Rust API.
5. The server evaluates the draft and persists a trace linked to the Run.
6. The SDK completes the Run and returns the permitted draft, a transformed reply, or a safe
   fallback.

Automatic Run bookkeeping is best-effort and never replaces the guard result
or the agent's own error. Pass `run: false` to keep these traces ungrouped, or
pass `run: { kind: 'workflow' }` to change the automatic Run kind. Explicit
`client.withRun(...)` scopes remain available for multi-turn sessions and are
reused rather than nested.

Automatic Run and transcript scoping uses the isolated async context available
in the SDK's supported Node.js runtime. If an unsupported browser/edge runtime
cannot provide that isolation, tool/output guards still run but automatic Run
and transcript capture are skipped to prevent cross-session data leakage.

### Keep one Run for a LiveKit session

The default reply boundary is safe for generic agents because an agent object
may serve many unrelated users. When the framework exposes a real session end,
bind that lifecycle once while decorating the agent:

~~~ts
import { guardAgent, liveKitRun } from '@featherlane-ai/sdk';

const session = createLiveKitAgentSession();
const agent = guardAgent(createAgent(), {
  agentId: 'support-agent',
  run: liveKitRun(session, {
    externalId: roomSid,
    metadata: { integrationName: 'livekit' },
  }),
});

await session.start({ agent, room });
~~~

The first guarded output or local tool call lazily creates one live_call Run.
Later guarded activity from the same wrapped session reuses its run ID. The Run
stays running until LiveKit emits close: model/session errors finish it as
failed, job shutdown finishes it as canceled, and normal participant, user, or
task completion finishes it as completed.

The helper is structurally typed and does not add LiveKit as an SDK dependency.
Use a LiveKit room SID as externalId when available. agentId identifies the
registered agent and must never be used as the customer-session key.

Other frameworks can provide the same deterministic contract directly:

~~~ts
const agent = guardAgent(createAgent(), {
  agentId: 'support-agent',
  run: {
    scope: 'session',
    externalId: chatSession.id,
    registerEnd(finish) {
      return chatSession.onEnd((outcome) =>
        finish(outcome.failed ? 'failed' : 'completed'),
      );
    },
  },
});
~~~

Session Run creation and completion remain best-effort. Use
onLifecycleWarning inside the run options to surface persistence failures
without changing the guarded result. An explicit client.withRun scope still
wins for that async boundary and is never nested.

The event is equivalent to:

```http
POST /v1/events
Authorization: Bearer <FEATHERLANE_AI_API_KEY>
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

With automatic Runs enabled, the raw user message and proposed assistant reply
are stored by default as `user_turn.input_summary` and
`assistant_turn.output_summary`. The user turn is transcript observability only:
it is not sent to the authorization endpoint and receives no policy decision.
The proposed assistant output and local executable tools remain guarded through
`POST /v1/events` before output delivery or tool execution. Pass `run: false`
to disable automatic Run and transcript persistence without disabling those
guards.

| Server effect | What `reply()` returns |
| --- | --- |
| `permit` | The original draft |
| `transform` | The server's safe transformed value |
| `deny` | The configured block message |
| `require_approval` | The configured holding message |
| `defer` | The configured retry-later message |
| Transport failure | A safe block by default for wrapped agents |

### Agent and tool contracts

When the agent exposes `reply()`, its first argument must be the user message
string and it must return `Promise<string>`:

```ts
interface ReplyAgent {
  reply(message: string, ...args: unknown[]): Promise<string>;
}
```

For local tools, the SDK looks for an `execute(input, ...context)` function plus
the framework's name, description, and input schema fields. It preserves
additional execution context arguments while replacing the proposed input with
the exact parameters authorized by Featherlane AI.

```ts
const agent = guardAgent(
  createAgent({
    tools: { weatherTool, bookAppointment, sendEmail },
  }),
  { agentId: 'support-agent' },
);
```

No `withAuthorizedAction(...)` call is added to those three tool
implementations. The decorator installs that authorization boundary once.

OpenAI-hosted tools, remote MCP tools hidden behind a framework, and any tool
whose local `execute()` is not exposed cannot be intercepted before execution.
Use their host adapter or an explicit typed helper at the boundary you own.

### Optional tool metadata registration

Discovery and tool-call guarding are automatic. Control-plane metadata
registration is off by default because it writes workspace configuration.
Enable lazy registration when the application should own that setup:

```ts
import { ToolRegistrationMode, guardAgent } from '@featherlane-ai/sdk';

const agent = guardAgent(createAgent({ tools }), {
  agentId: 'support-agent',
  tools: {
    register: ToolRegistrationMode.BestEffort,
    inferMetadata(tool) {
      return {
        side_effect: tool.name === 'send-email' ? 'external_communication' : 'read',
        reversible: false,
        params: [],
      };
    },
    onDiscoveryWarning(warning) {
      logger.warn(warning);
    },
  },
});
```

`best_effort` reports a warning and continues to authorization when
registration fails. `strict` stops the first tool call before authorization or
execution. Registration occurs once, lazily, before the first call to each
discovered tool.

### Function-Only Integrations

If a framework exposes only a function instead of an agent object, the
lower-level wrapper remains available. Use this only when there is no
agent-shaped object to decorate:

```ts
import { guard } from '@featherlane-ai/sdk';

const guardedReply = guard({
  agentId: 'support-agent',
}).wrap(generateReply);
```

## Guard an existing draft

When input and draft are already separate values, use the callable guard:

```ts
import { guard } from '@featherlane-ai/sdk';

const protect = guard({
  agentId: 'support-agent',
});

const reply = await protect({
  input: userMessage,
  draft: agentDraft,
});
```

Standalone guards fail closed on transport, decode, and retry-exhaustion errors
by default. `failClosed` can be set on the guard factory or an individual guard
call. Set it to `false` only when the integration explicitly accepts returning
the unchecked draft during these failures.

## Guard modes

| Mode | Behavior |
|------|----------|
| `strict` | Blocked or transformed output is rejected |
| `rewrite` | Uses the safe transformed output and blocks when none exists |
| `rewrite_or_regenerate` | Uses the transformed output or invokes your regeneration callback |

```ts
import { GuardMode, guardAgent } from '@featherlane-ai/sdk';

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

Use `withAuthorizedAction` when a framework does not expose its local tool
registry, when tools are created dynamically outside the decorated agent, or
when the caller must attach explicit provenance. Payments and typed financial
actions remain on their dedicated helpers.

## Requirements

- Node.js 22+
- TypeScript 5+ recommended

## Troubleshooting

- No trace appears: check `FEATHERLANE_AI_URL`, `FEATHERLANE_AI_API_KEY`, and that `agentId` matches
  the dashboard agent.
- `401 Unauthorized`: create or copy a runtime key for the same workspace and
  environment as the agent.
- Your framework does not expose `reply(): Promise<string>`: local tools can
  still be discovered, but guard the final framework result with the
  function-only `.wrap()` form.
- A hosted or hidden tool was not wrapped: provide
  `tools.onDiscoveryWarning`, then use the framework host adapter or
  `withAuthorizedAction` for that boundary.
- Streaming: buffer the complete response with `protect.stream(...)` before
  sending any tokens to the user.

## License

Apache-2.0
