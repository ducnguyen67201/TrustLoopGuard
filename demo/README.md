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

## NorthPay dispute (terminal demo)

The fastest way to see the guard work. The same payment-dispute agent handles the
same prompt-injection attack twice — once unprotected (the refund executes), once
behind TrustLoopGuard (the refund is gated):

```sh
pnpm --filter @trustloopguard/demo dispute
```

The unprotected half runs with no server. To see the guarded half block the
refund, start the server and register the demo agent + policies first (run
`dispute:setup`, below).

The protected integration is deliberately one line at the agent boundary:

```ts
await agent.handle(message, trustloopGuard(createClient(), AGENT_ID));
```

`trustloopGuard(...)` owns the TrustLoopGuard details: it opens a run, creates a
run event for the proposed action, submits the output/tool event, and the SDK
attaches `run_id` / `run_event_id` automatically. The agent never sees run ids
or TrustLoopGuard-specific event plumbing.

The HTTP adapter groups guarded chat turns into a run session. Send
`x-tlg-session-id` or `sessionId` to group multiple turns explicitly. Without
one, each request starts a fresh local demo session so separate red-team attacks
do not collapse into the same run.

## NorthPay dispute adapters for the Attacks tab

The dispute demo exposes the same payment-dispute agent in two modes:

- Raw target root: `http://127.0.0.1:9201`
- Guarded target root: `http://127.0.0.1:9202`

Use the root URL in the Attacks page and in saved agent config. The arena
adapter exposes both protocols:

- HackAgent/OpenAI-compatible chat: `/v1/models` and `/v1/chat/completions`
- Simple runner/manual chat: `/arena/chat`

So HackAgent can initiate chat through `http://127.0.0.1:9201/v1/...`, while
manual curl still uses `http://127.0.0.1:9201/arena/chat`.

Set up the dispute metadata once with the Rust server running:

```sh
TL_SERVER_URL=http://127.0.0.1:8080 \
TL_API_KEY=dev-admin \
TL_WORKSPACE_ID=ws_demo_workspace \
pnpm --filter @trustloopguard/demo dispute:setup
```

Start the dispute adapters:

```sh
TL_SERVER_URL=http://127.0.0.1:8080 \
TL_API_KEY=dev-admin \
TL_WORKSPACE_ID=ws_demo_workspace \
pnpm --filter @trustloopguard/demo dispute:serve
```

Open `http://localhost:3000/attacks`, then run against each root target:

1. `http://127.0.0.1:9201` should show the raw dispute agent issuing the
   attacker-directed refund.
2. `http://127.0.0.1:9202` should show the same proposed refund blocked by the
   guard when the workspace has the dispute tool metadata/policies enabled.

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
