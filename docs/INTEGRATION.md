# Integrating TrustLoopGuard

This document is the **fastest path** from "we have an agent in production" to "every agent reply is run through TrustLoopGuard." It covers:

1. The two-step model: register the **agent profile** once, then call `guard()` on every reply.
2. **TypeScript** quickstart.
3. **Python** quickstart (sync + async).
4. **Raw HTTP** for any other language.
5. **Fail-open vs fail-closed**: what `guard()` does on transport errors and how to override.

For the protocol itself see [`docs/openapi.yaml`](openapi.yaml). For why the runtime is shaped the way it is see [`docs/concept/v0-design-decisions.md`](concept/v0-design-decisions.md).

---

## The two-step model

```
                      one-time, off the hot path
              ┌──────────────────────────────────────────┐
              │  POST /v1/agents  { yaml: "..." }        │
              └──────────────────────────────────────────┘

                      on every agent reply
                ┌──────────────────────────────┐
   user msg ──► │  your agent → draft          │
                │  guard({ agent_id, input,    │ ──► send to user
                │          draft, callbacks }) │
                └──────────────────────────────┘
```

You register the **agent profile** once (a YAML doc describing what the agent is allowed to say, its tone, who it is, where its knowledge comes from). After that, every check just sends `agent_id` plus the draft. Tier 3 LLM judges read the profile server-side — you never have to resend it.

A minimal profile:

```yaml
# policies/agents/acme-support-v3.yaml
agent_id: acme-support-v3
display_name: Acme customer support
scope:
  in_scope:
    - billing
    - account management
    - product setup
  out_of_scope:
    - legal advice
    - medical advice
authority:
  can_promise:
    - issuing refunds within 30 days
  cannot_promise:
    - lifetime discounts
    - access to unreleased features
tone:
  target: "warm, concise, factual"
  forbidden: ["promise", "guaranteed", "definitely"]
knowledge_sources:
  - id: kb-v3
    description: Acme product knowledge base
```

Register it:

```bash
curl -X POST https://your-trustloopguard/v1/agents \
  -H "Authorization: Bearer $TL_ADMIN_API_KEY" \
  -H "Content-Type: application/yaml" \
  --data-binary @policies/agents/acme-support-v3.yaml
```

Use scoped keys in production:

- `TL_RUNTIME_API_KEY` is for agent runtime calls to `POST /v1/check`.
- `TL_ADMIN_API_KEY` is for policy and agent authoring endpoints, and can also call `/v1/check`.
- `TL_API_KEY` is still accepted as a legacy admin key for existing deployments.

---

## TypeScript quickstart

```bash
pnpm add @trustloopguard/sdk
# or: npm i @trustloopguard/sdk / yarn add @trustloopguard/sdk
```

```ts
import { Client, guard } from "@trustloopguard/sdk";

const client = new Client({
  baseUrl: process.env.TLG_URL!,
  apiKey: process.env.TLG_API_KEY!,
});

export async function reply(userMessage: string): Promise<string> {
  const draft = await myAgent.generate(userMessage); // your existing agent

  return guard({
    client,
    agentId: "acme-support-v3",
    input: userMessage,
    draft,
    context: { docs: await retrieveDocs(userMessage) },

    onBlock:    () => "I can't help with that — let me hand you to a teammate.",
    onEscalate: (d) => { humanQueue.push(d); return "Hold tight — a human is joining."; },

    // optional — defaults are fine
    onRevise: (revised) => revised ?? draft,
    onAllow:  (d) => d, // pass-through
    onError:  (_err, d) => d, // fail-open default

    log: (e) => logger.info({ tlg: e }),
  });
}
```

Verdict-to-callback mapping (same in both SDKs):

| verdict   | callback        | default                                     |
|-----------|-----------------|---------------------------------------------|
| `allow`   | `onAllow`       | return the draft unchanged                  |
| `rewrite` | `onRevise`      | return `decision.safe_output ?? draft`      |
| `block`   | `onBlock`       | **required** — you must provide this        |
| `escalate`| `onEscalate`    | **required** — you must provide this        |

---

## Python quickstart

```bash
pip install trustloopguard
```

### Sync

```python
import os
from trustloopguard import Client, guard

client = Client(
    base_url=os.environ["TLG_URL"],
    api_key=os.environ["TLG_API_KEY"],
)

def reply(user_message: str) -> str:
    draft = my_agent.generate(user_message)

    return guard(
        client=client,
        agent_id="acme-support-v3",
        input=user_message,
        draft=draft,
        context={"docs": retrieve_docs(user_message)},
        on_block=lambda _d: "I can't help with that — let me hand you to a teammate.",
        on_escalate=lambda d: (human_queue.push(d), "Hold tight — a human is joining.")[1],
    )
```

### Async

```python
import os
from trustloopguard import AsyncClient, guard_async

client = AsyncClient(
    base_url=os.environ["TLG_URL"],
    api_key=os.environ["TLG_API_KEY"],
)

async def reply(user_message: str) -> str:
    draft = await my_agent.generate(user_message)

    async def block(_d): return "I can't help with that."
    async def escalate(d):
        await human_queue.push(d)
        return "Hold tight — a human is joining."

    return await guard_async(
        client=client,
        agent_id="acme-support-v3",
        input=user_message,
        draft=draft,
        on_block=block,
        on_escalate=escalate,
    )
```

---

## Raw HTTP

For any language without a first-class SDK. The protocol is two endpoints:

### `POST /v1/agents`

Register/update an agent profile.

```bash
curl -X POST $TLG_URL/v1/agents \
  -H "Authorization: Bearer $TLG_API_KEY" \
  -H "Content-Type: application/yaml" \
  --data-binary @profile.yaml
```

### `POST /v1/check`

Run a check.

```bash
curl -X POST $TLG_URL/v1/check \
  -H "Authorization: Bearer $TLG_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "acme-support-v3",
    "channel": "chat",
    "input": "Can I get a refund?",
    "proposed_output": "Yes, full refund within 14 days — guaranteed.",
    "context": { "docs": ["Refund policy: 30 days, see clause 4.2."] }
  }'
```

Response shape:

```json
{
  "trace_id": "018f0c43-…",
  "verdict": "rewrite",
  "reason": "Tone: forbidden word 'guaranteed'.",
  "triggered_policies": [...],
  "safe_output": "Yes — our refund window is 30 days.",
  "latency_ms": 142,
  "tier_results": [
    {"tier": "Deterministic", "elapsed_ms": 1,  "status": "Allow",   "reasons": []},
    {"tier": "Fuzzy",         "elapsed_ms": 11, "status": "Allow",   "reasons": []},
    {"tier": "Llm",           "elapsed_ms": 128,"status": "Revise",  "reasons": ["tone:guaranteed"]}
  ]
}
```

Branch on `verdict` — `allow | rewrite | block | escalate`. Full schema is in [`docs/openapi.yaml`](openapi.yaml).

---

## Fail-open vs fail-closed

The runtime can fail in two places: the **network** (request never reaches TrustLoopGuard) and the **LLM tier** (Tier 3 timed out or its budget is exhausted).

### Network failures

Handled in the SDK. The `guard()` helper:

- **Default: fail-open.** If the check round-trip fails (network error, 5xx, retries exhausted), `guard()` returns the **original draft** so an outage on TrustLoopGuard's side doesn't take the agent down with it.
- **Fail-closed: pass an explicit `onError`.**

  ```ts
  onError: (_err, _draft) => "I'm having trouble right now — let me get a teammate."
  ```

  ```python
  on_error=lambda _err, _draft: "I'm having trouble right now — let me get a teammate."
  ```

The trade-off:

| Mode         | Availability | Safety | Use when |
|--------------|--------------|--------|----------|
| Fail-open    | Better       | Worse  | Brand-tone, soft policies. An outage shouldn't silence the agent. |
| Fail-closed  | Worse        | Better | PII, payments, regulated speech. An outage must NOT let the agent free-talk. |

A common pattern is **fail-open per-call, fail-closed per-policy**: run with the default `onError`, but mark your strict policies (`pii.*`, `payments.*`) with `on_judge_timeout: block` so that even when Tier 3 times out *server-side*, the verdict is still safe.

### LLM tier failures

Server-side. Each judge has a `deadline_ms` (default 800 ms). On timeout the engine emits a `TierResult { tier: Llm, status: Skipped }` with `reasons: ["judge_<kind>_timeout"]` and aggregates as if the judge had returned `Allow`. The policy can override this with `on_judge_timeout: { Block | Escalate | Allow }`.

When `LlmRouter` exhausts its token budget for the tenant the entire Tier 3 reports `Skipped` with `reasons: ["budget_exceeded"]`. Tiers 1 and 2 still run; their reasons still apply.

---

## Bear-trap checklist

- [ ] You registered the agent profile **before** calling `/v1/check` — unknown `agent_id` returns 400.
- [ ] `TL_RUNTIME_API_KEY` is used by the agent runtime, and `TL_ADMIN_API_KEY` is kept for policy/agent management. The server rejects requests without `Authorization: Bearer …` (except `/health`).
- [ ] You're passing `context.docs` when you have grounding to give Tier 3 — without docs, the hallucination judge will short-circuit to `Skipped`.
- [ ] Your `onBlock` and `onEscalate` are non-trivial — they're the customer-facing copy when something fired. The default `guard()` cannot pick these for you.
- [ ] If you need fail-closed, you've passed an explicit `onError` *and* you've set `on_judge_timeout: block` on the policies that need it.
- [ ] You're logging `trace_id` on your side — it's the joinable id across your logs, ours, and the `Traces` table.
