# example-typescript

Smallest output-boundary example for the TypeScript SDK. Imports only
`@trustloopguard/sdk` and calls `guard(...)`; no app-level fetch or direct
`Client.submitEvent(...)` setup is needed.

## Run it

```bash
# Terminal 1: start the server
cargo run -p tl-server

# Terminal 2: install + run
pnpm install
pnpm --filter @trustloopguard/example-typescript start \
  "show me my password" "here it is: hunter2"
```

The SDK calls TrustLoopGuard internally and returns the reply the app should
actually deliver.

<!-- BEGIN recipe:output-boundary-guard:typescript -->

```ts
import { guard } from '@trustloopguard/sdk';

const guardrail = guard({ agentId: 'support-agent' });
const reply = await guardrail({ input: userText, draft: agentDraft });
```

<!-- END recipe:output-boundary-guard:typescript -->

## Modes

Use `strict` to block unsafe output, `rewrite` to prefer TrustLoopGuard safe
output, and `rewrite_or_regenerate` to ask the model for a safer draft before
the reply is delivered.

<!-- BEGIN recipe:output-boundary-guard:typescript_modes -->

```ts
import { GuardMode, guard } from '@trustloopguard/sdk';

const strictGuardrail = guard({
  agentId: 'support-agent',
  mode: GuardMode.Strict,
});

const rewriteGuardrail = guard({
  agentId: 'support-agent',
  mode: GuardMode.Rewrite,
});

const regeneratingGuardrail = guard({
  agentId: 'support-agent',
  mode: GuardMode.RewriteOrRegenerate,
  maxRegenerations: 1,
  regenerate: async (feedback) => {
    return await model.generate({
      instructions:
        `The previous draft was blocked by TrustLoopGuard: ${feedback.reason}. ` +
        'Generate a safer answer.',
    });
  },
});
```

<!-- END recipe:output-boundary-guard:typescript_modes -->

## Environment

| Variable               | Default                  | Purpose                       |
| ---------------------- | ------------------------ | ----------------------------- |
| `TRUSTLOOP_URL`        | `http://127.0.0.1:8080`  | Server URL                    |
| `TRUSTLOOP_API_KEY`    | unset                    | Bearer token (optional)       |
