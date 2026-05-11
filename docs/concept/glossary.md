# Glossary

Every domain term defined once. If you find yourself explaining a term in a PR review, add it here instead.

---

## Domain terms

### Agent

An AI program that takes actions or produces outputs on behalf of a customer's product. Examples: customer-support chatbot, sales voice agent, internal IT helper, coding agent. TrustLoopGuard does not run the agent — it sits in the agent's output path.

### Agent profile

A YAML or JSON document registered once per agent (via `POST /v1/agents`) and referenced by `agent_id` on every check. Carries `scope` (`in_scope` / `out_of_scope`), `authority` (`can_promise` / `cannot_promise`), `tone` (target + forbidden), and pointers to `knowledge_sources`. Tier 3 LLM judges read this profile to know what the agent is *permitted* to claim — see `crates/tl-llm/src/prompts/`. Without a profile, Tier 3 reports `Skipped` (no grounding context).

### Channel

The medium an agent is operating on: `voice`, `chat`, `email`, `other("...")`. Channel drives the latency budget and which matchers are eligible. Voice has the strictest budget; email the loosest.

### CheckRequest

What a customer sends to TrustLoopGuard for a single decision. Contains:
- `agent_id` — which of the customer's agents this came from
- `channel` — voice, chat, etc.
- `input` — what the user said to the agent (context for the matchers)
- `proposed_output` — what the agent **wants** to say or do, before TrustLoopGuard sees it
- `policies` — optional policy ID list to scope evaluation
- `context` — free-form JSON the customer attaches (user tier, session id, etc.)
- `trace_id` — optional caller-supplied id for correlation

### Decision

What TrustLoopGuard returns. The ground truth of a check.
- `trace_id` — set by caller or generated
- `verdict` — `Allow | Block | Rewrite | Escalate`
- `reason` — human-readable summary
- `triggered_policies` — list of every policy that fired
- `safe_output` — present when `verdict = Rewrite`; the suggested replacement
- `latency_ms` — wall-clock time the engine spent

### Verdict

The four outcomes. Only ever one per `Decision`.

| Verdict | Meaning | What customer should do |
|---|---|---|
| `Allow` | No policy triggered. Output is safe to ship. | Send `proposed_output` as-is. |
| `Block` | At least one critical policy fired and there's no safe rewrite. | Suppress the output. Tell the user something neutral or escalate. |
| `Rewrite` | A policy fired and provided `safe_output`. | Send `safe_output` instead of `proposed_output`. |
| `Escalate` | Policy says "human in the loop." | Hand off to a human. Don't auto-send anything. |

### Severity

How bad a triggered policy is: `Low`, `Medium`, `High`, `Critical`. Used for sorting and dashboards. Does **not** by itself determine the verdict — that's what `Action` is for.

### Policy

One rule, written in YAML by the customer and stored in their git repo. Has:
- `id` — unique within a workspace
- `when` — guard clauses (e.g. only on voice channel)
- `match` — what triggers it (regex / literal / semantic / combinations)
- `action` — what to do if matched: `Allow`, `Block`, `Rewrite`, `Escalate`
- `rewrite` — replacement text when action is `Rewrite`
- `severity` — `Low | Medium | High | Critical`

Example: see [`policies/refund-promise.yaml`](../../policies/refund-promise.yaml).

### Matcher

A single pattern that can fire. Three kinds today:

| Kind | Matches when... | Cost |
|---|---|---|
| `Literal` | substring is present | nanoseconds (Aho-Corasick later) |
| `Regex` | pattern matches | microseconds |
| `Semantic` | LLM judge says it does | 50–300ms; opt-in only |

A `Policy` combines matchers via `Single`, `any`, or `all` clauses.

### TriggeredPolicy

Record of one policy that matched on this request. Carries the policy id, severity, and a human reason. A `Decision` can have zero, one, or many.

### Trace ID

UUIDv4 (or caller-supplied) string that uniquely identifies one decision. Used for: log correlation, replay, dashboard drilldown, customer support tickets. Round-trips through the customer's logs and ours.

### Action vs Verdict

**Action** lives on a `Policy`. It's the policy's *wish* if it triggers.
**Verdict** lives on a `Decision`. It's the *outcome* the engine actually picked.

When multiple policies trigger, the engine picks the most severe action. Today that's a stub (just takes the first); the real merge logic comes in Phase 2.

---

## Technical terms

### Hot path

The synchronous `Engine::check` call. Must complete in microseconds for voice, low-milliseconds for chat. No allocation in the steady state, no locks, no I/O. **The product's competitive moat lives here.**

### Cold path

Anything off the request path: policy compilation at boot, decision logging (best-effort async), replay, audit. Can take whatever time it needs.

### Static matcher

A matcher whose decision does not depend on a model: regex, literal, fixed PII rules. Fast, deterministic, no network. Always eligible regardless of channel.

### LLM judge

A semantic matcher that calls a remote model (or a small local one) to decide whether a policy fires. Opt-in per policy. Has a hard deadline; if the deadline expires, the engine falls back to the policy's configured `on_judge_timeout` behavior.

### Tier orchestrator

The parallel-with-cancellation runner inside `tl-engine`. Spawns Tier 1 (Deterministic), Tier 2 (Fuzzy), and Tier 3 (LLM) concurrently against the same draft; the first non-`None` `BlockSignal` wins and cancels the rest via a shared `CancellationToken`. The v0 behaviour is fully described in [`v0-design-decisions.md` §4](v0-design-decisions.md).

### Judge

One LLM-backed check inside Tier 3. v0 ships three: `Hallucination` (is the draft grounded in the supplied docs?), `Tone` (does it match the agent profile's voice?), `Authority` (does it promise something the profile says the agent cannot promise?). Each judge is one round-trip through the `LlmRouter`, fanned out via `tokio::join!`. Compare with **LLM judge** above — that entry describes the *category* of matcher; this entry describes the *specific judges* the engine implements.

### LlmRouter

The single chokepoint for all outbound LLM traffic. Lives in `tl-llm`. Routes each `JudgeKind` to a configured primary provider (OpenAI / OpenRouter), retries on the fallback when the primary 5xx's or times out, charges the call to a per-tenant `TokenBudget`, and records `llm.provider` / `llm.model` / `llm.fallback_used` / token counts on the current `tracing` span. Configured via `config/llm-routing.toml`. Engine code never touches a provider directly — always through the router.

### Cache key

`BLAKE3(canonical_json(domain || agent_id || input || draft || sorted_doc_ids))`, computed in `tl-cache`. Same inputs → identical key → cached `Decision` is reused for the moka TTL window (5 min default). The cache lookup happens *before* any tier runs.

### Trace writer

The background `tokio` task spawned by `tl-storage::spawn_writer`. Drains an `mpsc::Receiver<Trace>` and flushes to the daily-partitioned `Traces` table in batches of up to 50 rows or every 100 ms, whichever comes first. The hot path only does `try_send` — if the channel is full the trace is dropped rather than blocking the request.

### Escalation worker

The background task spawned by `tl-server` that POSTs `Escalate` decisions to `TL_ESCALATION_WEBHOOK_URL`. Retries with the policy `1s, 5s, 30s, 2m` (max 4 attempts) and marks the row `sent` or `failed` in the `Escalations` table. On boot, drains any `pending` rows older than five minutes (recovers from a process restart). See PR 16 for the full state machine.

### Embedded mode

Customer pulls `tl-engine` directly as a Rust dependency and calls `Engine::check` in-process. No HTTP. Lowest possible latency; highest integration cost.

### Hosted mode

Customer hits our `tl-server` over HTTP from their Rust/TS/Python/whatever code. Default integration. Adds one network hop's worth of latency.

### Streaming mode

Used for voice and token-by-token text. The customer feeds chunks into a `StreamingChecker`; if a block fires, the customer interrupts the agent's output mid-sentence. Lives in `tl-stream`.

### Decision log

The persistent record of every `Decision`. Powers replay, audit, dashboards, and customer support. Implemented behind the `DecisionStore` trait so we can swap memory → Postgres → Postgres+ClickHouse without engine changes.

### Replay

Re-running a stored decision through a current (or hypothetical) engine snapshot. Used to:
- Validate policy changes against real traffic before deploying.
- Audit "would this still trigger?" after a model upgrade.
- Reproduce customer support tickets deterministically.

### Latency budget

The committed p99 for each channel. See [architecture.md](architecture.md#latency-budget-committed). Treat as a contract.

### Fail-open vs fail-closed

When the SDK can't reach the server (network blip, server down):
- **Fail-open**: caller proceeds as if `verdict = Allow`. Better availability, worse safety.
- **Fail-closed**: caller treats it as `Block` or `Escalate`. Better safety, worse availability.

Configured per policy. Voice/PII policies should fail closed. Brand-tone policies probably fail open.

### Shadow mode

A policy that *evaluates* but does not *enforce*. Used to A/B test new policies on production traffic before turning them on. Logs would-be triggers without affecting the verdict.

---

## Things that are NOT TrustLoopGuard

Words you might hear that we explicitly do **not** own:

- **Permission / OAuth scope checks** — Clawvisor's territory. We trust that the agent is allowed to act; we judge whether it *should*.
- **Prompt injection detection** — adjacent but separate. May be one matcher type later, but it's not the wedge.
- **Eval / regression suite for prompts** — that's offline, pre-deploy. We're online, runtime.
- **Workflow / agent orchestration** — never our problem.
- **The agent itself** — we don't make the agent smarter; we keep it from saying the wrong thing.
