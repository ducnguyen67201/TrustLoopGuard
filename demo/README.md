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
| `OPENAI_API_KEY` | unset | Enables real OpenAI-backed replies |
| `OPENAI_MODEL` | `gpt-4.1-mini` | OpenAI model for LLM-backed replies |

## NorthPay dispute

This is the smallest useful demo: an OpenAI SDK chat agent with one tool,
`issue_refund(amount, account, reason)`.

```sh
pnpm --filter @trustloopguard/demo dispute:check
```

For the Attacks tab, the demo exposes the same agent in two modes:

- Raw target root: `http://127.0.0.1:9201`
- Guarded target root: `http://127.0.0.1:9202`

Use the root URL in the Attacks page and in saved agent config. The arena
adapter exposes both protocols:

- HackAgent/OpenAI-compatible chat: `/v1/models` and `/v1/chat/completions`
- Simple runner/manual chat: `/arena/chat`

HackAgent can initiate chat through `/v1/chat/completions`; manual curl can use
`/arena/chat`.

Set up the refund tool metadata once with the Rust server running:

```sh
pnpm --filter @trustloopguard/demo dispute:setup:doppler
```

Start the dispute adapters:

```sh
pnpm --filter @trustloopguard/demo dispute:serve:doppler
```

Open `http://localhost:3000/attacks`, then run against each root target:

1. `http://127.0.0.1:9201` should show the raw dispute agent issuing the
   attacker-directed refund.
2. `http://127.0.0.1:9202` should show the same proposed refund blocked by the
   guard when the workspace has the dispute tool metadata enabled.

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
