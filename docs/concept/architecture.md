# Architecture

> **v0 readers:** the layered "short-circuit on first hard block" model described below was the original v1 sketch. The runtime that actually ships in v0 runs all three tiers **in parallel with cancellation** — see [`v0-design-decisions.md` §4](v0-design-decisions.md) for the parallel-cancel orchestrator, the `HandlerCtx` shape, the `LlmRouter`, and the cache/storage/escalation wiring. Use this document for the high-level shape and integration story; use the design-decisions doc for what actually runs.

## What TrustLoopGuard is, in one sentence

A guardrail runtime that customers call **before** their AI agent's output reaches the outside world. It returns a verdict in milliseconds.

## The shape of one call

```
+-------------------+      CheckRequest       +-------------------+
|  Customer's       |  ────────────────────►  |  TrustLoopGuard   |
|  agent loop       |                         |  (tl-server)      |
|                   |                         |                   |
|  proposed_output  |  ◄────────────────────  |    Decision       |
+-------------------+      Decision           +-------------------+
                                                       │
                                                       ▼
                                              decision log (tl-storage)
```

The customer's agent does not stop being smart. TrustLoopGuard is a **gate**, not a brain. It says "this output is fine" or "this output is dangerous, here's a safer one."

## Two ways customers integrate

There is no third option in v1.

1. **HTTP SDK** — `POST /v1/check` to a hosted server (`tl-server`). The customer uses our SDK (`tl-sdk-rust`, or generated TS/Python) and handles the returned decision in code.
2. **Gateway** — provider-compatible proxy endpoints under `/v1/gateway/*`. The customer routes AI traffic through TrustLoopGuard, and the Rust gateway applies dashboard-managed enforcement behavior before returning a provider-shaped response. See [gateway.md](gateway.md).
3. **Embedded** — for users who want zero network hop, they pull `tl-engine` directly as a Rust dependency and call `Engine::check(&req)` in-process. Same types, no HTTP.

Both paths run the **same engine code**. The server crate is a thin axum wrapper around the engine.

## Layered model: input to verdict

Every check goes through these layers, in order, and short-circuits on the first hard block.

```
CheckRequest
    │
    ▼
┌───────────────────────────────────────────┐
│ Layer 1: Static matchers                  │  ← microseconds
│   regex, literal (Aho-Corasick), PII      │
└───────────────────────────────────────────┘
    │ no hard block?
    ▼
┌───────────────────────────────────────────┐
│ Layer 2: Local classifiers (later)        │  ← 5-20 ms
│   small models via ONNX / candle          │
└───────────────────────────────────────────┘
    │ no hard block?
    ▼
┌───────────────────────────────────────────┐
│ Layer 3: Remote LLM judge (opt-in)        │  ← 50-300 ms, deadline-bound
│   only when a policy explicitly opts in   │
└───────────────────────────────────────────┘
    │
    ▼
Decision { verdict, reason, triggered_policies, safe_output, latency_ms }
```

**Layer 1 is the moat.** Voice-channel checks must finish in Layer 1. Layers 2 and 3 are off-path for voice unless the policy author accepts the latency cost.

## Request lifecycle (HTTP path)

Concrete trace of one `POST /v1/check`:

| Step | Where | What happens |
|---|---|---|
| 1 | `tl-server/src/main.rs:24` | `axum::serve` accepts the connection |
| 2 | router | path matches `/v1/check`, dispatches to `check_handler` |
| 3 | `tl-server/src/main.rs:11` | axum extracts `Json<CheckRequest>` and shared `AppState` |
| 4 | `tl-engine/src/lib.rs:24` | `Engine::check(&req)` runs |
| 5 | `tl-engine/src/engine_match.rs` | each policy's matchers run against `proposed_output` |
| 6 | engine | first triggered policy's `Action` becomes the `Verdict` |
| 7 | server | `Decision` is serialized as JSON, returned over HTTP |
| 8 | (later) `tl-storage` | decision is persisted asynchronously |

Steps 4–6 are the **hot path**. They must be allocation-light and lock-free for the voice latency budget.

## Latency budget (committed)

These are the numbers we put in marketing. The architecture exists to honor them.

| Channel | Mode | p99 budget | What's allowed |
|---|---|---|---|
| Voice | streaming | < 50 ms | Layer 1 only |
| Chat | sync | < 150 ms | Layer 1 + Layer 2, optional fast Layer 3 |
| Email / async | sync | < 500 ms | All layers |
| Replay / audit | offline | best-effort | All layers, full LLM grading |

If we cannot keep these p99s with realistic policy sets, the wedge falls apart. Treat any change that risks them as a P0.

## What is explicitly NOT in v1

- **Tool/permission/credential layer** — Clawvisor's territory. We interoperate, we don't compete.
- **Coding-agent diff review** — different product surface; defer.
- **Browser-agent action approval** — defer.
- **Workflow / orchestration / agent platform** — never in scope.
- **Non-engineer policy UI** — v1 ships YAML in Git. UI is v2 once shape stabilizes.

## Dashboard-owned surfaces

Some durable surfaces are dashboard-facing only — Rust still owns them, but they don't sit on the guardrail hot path. They share the same `/v1/...` API discipline.

- **Runs** — one execution of a registered customer agent, such as a chat session, live call, workflow execution, or background job. Runs are surfaced through `/v1/runs/*` and group persisted decision traces through `traces.run_id`. Ordered run events are stored in `run_events` and can be linked from traces through `traces.run_event_id`. They are observability containers only; TrustLoopGuard does not orchestrate customer agents or workflows. See [runs.md](runs.md).
- **Workspace policies** — policy authoring, listing, editing, delete, and enablement changes are Rust-owned through `/v1/policies/*`. The dashboard may batch-enable or batch-disable policies through `PATCH /v1/policies/batch/enabled`; runtime checks only load enabled policies.
- **Workspace team + invites** — `workspace_members` and `workspace_invites`, surfaced via `/v1/team/*`. See [team-and-invites.md](team-and-invites.md).
- **Workspace API keys** — `workspace_api_keys`, surfaced via `GET /v1/api-keys`, `POST /v1/api-keys`, and `PATCH /v1/api-keys/batch/revoke`. Runtime SDK requests send these as `Authorization: Bearer tl_live_...`; the middleware resolves the workspace from storage. See [authorization.md](authorization.md#workspace-api-keys).
- **Gateway configuration** — provider connections, gateway routes, and enforcement profiles are Rust-owned through `/v1/gateway/*` and `/v1/enforcement-profiles`. Gateway model traffic also terminates in Rust, not the web app. See [gateway.md](gateway.md).

## End-state to keep in mind

The repo is built so any of these can be added without re-architecting:

- A second binary (e.g. `tl-edge`) that embeds `tl-engine` as a sidecar with no HTTP.
- A gRPC interface (just a new transport over the same engine).
- Postgres / ClickHouse decision logs (swap the `DecisionStore` impl, no engine change).
- Provider integrations (LiveKit, Pipecat, OpenAI middleware) — each is a new example crate, not a core change.

The crate boundaries (see [crates.md](crates.md)) exist precisely so these additions are mechanical.
