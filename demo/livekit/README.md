# LiveKit agent guardrail demo

This demo mirrors the LiveKit healthcare-agent pattern: define an `Agent`, start an
`AgentSession`, and guard drafts before they are spoken or sent.

Create the TrustLoopGuard guardrail once when the session starts. The repeated
runtime call is the output-boundary check: it is what lets TrustLoopGuard rewrite,
block, or escalate a draft before LiveKit speaks it.

The integration point is intentionally small:

<!-- BEGIN recipe:output-boundary-guard:python_livekit -->

```py
import trustloopguard as trustloop

guardrail = trustloop.guard(
    agent_id="demo-healthcare-livekit",
    base_url="http://127.0.0.1:8080",
    channel=trustloop.Channel.voice,
)

guarded_reply = await guardrail(
    input=user_text,
    draft=agent_draft,
)

await session.say(guarded_reply)
```

<!-- END recipe:output-boundary-guard:python_livekit -->

There is no app-level `fetch` call. The SDK owns the guardrail request.

The same output-boundary shape exists in TypeScript for chat agents and regular workflows:

<!-- BEGIN recipe:output-boundary-guard:typescript -->

```ts
import { guard } from '@trustloopguard/sdk';

const guardrail = guard({ agentId: 'support-agent' });
const reply = await guardrail({ input: userText, draft: agentDraft });
```

<!-- END recipe:output-boundary-guard:typescript -->

## Files

- `minimal_agent_guard.py` shows the smallest copyable one-time guardrail setup.
- `guarded_healthcare_agent.py` shows the same pattern inside a LiveKit `Agent`
  shaped like the upstream healthcare example.

## Run

Install LiveKit agent dependencies and the local Python SDK in your Python env:

```sh
pip install -e sdks/python
pip install "livekit-agents[openai,silero]" python-dotenv
```

Start TrustLoopGuard:

```sh
cargo run -p tl-server
```

Then run the demo with the same LiveKit environment variables the upstream examples use:

```sh
TL_SERVER_URL=http://127.0.0.1:8080 \
TL_AGENT_ID=demo-healthcare-livekit \
python demo/livekit/guarded_healthcare_agent.py dev
```

Optional:

- `TL_API_KEY` if your TrustLoopGuard server requires auth.
- LiveKit provider env vars required by your local LiveKit Agents setup.
