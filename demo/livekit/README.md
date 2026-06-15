# LiveKit agent guardrail demo

This demo mirrors the LiveKit healthcare-agent pattern: define an `Agent`, start an
`AgentSession`, and protect replies before they are spoken or sent.

There are two integration modes:

- `guarded_healthcare_agent.py` uses SDK mode. The app receives a `Decision` and
  applies it before calling `session.say(...)`.
- `proxy_healthcare_agent.py` uses gateway mode. LiveKit's OpenAI-compatible LLM
  points at `/v1/gateway/<route_id>/openai`, and TrustLoopGuard applies the
  decision inside the proxy.

## SDK mode

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

## Modes

Use `strict` when unsafe output should stop immediately, `rewrite` when
TrustLoopGuard safe output is enough, and `rewrite_or_regenerate` when the app
should ask the model for a safer answer in real time.

<!-- BEGIN recipe:output-boundary-guard:python_modes -->

```py
import trustloopguard as trustloop

strict_guardrail = trustloop.guard(
    agent_id="support-agent",
    mode=trustloop.GuardMode.STRICT,
)

rewrite_guardrail = trustloop.guard(
    agent_id="support-agent",
    mode=trustloop.GuardMode.REWRITE,
)

async def regenerate_reply(feedback: trustloop.RegenerateFeedback) -> str:
    return await model.generate(
        instructions=(
            "The previous draft was blocked by TrustLoopGuard: "
            f"{feedback.reason}. Generate a safer answer."
        )
    )

regenerating_guardrail = trustloop.guard(
    agent_id="support-agent",
    mode=trustloop.GuardMode.REWRITE_OR_REGENERATE,
    regenerate=regenerate_reply,
    max_regenerations=1,
)
```

<!-- END recipe:output-boundary-guard:python_modes -->

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

## Files

- `minimal_agent_guard.py` shows the smallest copyable one-time guardrail setup.
- `guarded_healthcare_agent.py` shows the same pattern inside a LiveKit `Agent`
  shaped like the upstream healthcare example.
- `proxy_healthcare_agent.py` shows gateway mode by configuring LiveKit's OpenAI
  plugin with a TrustLoopGuard gateway base URL.
- `../README.md` lists the rest of the SDK-backed demo surfaces.

## Setup (isolated env)

This demo keeps its own virtualenv and dependency set under `demo/livekit/`, so
it does not touch the rest of the repo's Python tooling. Run everything from this
directory:

```sh
cd demo/livekit
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt          # LiveKit Agents + the editable TrustLoopGuard SDK
python proxy_healthcare_agent.py download-files   # pre-fetch silero/turn-detector weights
```

`requirements.txt` installs the SDK editable from `../../sdks/python` (the repo's
single source of truth — it is not vendored into this folder).

Secrets come from Doppler, never a `.env` file (repo convention). The demo reads
them from the `trustloopguard_demo_agent` project's `dev_livekit` config, so every
command below is wrapped in `doppler run`.

## Run SDK mode

Start TrustLoopGuard (from the repo root, in another terminal):

```sh
make server
```

Then run the demo from `demo/livekit/` with secrets injected by Doppler:

```sh
TL_AGENT_ID=demo-healthcare-livekit \
doppler run -p trustloopguard_demo_agent -c dev_livekit -- \
  python guarded_healthcare_agent.py dev
```

Optional:

- `TL_API_KEY` (in Doppler) if your TrustLoopGuard server requires auth.
- LiveKit provider env vars (`LIVEKIT_URL` / `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET`)
  live in the same Doppler config.

For realtime voice, the sample uses a 250 ms timeout and one SDK attempt:

```py
retry=RetryConfig(max_attempts=1, total_budget_s=0.25)
```

That keeps the demo aligned with live-call latency expectations while still
using the same SDK `guard()` helper as the chat demo.

## Run gateway mode

First create a TrustLoopGuard gateway route in the dashboard **Gateway** page.
The route must use an OpenAI-compatible provider connection. Start the server
first:

```sh
make server                              # = doppler run -- cargo run -p tl-server
```

Once the route reads `Ready`, the Gateway page shows the workspace, route id,
and a workspace runtime key:

```text
workspace: ws_proxy_demo_...
route    : demo-proxy-route-...
key      : tl_live_...
```

Copy the route id and runtime key into the Doppler config (not a `.env` file):

```sh
doppler secrets set -p trustloopguard_demo_agent -c dev_livekit \
  TL_GATEWAY_ROUTE_ID=demo-proxy-route-... \
  TLG_API_KEY=tl_live_...
```

`TL_SERVER_URL` and `OPENAI_MODEL` already live in that config; set
`OPENAI_API_KEY` and the three `LIVEKIT_*` values there too before running.

Then run the LiveKit gateway demo from `demo/livekit/`:

```sh
doppler run -p trustloopguard_demo_agent -c dev_livekit -- \
  python proxy_healthcare_agent.py dev
```

LiveKit calls TrustLoopGuard as if it were an OpenAI-compatible provider:

```text
LiveKit AgentSession
  -> /v1/gateway/<route_id>/openai
  -> TrustLoopGuard input check
  -> provider
  -> TrustLoopGuard output check
  -> LiveKit agent reply
```

In the dashboard, gateway traffic appears under the route workspace's runs and
traces. The raw provider key never leaves the TrustLoopGuard provider
connection; the LiveKit process only uses the workspace runtime key. The demo
sends the LiveKit room id as `X-TLG-Run-External-Id`, so repeated model calls
from one room are grouped into one dashboard run.

The LiveKit proxy demo intentionally produces unsafe output by default for
output-enforcement testing. Its greeting and normal scheduling replies try to
say `That is a stupid question. Figure it out yourself.`, which the gateway
rewrite policy should replace before LiveKit speaks it. Refund questions try to
say `We guarantee a full refund immediately.`, which the gateway block policy
should stop. The proxy demo profile uses `full_body` retention so the run detail
page can show the bounded checked output excerpt for local debugging.
