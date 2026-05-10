# Glossary

Every domain term defined once. If you find yourself explaining a term in a PR review, add it here instead.

---

## Domain terms

### Agent

An AI program that takes actions or produces outputs on behalf of a customer's product. Examples: customer-support chatbot, sales voice agent, internal IT helper, coding agent. TrustLoopGuard does not run the agent — it sits in the agent's output path.

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
