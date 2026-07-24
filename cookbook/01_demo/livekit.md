# LiveKit Voice Agent

## What It Teaches

Apply TrustLoopGuard to a realtime Python voice agent in either integration
mode:

- **SDK mode:** buffer the draft, call `guard()`, then speak only the guarded
  reply.
- **Gateway mode:** point LiveKit's OpenAI-compatible model client at a
  TrustLoopGuard gateway route.

The SDK example uses a 250 ms budget and one attempt so failure behavior is
bounded for a live conversation.

## Fastest Check

Compile the three examples without starting LiveKit:

```bash
python -m py_compile \
  demo/livekit/minimal_agent_guard.py \
  demo/livekit/guarded_healthcare_agent.py \
  demo/livekit/proxy_healthcare_agent.py
```

## Run It

Create the isolated environment and install its requirements:

```bash
cd demo/livekit
python -m venv .venv
pip install -r requirements.txt
```

Use `guarded_healthcare_agent.py dev` for SDK mode. Use
`proxy_healthcare_agent.py dev` after creating a ready OpenAI-compatible
gateway route and setting its route ID plus workspace runtime key.

## Expected Proof

- No draft is spoken before the guard returns.
- Realtime checks stay within the configured attempt and time budget.
- Gateway mode groups calls from one LiveKit room into one dashboard Run.
- The provider key remains in the Rust-owned gateway connection.

## Read The Code

- [`minimal_agent_guard.py`](../../demo/livekit/minimal_agent_guard.py) is the
  smallest Python integration.
- [`guarded_healthcare_agent.py`](../../demo/livekit/guarded_healthcare_agent.py)
  shows SDK mode.
- [`proxy_healthcare_agent.py`](../../demo/livekit/proxy_healthcare_agent.py)
  shows gateway mode.
- [`demo/livekit/README.md`](../../demo/livekit/README.md) owns full setup.
