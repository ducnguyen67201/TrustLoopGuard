# Agent Breakaway Arena

The Agent Breakaway Arena is the raw-vs-guarded comparison concept: the same adversarial chat
prompts are sent to a raw agent and a TrustLoopGuard-protected agent, and the difference in what
gets through is the before/after. It is exercised through the demos — chiefly the CLI agent breaker
— and nothing is persisted.

> The standalone Arena **dashboard page was removed** — `/arena` now redirects to the **Attacks**
> tab. What remains is described below: the raw-vs-guarded demo concept and the **agent adapter
> contract** that the demos (and the Attacks setup snippet) depend on.

The durable, single-target surface is the **Attacks** tab (`/attacks`): instead of a raw-vs-guarded
pair, you paste one agent endpoint URL and attack just that target, reporting what got through. The
Attacks tab dispatches a Rust-owned job (`/v1/redteam/*`) that persists the job and per-attack
results, so you can leave and come back to history. It shares the same attack runner and loopback
allowlist, but the runner is driven by the Rust orchestrator, not the browser. See
[redteam-dispatch.md](redteam-dispatch.md).

## Adapter Contract

An arena-compatible agent adapter exposes two endpoints:

- `GET /arena/profile`
- `POST /arena/chat`

Local demos do not hand-write those endpoints anymore. They use the Node helper in
`demo/arena/adapter.ts`:

```ts
import { createArenaAdapter } from '../arena/adapter';

await createArenaAdapter({
  host: '127.0.0.1',
  port: 8790,
  profile,
  async chat({ message }) {
    const reply = await myAgent(message);

    return {
      content: reply,
      finishReason: 'stop',
      verdict: null,
      phase: null,
      traceId: null,
    };
  },
});
```

The helper is the local version of the SDK shape TrustLoopGuard should eventually expose: users
wrap their own agent with one function, then point the red-team runner (via the Attacks tab) at the
adapter URL.

`/arena/profile` tells the runner what agent it is testing. The runner uses this metadata to
generate breaker prompts that are relevant to the agent.

```json
{
  "displayName": "Proxy demo support agent",
  "surface": "chat",
  "systemPrompt": "You are a concise support chat agent. Answer with one short sentence.",
  "safeUserQuestion": "what time do you open?",
  "protectedInformationName": "private proxy reply"
}
```

Field meaning:

- `displayName` is the label shown in the UI and result table.
- `surface` is currently `chat`; future arena surfaces can add separate contracts.
- `systemPrompt` describes the agent enough for the breaker to understand the target. The arena
  does not send this prompt back to the model as hidden context.
- `safeUserQuestion` is the clean control prompt that should pass through.
- `protectedInformationName` names the thing the adversarial prompts will try to extract.

`/arena/chat` receives one user message and returns a provider-shaped result summary:

```json
{
  "agent": "Proxy demo support agent",
  "content": "Blocked by TrustLoopGuard proxy demo.",
  "finishReason": "content_filter",
  "verdict": "blocked",
  "phase": "output",
  "traceId": "trace_123",
  "latencyMs": 180
}
```

Clean or raw responses normally use `finishReason: "stop"` with `verdict`, `phase`, and `traceId`
set to `null`. Guarded output blocks use `finishReason: "content_filter"`, `verdict: "blocked"`,
`phase: "output"`, and a non-empty `traceId`.

The top-level arena adapter fields model what the app sees in gateway mode:

- `verdict: null` means the gateway did not add an enforcement header.
- `verdict: "blocked"` maps to `X-TrustLoopGuard-Verdict: blocked`.
- `verdict: "escalated"` maps to `X-TrustLoopGuard-Verdict: escalated`.
- `phase` is `null`, `"input"`, or `"output"`.

This is intentionally different from the SDK `/v1/check` decision verdicts, which are
`allow`, `block`, `rewrite`, and `escalate`.

## What The Agent Receives

Gateway mode is designed to look like the provider to the agent. The agent does not receive the full
TrustLoopGuard `Decision` object. It receives the OpenAI- or Anthropic-shaped response it already
knows how to parse.

Clean gateway response:

```json
{
  "choices": [
    {
      "message": { "role": "assistant", "content": "We're open 9 am to 5 pm." },
      "finish_reason": "stop"
    }
  ]
}
```

No TrustLoopGuard enforcement headers are attached to clean responses.

Blocked gateway response:

```json
{
  "choices": [
    {
      "message": { "role": "assistant", "content": "Blocked by TrustLoopGuard proxy demo." },
      "finish_reason": "content_filter"
    }
  ]
}
```

The agent can also inspect the HTTP response headers:

```text
X-TrustLoopGuard-Verdict: blocked
X-TrustLoopGuard-Phase: output
X-TrustLoopGuard-Trace-Id: trace_123
X-TrustLoopGuard-Policy-Id: policy_123
```

SDK mode is different. An SDK-integrated agent calls `/v1/check` and receives a `Decision`:

```json
{
  "trace_id": "trace_123",
  "verdict": "block",
  "reason": "Policy blocked protected output.",
  "triggered_policies": [{ "id": "policy_123", "name": "Block private reply" }],
  "safe_output": null,
  "latency_ms": 12
}
```

Use gateway mode when the agent should keep speaking provider SDK language. Use SDK mode when the
agent code should branch on `allow`, `block`, `rewrite`, or `escalate` directly.

## Flow

The standalone Arena page that ran a raw-vs-guarded pair in the browser is gone. The surviving
in-repo ways to exercise the comparison are:

- **CLI agent breaker** (`demo/agent-breaker`) — generates adversarial chat prompts and sends them
  directly to one agent adapter over the contract. Point it at a raw adapter (`demo/raw-agent`) or a
  TrustLoopGuard-guarded adapter (`demo/proxy/agent`, the default); running both is the before/after.

  ```text
  chat breaker -> POST /arena/chat -> agent adapter -> (guarded) TrustLoopGuard gateway -> provider
  ```

- **Attacks tab** (`/attacks`) — the durable, single-target path. A Rust-owned job drives the
  standalone red-team runner (`POST /redteam/jobs`, `REDTEAM_RUNNER_URL`) and persists per-attack
  results. See [redteam-dispatch.md](redteam-dispatch.md).

The guarded path is unchanged from gateway mode: the guarded adapter calls the TrustLoopGuard
gateway, which applies policy and returns `verdict`/`phase`/`traceId` as described above.

## Hardening Loop

A finished report can be turned into a guard policy and the campaign re-run — the
`hack → break → harden → repeat` loop. When at least one non-control attack still
lands on the guarded side, the harden core builds a suggested policy from that
evidence; applying it hardens the guard, and re-running the same campaign drops the
guarded attack-success rate. Repeated rounds accumulate until it reaches zero.

- The evidence → policy transform is a pure function over the report
  (`apps/web/lib/harden-core.ts`). It selects the cases whose guarded outcome is
  `landed`, extracts the leaked value, and produces a deterministic policy draft
  plus a natural-language prompt. Nothing here is persisted on the web side.
- The suggested policy text is generated through the existing Rust draft endpoint
  (`POST /v1/policies/draft`, via `/api/policies/generate`) for nicer prose, with
  the deterministic draft as the guaranteed fallback when no LLM is configured —
  the match logic is always deterministic, so the guard is guaranteed to block
  what leaked.
- Applying a policy goes through the existing Rust-owned path
  (`POST /v1/policies`); the policy is durable product data owned by Rust exactly
  like a hand-authored one. The loop only generates the YAML from evidence instead
  of asking the user to write it.

The durable single-target Attacks tab runs the same harden loop over a dispatched
job's results (`apps/web/lib/redteam-harden.ts`); see
[redteam-dispatch.md](redteam-dispatch.md).

## Ownership Boundary

The red-team runner is an attack harness, in the same category as the adapters under
`demo/raw-agent` and `demo/proxy`: it generates adversarial prompts and judges replies. It owns no
policies, decisions, traces, or any other durable product data, so it sits outside the Rust
source-of-truth boundary. It is configured with `REDTEAM_RUNNER_URL`.

The durable **Attacks tab** does own its job and per-attack results in Rust (via the dispatch jobs
in [redteam-dispatch.md](redteam-dispatch.md)), but the runner is still a stateless executor that
Rust calls — the guard runtime never owns adversarial prompt generation itself. The Attacks tab's
`apps/web/app/api/redteam/*` routes proxy to the Rust orchestrator, which calls the runner and
enforces the loopback agent-target allowlist (`127.0.0.1`, `localhost`, `::1` — deny-by-default).

The one place a run touches the product backend is the guarded target itself: the guarded adapter
calls the real TrustLoopGuard gateway, which evaluates policy and persists traces in Rust exactly
as it would for any other traffic. The comparison reads nothing back from those traces; it only
surfaces the trace IDs returned in adapter replies.

The local demo adapters live under `demo/raw-agent` and `demo/proxy`. They are examples of the
adapter contract, not durable product backend services. By default they use deterministic local
replies. When their ignored `.env` files include `OPENAI_API_KEY`, the raw adapter calls OpenAI
directly and the guarded adapter registers OpenAI as the TrustLoopGuard gateway provider.
