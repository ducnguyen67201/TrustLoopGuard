# Agent breaker

The agent breaker holds adversarial demo cases for testing an agent through
TrustLoopGuard.

For now it supports chat only:

- `buildChatBreakCases()` takes the target agent prompt/profile.
- It creates clean and adversarial user prompts for that agent.
- `expect` describes what the guarded gateway response should do.

The networked demo sends each generated prompt over HTTP:

```text
chat breaker -> /arena/chat -> chat agent -> TrustLoopGuard gateway -> mock provider
```

Run it against a waiting proxy agent:

```sh
pnpm demo:agent-breaker
```

By default it attacks `http://127.0.0.1:8788`. Override the target with
`PROXY_AGENT_URL`. The target must expose `GET /arena/profile` and
`POST /arena/chat`.

Add new attack patterns in `chat.ts`. Keep each generated case small enough
that it is clear which policy or gateway behavior it is supposed to test.
