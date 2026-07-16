# Architecture

## What TrustLoopGuard is, in one sentence

A guardrail runtime that customers call **before** their AI agent's output reaches the outside world. It returns an authorization effect in milliseconds.

## The shape of one call

![TrustLoopGuard concept overview](assets/trustloop-concept.svg)

```
+-------------------+      GuardEvent         +-------------------+
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

`GuardEvent` is the public runtime surface: the normalized vocabulary for proposed outputs, tool calls, memory writes, file actions, shell commands, network requests, browser actions, database mutations, API mutations, and external messages. See [event-engine.md](event-engine.md).

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
| Runtime checks and authorization decisions | `crates/tl-server` + `crates/tl-engine` | The hot path must be shared by SDK, HTTP, and dashboard-visible traces. |
| Environments, policies, agents, traces, API keys, knowledge sources | `crates/tl-storage` | Durable guardrail data must not split between Rust and the web app. |
| Dashboard pages and browser-friendly proxy routes | `apps/web` | The web layer handles UI concerns, session context, and same-origin calls. |
| Wire contracts | `crates/tl-core` | SDKs, OpenAPI, server handlers, and storage agree on one type vocabulary. |
| Marketing and dashboard product-usage events | PostHog | Client-only product analytics are observational and never become guardrail/runtime state. See [product analytics](product-analytics.md). |

## Customer integration paths

1. **HTTP SDK** — `POST /v1/events` to a hosted server (`tl-server`). The customer uses our SDK (`tl-sdk-rust`, or generated TS/Python) and handles the returned decision in code.
2. **Gateway** — provider-compatible proxy endpoints under `/v1/gateway/*`. The customer routes AI traffic through TrustLoopGuard, and the Rust gateway applies the same composed `Decision` as `/v1/events` before returning a provider-shaped response. See [gateway.md](gateway.md).
3. **Local MCP adapter** — stdio tools in `apps/mcp-server` for agent workbenches and coding assistants. The adapter uses the TypeScript SDK and calls the same Rust `/v1/*` API for guard events, setup, runs, traces, policies, agents, and tool metadata.
4. **Embedded** — for users who want zero network hop, they pull `tl-engine` directly as a Rust dependency and call `Engine::check(&req)` in-process. Same types, no HTTP.
5. **GitHub-assisted installation** — a dashboard owner/admin connects a selected GitHub repository to an existing agent/environment and reviews a Rust-generated draft PR that adds the SDK call. It is onboarding control-plane work, not a runtime path; see [github-assisted-installation.md](github-assisted-installation.md).

All runtime paths use the **same engine contracts**. The server crate is a thin axum wrapper around the engine and Rust-owned storage.

## Event-centered check model

The runtime is SDK-first and Rust-owned. Public runtime traffic enters as `GuardEvent` through `POST /v1/events`. The server resolves workspace/environment identity, validates event bounds, resolves action metadata, resolves source labels and provenance, runs mode-gated built-in safety checkers, loads policies enabled for the resolved environment, and delegates typed family evaluation to the matching authorization adapter before composing one `Decision`. Content policy evaluation first runs deterministic literal/regex matchers; semantic candidates use the configured `semantic_policy` LLM judge route in one bounded batch. Tool policies are deterministic; shell command analysis is described in [command-safety.md](command-safety.md). See [event-engine.md](event-engine.md) for the common pipeline and trace evidence shape.

Events accept an optional, additive `session_id` inside the `GuardEvent` principal so an SDK that opted into monitoring can tag all its traffic with one monitoring session; persisted traces carry it as an indexed column and `GET /v1/traces` accepts a `session_id` query filter. The id is opaque, length-bounded metadata — never an enforcement input (see the glossary's "Monitoring session" entry).

```
GuardEvent
    │
    ▼
┌───────────────────────────────────────────┐
│ Server auth and identity resolution        │
│   workspace/environment from credentials   │
└───────────────────────────────────────────┘
    │ resolved event
    ▼
┌───────────────────────────────────────────┐
│ Event pipeline                             │
│   action metadata resolution               │
│   label + provenance resolution            │
│   mode-gated deterministic checkers        │
└───────────────────────────────────────────┘
    │ checker decision seed + evidence
    ▼
┌───────────────────────────────────────────┐
│ Policy evaluation                          │
│   enabled workspace policies               │
│   literal/regex + semantic judge           │
└───────────────────────────────────────────┘
    │ one composed decision + event evidence
    ▼
Decision {
  effect,
  reason,
  triggered_policies,
  safe_output,
  checked_input_excerpt,
  checked_output_excerpt,
  latency_ms,
  optional evidence
}
```

The event-engine seams in `tl-engine::event_pipeline` normalize, resolve principals, resolve tool metadata from the workspace registry (a cached read that fails open), attach labels, provenance, checker findings, advisory signals, compose decisions, and enqueue traces. Tool metadata resolution, label resolution, provenance propagation, deterministic checkers, and mode-aware decision composition are live. Checker enforcement is opt-in per workspace via enforcement modes (`off`/`shadow`/`enforce`, default `off`), so customer-visible behavior is unchanged until a workspace opts in; see [event-engine.md](event-engine.md) for checker rules, modes, and evidence shape.

## Request lifecycle (HTTP path)

Concrete trace of one `POST /v1/events`:

| Step | Where | What happens |
|---|---|---|
| 1 | `tl-server/src/main.rs:24` | `axum::serve` accepts the connection |
| 2 | router | path matches `/v1/events`, dispatches to the event submission handler |
| 3 | server | axum extracts `Json<GuardEvent>` and shared `AppState` |
| 4 | server | resolves workspace and environment from the runtime API key or trusted dashboard context, then loads workspace settings |
| 5 | event pipeline | overwrites event workspace/environment, resolves tool metadata, labels, and provenance |
| 6 | event pipeline | runs built-in checkers according to effective checker modes |
| 7 | server | loads policies enabled for the resolved environment; content policies evaluate on observations and typed family policies evaluate at their domain adapter boundary |
| 8 | authorization coordinator | fingerprints executable subjects, composes findings, resolves exact grants, and creates approvals, leases, and receipts when applicable |
| 9 | server | projects policy findings into one `Decision`, serializes it as JSON, and returns it over HTTP |
| 10 | (later) `tl-storage` | decision is persisted asynchronously with its environment id and event evidence |

Steps 5–8 are the **hot path**. They must be allocation-light and lock-free for the tightest (streaming) latency budget. Runtime authorization findings come from built-in safety checkers plus enabled policies loaded for the resolved environment, not hardcoded engine defaults. The common coordinator composes those findings and resolves only explicit authority requirements against matching grants. New workspaces receive disabled starter policies for common PII and prompt-injection patterns so operators can opt into them per environment. Customers with hard residency rules should redact inside their own environment before calling hosted `/v1/events`.

## Latency budget (committed)

These are the numbers we put in marketing. The architecture exists to honor them.

| Channel | Mode | p99 budget | What's allowed |
|---|---|---|---|
| Streaming chat | streaming | < 50 ms | deterministic hot path only |
| Chat | sync | < 150 ms | deterministic + fuzzy, bounded LLM only when configured |
| Email / async | sync | < 500 ms | full configured tier set |
| Replay / audit | offline | best-effort | full configured tier set and grading |

If we cannot keep these p99s with realistic policy sets, the wedge falls apart. Treat any change that risks them as a P0.

Trace persistence is deliberately fire-and-forget in service of these budgets: writes enter a bounded channel via non-blocking enqueue, and when the channel is full the trace is dropped with a warning rather than delaying the decision. The accepted consequence is that a sustained burst — including a misbehaving or compromised integration flooding `/v1/events` — can silently drop traces for its workspace while requests keep succeeding. There is no per-key rate limit today; when trace completeness gets an SLO, add a drop-rate metric/alert and per-key limiting rather than blocking the request path.

## What is explicitly NOT in v1

- **Authorization authority** — TrustLoopGuard owns database-backed approvals, scoped grants, execution leases, and receipts. External identity or signature systems may supply future attestations, but no opaque bearer proof bypasses the kernel. See [authorization-kernel.md](authorization-kernel.md).
- **Coding-agent diff review** — different product surface; defer.
- **Browser-agent execution adapters** — defer; browser actions can use the generic exact-action approval contract when the caller owns the execution hook.
- **Workflow / orchestration / agent platform** — never in scope.
- **Non-engineer policy UI** — v1 ships YAML in Git. UI is v2 once shape stabilizes.

## Dashboard-owned surfaces

Some durable surfaces are dashboard-facing only — Rust still owns them, but they don't sit on the guardrail hot path. They share the same `/v1/...` API discipline.

- **Environments** - Rust-owned deployment boundaries inside a workspace. Runtime API keys resolve one environment, policy deployment state is environment-scoped, and runs/traces/analytics carry the environment for filtering. See [environments.md](environments.md).
- **Runs** — one execution of a registered customer agent, such as a chat session, live call, workflow execution, or background job. Runs are surfaced through `/v1/runs/*` and group persisted decision traces through `traces.run_id`. Ordered run events are stored in `run_events` and can be linked from traces through `traces.run_event_id`. SDK callers may create runs explicitly; gateway model requests create a `chat_session` run automatically. They are environment-stamped observability containers only; TrustLoopGuard does not orchestrate customer agents or workflows. See [runs.md](runs.md).
- **Red-team dispatch jobs** — durable single-target attack jobs in `redteam_jobs` + `redteam_attack_sessions` + `redteam_session_events`, surfaced through `/v1/redteam/*`. Rust persists the job, each independent attack session, and ordered transcript events while an in-process worker calls a compatible private runner (`REDTEAM_RUNNER_URL`); dispatch returns a `jobId` immediately. The dashboard Attacks tab dispatches, polls, lists, and cancels. See [redteam-dispatch.md](redteam-dispatch.md).
- **Red-team harden** — synthesizes guardrail policies from a job's landed attacks and verifies each candidate before recommending it (`POST /v1/redteam/jobs/{id}/harden`). Classification + policy construction live in `tl-policy`; the endpoint, verify loop, and persistence in `tl-server`; survivors create or tighten stable harden policies, while failed attempts return rejection reasons. See [redteam-harden.md](redteam-harden.md).
- **Agent-hardening loop** — the import → plan → attack → harden → repeat cycle. Prompt-backed dashboard import creates and enables baseline guardrails before the first attack run. The attack-vector planner (`POST /v1/agents/{id}/redteam/plan`) derives attacks tailored to an agent's own definition (chat prompt and/or imported `workflow_definition`), grounded by a static workflow analyzer that finds injectable `source → sink` paths; those vectors seed the run so the attack is gray-box. Each plan is **saved** (Rust-owned, in `redteam_plans`) as a named, per-agent library entry, listed via `GET /v1/agents/{id}/redteam/plans` and removed via `DELETE /v1/redteam/plans/{id}`, so a plan can be re-selected and re-run rather than regenerated. A static path (`POST /v1/agents/{id}/redteam/static-policies`) turns unguarded paths into preventive policies for agents with no runnable target. Reuses dispatch + harden as-is. See [agent-hardening-loop.md](agent-hardening-loop.md).
- **Red-team report shares** — durable, expiring, revocable capability tokens in `redteam_report_shares` that grant public, read-only access to one vulnerability report (a completed job, optionally a same-agent comparison). Rust owns the token and computes the report payload (`GET /v1/redteam/jobs/{id}/report`, public `GET /v1/redteam/reports/{token}`); the web layer renders the PDF. See [redteam-report-sharing.md](redteam-report-sharing.md).
- **Custom analytics dashboards** — Rust-computed analytics queries and saved workspace dashboard views, surfaced through `/v1/analytics/catalog`, `/v1/analytics/query`, and `/v1/analytics/views/*`. The web dashboard may provide Datadog-style filters and widget controls, but saved views and query semantics are Rust-owned. See [analytics-dashboards.md](analytics-dashboards.md).
- **Human review analytics** — append-only `human_review_events` linked to persisted traces, surfaced through `/v1/traces/{trace_id}/review-events` and `/v1/analytics/human-review`. They record customer review outcomes for monitoring and audit without turning TrustLoopGuard into a review queue. See [human-review-analytics.md](human-review-analytics.md).
- **Financial authorization and execution** — typed financial actions delegate authorization to the common kernel while financial code retains ledger admission, reservations, provider execution, execution receipts, outcomes, and reversals. Rust projects those evidence, authorization, and execution axes into the dashboard-facing `state` and `state_reason`; the underlying `authorization_effect`, `authorization_status`, and `execution_status` remain separate. See [financial-authorization.md](financial-authorization.md) and [authorization-kernel.md](authorization-kernel.md).
- **Workspace policies** — policy authoring, listing, editing, delete, and enablement changes are Rust-owned through `/v1/policies/*`. Policy definitions are workspace-level, carry a `family` discriminator, and use environment-scoped deployment state for runtime enablement. Runtime checks only load policies enabled in the resolved environment. Workspace creation seeds disabled starter policies that users can enable, edit, or delete like any other policy. See [policies.md](policies.md).
- **Shell command safety** — `family: tool` policies can match deterministic facts or exact parameter paths for `shell.action.proposed` events. The Tool adapter emits common findings and exact approval requirements; no web route or command-specific endpoint evaluates commands. See [command-safety.md](command-safety.md).
- **Tool metadata registry** — workspace-scoped tool semantics in `tool_metadata`, surfaced via `/v1/tool-metadata` and `/v1/tool-metadata/{tool}`. The event pipeline reads the same registry for action resolution. See [event-engine.md](event-engine.md).
- **Source label policies** — workspace-scoped per-origin label overrides are `family: source_label` policies in the unified registry. `/v1/label-policies` remains an ergonomic compatibility wrapper, while the event pipeline reads the same registry-backed label policy state for label resolution. See [event-engine.md](event-engine.md) and [policies.md](policies.md).
- **Workspace runtime settings + checker rollout** — `workspace_settings` (read via `GET /v1/settings`, partially updated via `PATCH /v1/settings`) carries per-workspace checker enforcement modes; `environment_checker_modes` (surfaced via `GET`/`PUT /v1/environments/{environment_id}/checker-modes`) carries per-environment overrides where `NULL` inherits the workspace mode. The event pipeline resolves effective modes per request from both. See [event-engine.md](event-engine.md).
- **Workspace team, invites, and dashboard feature rollout** — `workspaces` carries default-off dashboard availability flags, while `workspace_members` and `workspace_invites` carry access. `GET /v1/team/my-workspaces` returns both membership and feature availability so the web dashboard can hide and reject unreleased workspace features. See [team-and-invites.md](team-and-invites.md).
- **Workspace API keys** — `workspace_api_keys`, surfaced via `GET /v1/api-keys`, `POST /v1/api-keys`, and `PATCH /v1/api-keys/batch/revoke`. Runtime SDK and gateway model requests send these as `Authorization: Bearer tl_live_...`; the middleware resolves the workspace and environment from storage. See [authorization.md](authorization.md#workspace-api-keys).
- **Gateway configuration, usage, and spend admission** — provider connections, gateway routes,
  provider edit/permanent-delete lifecycle, precise customer/guardrail usage events, price
  snapshots, meter-scoped budget alerts, and durable per-principal budget reservations are
  Rust-owned through `/v1/*` and Rust storage. A provider referenced by a route cannot be deleted.
  Routes bind a provider and agent; enabled environment/agent policies apply automatically.
  `meter: llm_usage` policies remain the spending-cap source of truth, while run events provide
  correlated audit evidence. Bounded calls receive hard preflight admission; unbounded calls use
  the soft-cap behavior defined in [gateway.md](gateway.md).
  Runtime keys may use gateway model endpoints but cannot manage this configuration. Gateway model
  traffic terminates in Rust, not the web app. See [gateway.md](gateway.md).
- **OAuth identity links** — `oauth_identities`, surfaced through `POST /v1/identity/oauth-session`. Google/GitHub authenticate the browser user; Rust maps the provider account to one local `users.id` before workspace membership checks run. See [authorization.md](authorization.md#oauth-users-google--github).
- **User approval** — `users.is_approved` gates authenticated dashboard access in every environment. See [authorization.md](authorization.md#three-lanes-one-middleware).

## End-state to keep in mind

The repo is built so any of these can be added without re-architecting:

- A second binary (e.g. `tl-edge`) that embeds `tl-engine` as a sidecar with no HTTP.
- A gRPC interface (just a new transport over the same engine).
- Postgres / ClickHouse decision logs (swap the `DecisionStore` impl, no engine change).
- Provider integrations (LiveKit, Pipecat, OpenAI middleware) — each is a new example crate, not a core change.

The crate boundaries (see [crates.md](crates.md)) exist precisely so these additions are mechanical.
