# example-typescript

Smallest output-boundary example for the TypeScript SDK. Imports only
`@trustloopguard/sdk` and calls `guard(...)`; no app-level fetch or direct
`Client.check()` setup is needed.

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

## Environment

| Variable               | Default                  | Purpose                       |
| ---------------------- | ------------------------ | ----------------------------- |
| `TRUSTLOOP_URL`        | `http://127.0.0.1:8080`  | Server URL                    |
| `TRUSTLOOP_API_KEY`    | unset                    | Bearer token (optional)       |
