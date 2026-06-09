# Architecture

## What TrustLoopGuard is, in one sentence

A guardrail runtime that customers call **before** their AI agent's output reaches the outside world. It returns a verdict in milliseconds.

## The shape of one call

![TrustLoopGuard concept overview](assets/trustloop-concept.svg)

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

The customer's agent does not stop being smart. TrustLoopGuard is a **gate**, not a brain. It says "this proposed output or action is fine" or "this is dangerous, here's a safer path."

`CheckRequest` remains the public `/v1/check` compatibility surface. The SDK-first engine contract underneath it is `GuardEvent`, the normalized vocabulary for proposed outputs, tool calls, memory writes, file actions, shell commands, network requests, browser actions, database mutations, API mutations, and external messages. See [event-engine.md](event-engine.md).

## Runtime data flow

![Runtime data flow](assets/runtime-data-flow.svg)

Runtime checks do not pass through the dashboard. Customer applications call
the Rust API through one of the SDKs. The dashboard calls same-origin Next.js
API routes only so browser code gets authentication, workspace resolution, and
camelCase/snake_case translation in one place. Those routes still proxy to
Rust; they do not own runtime guardrail state.

That boundary keeps one source of truth:

| Data or behavior | Owner | Why |
|---|---|---|
| Runtime checks and verdicts | `crates/tl-server` + `crates/tl-engine` | The hot path must be shared by SDK, HTTP, and dashboard-visible traces. |
| Environments, policies, agents, traces, API keys, knowledge sources | `crates/tl-storage` | Durable guardrail data must not split between Rust and the web app. |
| Dashboard pages and browser-friendly proxy routes | `apps/web` | The web layer handles UI concerns, session context, and same-origin calls. |
| Wire contracts | `crates/tl-core` | SDKs, OpenAPI, server handlers, and storage agree on one type vocabulary. |

## Customer integration paths

1. **HTTP SDK** — `POST /v1/check` to a hosted server (`tl-server`). The customer uses our SDK (`tl-sdk-rust`, or generated TS/Python) and handles the returned decision in code.
2. **Gateway** — provider-compatible proxy endpoints under `/v1/gateway/*`. The customer routes AI traffic through TrustLoopGuard, and the Rust gateway applies dashboard-managed enforcement behavior before returning a provider-shaped response. See [gateway.md](gateway.md).
3. **Embedded** — for users who want zero network hop, they pull `tl-engine` directly as a Rust dependency and call `Engine::check(&req)` in-process. Same types, no HTTP.

All runtime paths use the **same engine contracts**. The server crate is a thin axum wrapper around the engine and Rust-owned storage.

## Event-centered check model

The runtime is SDK-first and Rust-owned. Today, public `/v1/check` requests still enter as `CheckRequest` for compatibility, then run through the existing parallel tier orchestrator. The event-engine foundation maps that same request into `GuardEvent { kind: output.proposed, ... }` through a no-op compatibility adapter so SDK, gateway, and adapter work can converge on one event vocabulary without changing current verdict behavior.

```
CheckRequest
    │
    ▼
┌───────────────────────────────────────────┐
│ Server redaction stage                    │
│   optional defense-in-depth sanitization  │
└───────────────────────────────────────────┘
    │ sanitized request
    ▼
┌───────────────────────────────────────────┐
│ Legacy event normalizer                    │
│   CheckRequest -> output.proposed event    │
│   available as an engine seam              │
└───────────────────────────────────────────┘
    │ compatibility event
    ▼
┌───────────────────────────────────────────┐
│ Existing tier orchestrator                 │
│   deterministic + fuzzy + LLM tiers        │
│   parallel with cancellation               │
└───────────────────────────────────────────┘
    │ first hard block, timeout escalation,
    │ or all tiers clear
    ▼
Decision {
  verdict,
  reason,
  triggered_policies,
  safe_output,
  checked_input_excerpt,
  checked_output_excerpt,
  latency_ms,
  redaction,
  optional evidence
}
```

The event-engine seams in `tl-engine::event_pipeline` are no-op by default: they normalize, resolve principals, attach tool metadata, labels, provenance, checker findings, advisory signals, compose decisions, and enqueue traces without adding writes, network calls, or customer-visible behavior. The current request still returns the same `Decision` shape unless optional evidence is deliberately populated.

## Request lifecycle (HTTP path)

Concrete trace of one `POST /v1/check`:

| Step | Where | What happens |
|---|---|---|
| 1 | `tl-server/src/main.rs:24` | `axum::serve` accepts the connection |
| 2 | router | path matches `/v1/check`, dispatches to `check_handler` |
| 3 | `tl-server/src/main.rs:11` | axum extracts `Json<CheckRequest>` and shared `AppState` |
| 4 | server | resolves workspace and environment from the runtime API key or trusted dashboard context, then loads workspace settings |
| 5 | server | when `CheckRequest.redaction.mode = server`, redacts `input`, `proposed_output`, configured context strings, and inline run-event summaries before engine/cache/trace paths |
| 6 | `tl-engine/src/lib.rs` | `Engine::check_async_with_policies(&req, ...)` runs against policies enabled for the resolved environment |
| 7 | `tl-engine/src/pipeline/` | deterministic, fuzzy, and LLM tiers run through the parallel-cancel orchestrator |
| 8 | engine | the first hard block wins; an LLM timeout can escalate; otherwise the request is allowed |
| 9 | server | `Decision` is serialized as JSON, returned over HTTP |
| 10 | (later) `tl-storage` | decision is persisted asynchronously with its environment id |

Steps 5–8 are the **hot path**. They must be allocation-light and lock-free for the voice latency budget. Runtime guardrail verdicts come from enabled policies loaded for the resolved environment, not hardcoded engine defaults. New workspaces receive disabled starter policies for common PII and prompt-injection patterns so operators can opt into them per environment. Hosted server redaction is defense in depth; customers with hard residency rules should redact in the SDK or inside their own environment before calling hosted `/v1/check`.

## Latency budget (committed)

These are the numbers we put in marketing. The architecture exists to honor them.

| Channel | Mode | p99 budget | What's allowed |
|---|---|---|---|
| Voice | streaming | < 50 ms | deterministic hot path only |
| Chat | sync | < 150 ms | deterministic + fuzzy, bounded LLM only when configured |
| Email / async | sync | < 500 ms | full configured tier set |
| Replay / audit | offline | best-effort | full configured tier set and grading |

If we cannot keep these p99s with realistic policy sets, the wedge falls apart. Treat any change that risks them as a P0.

## What is explicitly NOT in v1

- **Tool/permission/credential layer** — Clawvisor's territory. We interoperate, we don't compete.
- **Coding-agent diff review** — different product surface; defer.
- **Browser-agent action approval** — defer.
- **Workflow / orchestration / agent platform** — never in scope.
- **Non-engineer policy UI** — v1 ships YAML in Git. UI is v2 once shape stabilizes.

## Dashboard-owned surfaces

Some durable surfaces are dashboard-facing only — Rust still owns them, but they don't sit on the guardrail hot path. They share the same `/v1/...` API discipline.

- **Environments** - Rust-owned deployment boundaries inside a workspace. Runtime API keys resolve one environment, policy deployment state is environment-scoped, and runs/traces/analytics carry the environment for filtering. See [environments.md](environments.md).
- **Runs** — one execution of a registered customer agent, such as a chat session, live call, workflow execution, or background job. Runs are surfaced through `/v1/runs/*` and group persisted decision traces through `traces.run_id`. Ordered run events are stored in `run_events` and can be linked from traces through `traces.run_event_id`. SDK callers may create runs explicitly; gateway model requests create a `chat_session` run automatically. They are environment-stamped observability containers only; TrustLoopGuard does not orchestrate customer agents or workflows. See [runs.md](runs.md).
- **Custom analytics dashboards** — Rust-computed analytics queries and saved workspace dashboard views, surfaced through `/v1/analytics/catalog`, `/v1/analytics/query`, and `/v1/analytics/views/*`. The web dashboard may provide Datadog-style filters and widget controls, but saved views and query semantics are Rust-owned. See [analytics-dashboards.md](analytics-dashboards.md).
- **Human review analytics** — append-only `human_review_events` linked to persisted traces, surfaced through `/v1/traces/{trace_id}/review-events` and `/v1/analytics/human-review`. They record customer review outcomes for monitoring and audit without turning TrustLoopGuard into a review queue. See [human-review-analytics.md](human-review-analytics.md).
- **Workspace policies** — policy authoring, listing, editing, delete, and enablement changes are Rust-owned through `/v1/policies/*`. Policy definitions are workspace-level, while enablement is stored as environment-scoped policy deployment state. Runtime checks only load policies enabled in the resolved environment. Workspace creation seeds disabled starter policies that users can enable, edit, or delete like any other policy.
- **Workspace team + invites** — `workspace_members` and `workspace_invites`, surfaced via `/v1/team/*`. See [team-and-invites.md](team-and-invites.md).
- **Workspace API keys** — `workspace_api_keys`, surfaced via `GET /v1/api-keys`, `POST /v1/api-keys`, and `PATCH /v1/api-keys/batch/revoke`. Runtime SDK and gateway model requests send these as `Authorization: Bearer tl_live_...`; the middleware resolves the workspace and environment from storage. See [authorization.md](authorization.md#workspace-api-keys).
- **Gateway configuration** — provider connections, gateway routes, and enforcement profiles are Rust-owned through `/v1/gateway/*` and `/v1/enforcement-profiles`. Runtime keys may use gateway model endpoints but cannot manage this configuration. Gateway model traffic also terminates in Rust, not the web app. See [gateway.md](gateway.md).
- **OAuth identity links** — `oauth_identities`, surfaced through `POST /v1/identity/oauth-session`. Google/GitHub authenticate the browser user; Rust maps the provider account to one local `users.id` before workspace membership checks run. See [authorization.md](authorization.md#oauth-users-google--github).
- **Hosted user approval** — `users.is_approved` gates TrustLoopGuard-operated staging and production dashboard access when `TL_HOSTED_DEPLOYMENT=true`. Self-hosted deployments leave that hosted flag unset. See [authorization.md](authorization.md#three-lanes-one-middleware).

## End-state to keep in mind

The repo is built so any of these can be added without re-architecting:

- A second binary (e.g. `tl-edge`) that embeds `tl-engine` as a sidecar with no HTTP.
- A gRPC interface (just a new transport over the same engine).
- Postgres / ClickHouse decision logs (swap the `DecisionStore` impl, no engine change).
- Provider integrations (LiveKit, Pipecat, OpenAI middleware) — each is a new example crate, not a core change.

The crate boundaries (see [crates.md](crates.md)) exist precisely so these additions are mechanical.
