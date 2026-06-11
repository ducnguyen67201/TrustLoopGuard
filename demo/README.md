# TrustLoopGuard demos

These demos exercise the same output-boundary pipeline through the public SDKs:

1. The agent drafts output.
2. The demo calls `guard()` through the TypeScript or Python SDK.
3. TrustLoopGuard returns a decision, trace id, and latency.
4. The demo delivers only the guarded output.

Start the Rust server first:

```sh
cargo run -p tl-server
```

Optional environment:

| Var | Default | Purpose |
| --- | --- | --- |
| `TL_SERVER_URL` | `http://127.0.0.1:8080` | TrustLoopGuard server URL |
| `TL_API_KEY` | unset | Bearer token when the server requires auth |
| `TL_AGENT_ID` | `demo-acme-support` | Demo agent profile id |
| `TL_WORKSPACE_ID` | unset | Optional local workspace header override, e.g. `ws_test` |
| `OPENAI_API_KEY` | unset | Enables real OpenAI-backed replies in interactive chat |
| `OPENAI_MODEL` | `gpt-4.1-mini` | OpenAI model for interactive chat replies |

## 60-second showdown

Start here. One command contrasts an unguarded agent (leaks PII, promises an
unauthorized refund) with the same agent behind TrustLoopGuard (block +
rewrite, with trace IDs and a dashboard link):

```sh
pnpm demo:60s
```

Deterministic, self-seeding, no API keys. See
[`showdown/README.md`](showdown/README.md) for the expected output and a
60-second talk track.

## Live chat

Deterministic scripted scenarios:

```sh
pnpm demo:chat
```

Interactive local chat loop:

```sh
pnpm demo:chat:interactive
```

When `OPENAI_API_KEY` is set, the interactive chat asks OpenAI for the agent
draft before sending that draft through `guard()`. Without it, the demo uses
local deterministic drafts.

## Background job

Runs a few job-style steps and guards each step output:

```sh
pnpm demo:job
```

## Gateway proxy smoke test

Runs the gateway chat-agent flow in one process without calling a paid provider:

```sh
TL_API_KEY=dev-admin \
TL_GATEWAY_CREDENTIAL_KEY=local-demo-gateway-secret \
cargo run -p tl-server

TL_API_KEY=dev-admin pnpm demo:proxy
```

The demo creates a workspace, runtime key, provider connection, enforcement
profile, and gateway route. It starts a local OpenAI-compatible mock provider,
then runs generated breaker prompts against the route base URL. Clean traffic
passes through; breaker traffic is converted into provider-shaped
`content_filter` responses with correlation headers.

## Networked proxy agent

For a more realistic demo, run the proxy agent and breaker as separate local
processes:

```sh
TL_API_KEY=dev-admin \
TL_GATEWAY_CREDENTIAL_KEY=local-demo-gateway-secret \
cargo run -p tl-server

pnpm demo:raw-agent

TL_API_KEY=dev-admin pnpm demo:proxy:agent

pnpm dev
```

Open `http://localhost:3000/arena`, then compare:

- Raw agent URL: `http://127.0.0.1:8787`
- Guarded agent URL: `http://127.0.0.1:8788`

Both agents use `createArenaAdapter()` from `demo/arena/adapter.ts`, which
exposes `GET /arena/profile` and `POST /arena/chat` for them. The arena fetches
profiles in the browser, generates chat attacks, and sends them to both waiting
adapters.

For real-provider arena testing, paste local secrets into ignored env files:

- `demo/proxy/.env` for `TL_API_KEY`, `TL_SERVER_URL`, and `OPENAI_API_KEY`
- `demo/raw-agent/.env` for `OPENAI_API_KEY`

When `OPENAI_API_KEY` is present, the raw agent calls OpenAI directly and the
guarded agent registers OpenAI as the TrustLoopGuard gateway provider. Without
it, both agents keep using the deterministic local mock.

The CLI breaker still works for terminal-only demos:

```sh
pnpm demo:agent-breaker
```

See `proxy/README.md` for the step-by-step setup and expected output.

## Agent breaker

The breaker is chat-only for now. It takes the target agent prompt/profile and
generates a small set of clean and adversarial chat prompts.

## n8n workflow

The workflow calls a tiny local bridge so n8n does not need to load workspace
packages. The bridge is still SDK-backed: it uses `@trustloopguard/sdk` and
calls `guard()` for every request.

Start the bridge:

```sh
pnpm demo:n8n:bridge
```

Then import `demo/n8n/workflow.json` into n8n and run it manually. The
workflow posts a draft to `http://127.0.0.1:8787/guard` and receives:

```json
{
  "guardedOutput": "...",
  "originalDraft": "...",
  "guard": {
    "verdict": "block",
    "branch": "block",
    "traceId": "...",
    "latencyMs": 12
  }
}
```

## LiveKit

The LiveKit demo is Python because it follows the LiveKit Agents runtime. SDK
mode guards the draft right before the agent speaks:

```sh
pip install -e sdks/python
pip install "livekit-agents[openai,silero]" python-dotenv

TL_SERVER_URL=http://127.0.0.1:8080 \
TL_AGENT_ID=demo-healthcare-livekit \
python demo/livekit/guarded_healthcare_agent.py dev
```

For voice, the demo configures a 250 ms guardrail budget and one SDK attempt:
the runtime either returns guarded output within the realtime budget or follows
the SDK's configured failure behavior.

Gateway mode points LiveKit's OpenAI-compatible LLM at TrustLoopGuard instead of
calling the provider directly:

```sh
python demo/livekit/proxy_healthcare_agent.py dev
```

Set `TLG_API_KEY` and `TL_GATEWAY_ROUTE_ID` in `demo/livekit/.env` first. See
`livekit/README.md` for the full setup.
