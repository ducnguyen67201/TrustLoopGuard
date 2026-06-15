# Agent Breakaway Arena

The Agent Breakaway Arena is the raw-vs-guarded comparison concept: the same adversarial chat
prompts are sent to a raw agent and a TrustLoopGuard-protected agent, and the difference in what
gets through is the before/after.

> The standalone Arena **dashboard page was removed** — `/arena` now redirects to the **Attacks**
> tab. What remains is described below: the raw-vs-guarded comparison concept and the **agent
> adapter contract** that the Attacks setup depends on.

The durable, single-target surface is the **Attacks** tab (`/attacks`): instead of a raw-vs-guarded
pair, you paste one agent endpoint URL and attack just that target, reporting what got through. The
Attacks tab dispatches a Rust-owned job (`/v1/redteam/*`) that persists the job and per-attack
results, so you can leave and come back to history. It shares the same attack runner and loopback
allowlist, but the runner is driven by the Rust orchestrator, not the browser. See
[redteam-dispatch.md](redteam-dispatch.md).

Durable raw-vs-guarded comparisons belong to
[TrustLoopGuardBench](trustloopguard-bench.md), not the arena. Benchmark runs
persist a Rust-owned parent record, map raw and guarded arms to child red-team
jobs, and compute the report in Rust.

## Adapter Contract

A target the Attacks tab can drive exposes two endpoints:

- `GET /arena/profile`
- `POST /arena/chat`

You implement these two endpoints on your own agent, then point the red-team runner (via the
Attacks tab) at the adapter URL. Exposing this as a one-function SDK helper that wraps an existing
agent is on the roadmap.

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

This is intentionally different from the SDK `/v1/events` decision verdicts, which are
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

SDK mode is different. An SDK-integrated agent submits a `GuardEvent` to `/v1/events` and receives a `Decision`:

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

The standalone Arena page that ran a raw-vs-guarded pair in the browser is gone. The durable way to
exercise the comparison is the **Attacks tab** (`/attacks`): a Rust-owned job drives the compatible
private runner (`POST /redteam/jobs`, `REDTEAM_RUNNER_URL`) and persists per-attack results. See
[redteam-dispatch.md](redteam-dispatch.md).

```text
Attacks tab -> Rust orchestrator -> runner -> POST /arena/chat -> agent adapter
            -> (guarded) TrustLoopGuard gateway -> provider
```

The guarded path is unchanged from gateway mode: the guarded adapter calls the TrustLoopGuard
gateway, which applies policy and returns `verdict`/`phase`/`traceId` as described above.

## Hardening Loop

A finished report can be turned into a guard policy and the campaign re-run — the
attack -> break -> harden -> repeat loop. When at least one non-control attack still
lands on the guarded side, the dashboard can ask Rust to synthesize and verify
candidate policies for that job. The web app only calls the same-origin wrapper
in `apps/web/lib/redteam-harden.ts`; synthesis, verification, and optional policy
persistence are Rust-owned. See [redteam-harden.md](redteam-harden.md).

## Ownership Boundary

The compatible red-team runner is an attack harness: it generates adversarial prompts and judges
replies. It owns no policies, decisions, traces, or any other durable product data, so it sits
outside the Rust source-of-truth boundary. It is configured with `REDTEAM_RUNNER_URL`.

The durable **Attacks tab** does own its job and per-attack results in Rust (via the dispatch jobs
in [redteam-dispatch.md](redteam-dispatch.md)), but the runner is still a stateless executor that
Rust calls — the guard runtime never owns adversarial prompt generation itself. The Attacks tab's
`apps/web/app/api/redteam/*` routes proxy to the Rust orchestrator, which calls the runner and
enforces the loopback agent-target allowlist (`127.0.0.1`, `localhost`, `::1` — deny-by-default).

The one place a run touches the product backend is the guarded target itself: the guarded adapter
calls the real TrustLoopGuard gateway, which evaluates policy and persists traces in Rust exactly
as it would for any other traffic. The comparison reads nothing back from those traces; it only
surfaces the trace IDs returned in adapter replies.
