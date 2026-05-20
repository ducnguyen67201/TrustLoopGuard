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

## Gateway proxy

Runs an end-to-end gateway smoke test without calling a paid provider:

```sh
TL_API_KEY=dev-admin cargo run -p tl-server
TL_API_KEY=dev-admin pnpm demo:proxy
```

The demo creates a workspace, runtime key, provider connection, enforcement
profile, and gateway route. It starts a local OpenAI-compatible mock provider,
calls `/v1/gateway/<route_id>/openai/chat/completions`, and asserts that
TrustLoopGuard blocks the unsafe mock output with a provider-shaped
`content_filter` response.

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

The LiveKit demo is Python because it follows the LiveKit Agents runtime:

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
