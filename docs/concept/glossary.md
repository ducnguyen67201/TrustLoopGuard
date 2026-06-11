# Glossary

Every domain term defined once. If you find yourself explaining a term in a PR review, add it here instead.

---

## Domain terms

### Agent

An AI program that takes actions or produces outputs on behalf of a customer's product. Examples: customer-support chatbot, sales voice agent, internal IT helper, coding agent. TrustLoopGuard does not run the agent — it sits in the agent's output path.

### Agent profile

A YAML or JSON document registered once per agent (via `POST /v1/agents`) and referenced by `agent_id` on every check. Carries `scope` (`in_scope` / `out_of_scope`), `authority` (`can_promise` / `cannot_promise`), `tone` (target + forbidden), and approved `knowledge_sources` (`local` or `web`). Tier 3 LLM judges read this profile to know what the agent is *permitted* to claim — see `crates/tl-llm/src/prompts/`. Without a profile, Tier 3 reports `Skipped` (no grounding context).

### Channel

The medium an agent is operating on: `voice`, `chat`, or `email`. Channel drives the latency budget and which matchers are eligible. Voice has the strictest budget; email the loosest.

### CheckRequest

What a customer sends to TrustLoopGuard for a single decision. Contains:
- `agent_id` — which of the customer's agents this came from
- `channel` — voice, chat, etc.
- `input` — what the user said to the agent (context for the matchers)
- `proposed_output` — what the agent **wants** to say or do, before TrustLoopGuard sees it
- `policies` — optional policy ID list to scope evaluation
- `context` — free-form JSON the customer attaches (user tier, session id, etc.)
- `trace_id` — optional caller-supplied id for correlation
- `redaction` — optional metadata describing where redaction ran, whether it was applied, and which typed placeholder tokens were produced
- `run_id` / `run_event_id` / `run_event` — optional execution grouping metadata for trace linkage

### Decision

What TrustLoopGuard returns. The ground truth of a check.
- `trace_id` — set by caller or generated
- `verdict` — `Allow | Block | Rewrite | Escalate`
- `reason` — human-readable summary
- `triggered_policies` — list of every policy that fired
- `safe_output` — present when `verdict = Rewrite`; the suggested replacement
- `checked_input_excerpt` / `checked_output_excerpt` — optional bounded gateway debug excerpts, populated only when retention allows full body capture
- `latency_ms` — wall-clock time the engine spent
- `redaction` — optional summary copied from the sanitized `CheckRequest`
- `violated_rule`, `remediation`, `source_chain`, `risk_source`, `failure_mode`, `harm_class`, `constraints` — optional event-engine evidence, omitted from JSON when empty

### GuardEvent

The normalized event envelope for one proposed agent step. It is the SDK-first vocabulary that adapters converge on before runtime checking. A legacy `CheckRequest` can normalize into `GuardEvent { kind: output.proposed, action.operation: "output", ... }`. The contract is described in [event-engine.md](event-engine.md).

### Event kind

The dotted taxonomy on `GuardEvent.kind`, such as `output.proposed`, `tool.call.proposed`, `memory.write.proposed`, `shell.action.proposed`, or `database.mutation.proposed`. The spelling is stable wire contract, not display text.

### Principal

The identity block on a `GuardEvent`: workspace, environment, agent, optional user/session/task identity, and optional run/run-event linkage.

### Event action

The operation proposed by a `GuardEvent`, including the operation name, JSON parameters, and side-effect class. This is distinct from a policy `Action`, which describes what the engine should do after a policy matches.

### Source

One input that influenced a `GuardEvent`, such as user text, system instructions, tool output, memory, a file, web content, email content, or API data.

### Labels

Data-classification metadata attached to a `Source`: trust, confidentiality, and integrity. Unknown is the safe default when the runtime cannot classify a source yet.

### Provenance map

A map from output or parameter paths to source ids. It records which sources influenced which parts of a proposed event without copying the source content into the map itself.

### Tool metadata

Static metadata about a tool or host operation: side-effect class, whether the action is reversible, parameter roles, allowed source origins, approval requirements, and sandbox hints. Stored per workspace in the tool metadata registry and managed via `/v1/tool-metadata`.

### Tool resolution

The evidence the event pipeline attaches after looking up an event's `action.operation` in the tool metadata registry: `resolved` carries the matched metadata and makes the registry's side-effect class authoritative for the event; `unregistered` is the conservative default for unknown or disabled tools; `resolution_failed` records that the registry itself could not be consulted (e.g. a storage outage), so degraded resolution is never mistaken for absence. Resolution never changes a decision in observe-only mode.

### Label resolution

The evidence the event pipeline attaches after resolving every source's labels: per-source resolved labels with a label basis, derived labels per provenance path, and a policy status (`not_configured`, `applied`, or `unavailable` when the policy store could not be consulted — fail open, defaults apply). Label resolution never changes a decision in observe-only mode. See [event-engine.md](event-engine.md).

### Label basis

Why one resolved label family value was chosen for a source: `origin_default` (built-in default for the source origin), `workspace_override` (an enabled source label policy applied), or `declared` (the producer declared the value and it was accepted).

### Derived labels

Labels computed for a parameter path by deterministically folding the resolved labels of every source the provenance map lists for that path. Any untrusted contributor makes the path untrusted, the highest confidentiality claim wins, and integrity is capped by the weakest contributor. A path with no provenance entry has unknown derivation — absence is never treated as clean.

### Source label policy

A workspace-scoped per-origin label override managed via `/v1/label-policies`. Each row may override trust, confidentiality, and/or integrity for one origin; families left unset inherit the built-in origin defaults. Disabled rows stay manageable but are skipped at runtime.

### Checker

A deterministic, in-process, pure evaluation of the resolved event in the event pipeline — no I/O, no clock, no LLM. Four exist: `information_flow` (sensitive-data-to-external-sink and untrusted-control rules), `memory` (write-time memory protection), `parameter_auth` (parameter-source authorization against tool registry `allowed_sources`), and `approval` (escalation for tools whose registry metadata requires human approval). Each runs under an enforcement mode resolved per workspace and environment. See [event-engine.md](event-engine.md).

### Checker finding

One rule violation observed by a checker: the violated rule, a recommended verdict, the offending source chain, and forensic fields (`risk_source`, `failure_mode`, `harm_class`). Findings persist as trace evidence in `CheckerRun` entries on the event (`checks`), including in shadow mode where they carry the full hypothetical verdict without affecting the decision.

### Enforcement mode

Per-checker rollout state: `off` (default — checker not evaluated, no evidence), `shadow` (evaluated, full hypothetical evidence persisted, decision unchanged), `enforce` (evaluated, findings change the decision via worst-verdict-wins). Workspace-level modes live in workspace settings; per-environment overrides (see Environment checker-mode override) win per checker. Mode is configuration data, not a code fork: the same checker code runs in shadow and enforce.

### Environment checker-mode override

A per-environment row in `environment_checker_modes` overriding individual checker enforcement modes for one environment. `NULL` columns inherit the workspace mode, so an override can tighten or loosen one checker without restating the rest. Managed via `GET`/`PUT /v1/environments/{environment_id}/checker-modes`; a failed override lookup falls back to workspace modes. See [event-engine.md](event-engine.md).

### TrustLoopGuardBench

The behavioral regression harness for the event pipeline (`crates/tl-bench`): seed attack and benign-twin scenarios per risk track (indirect prompt injection, private-data flow, delayed memory risk) run through the pipeline under configurable checker modes, producing catch-rate/false-block metrics. Distinct from the criterion latency microbenchmarks in `tl-engine/benches`. See [trustloopguard-bench.md](trustloopguard-bench.md).

### Authority-bearing parameter

A tool parameter whose value controls what an action does or where its effects land — a recipient, destination, file path, or payment target — declared with role `authority_bearing` in tool metadata, in contrast to `content_bearing` parameters that only carry payload. The `parameter_auth` checker requires every authority-bearing parameter to carry provenance whose sources all match the tool's `allowed_sources`: a wrong source blocks and missing proof escalates in enforce mode, because missing provenance is never treated as clean.

### Redaction

Replacement of sensitive values in check content with typed placeholders such as `[EMAIL_1]`, `[SIN_1]`, or `[PERSON_NAME_1]`. Raw-to-token maps remain local to the redactor and are not sent to hosted TrustLoopGuard.

### Workspace Data Handling Mode

Workspace-level runtime setting that controls how `/v1/check` may handle request bodies. `raw_allowed` is the default. `redacted_only` rejects obvious raw sensitive values unless redaction metadata says redaction was applied or explicitly requests server redaction. `no_body_retention` and `private_deployment` are reserved modes for deployments with different processing or persistence rules.

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

One rule, written in YAML by the customer and stored in their git repo or the
cloud policy store. Has:
- `id` — unique within a workspace
- `description` — human-readable purpose for reviewers and dashboard users
- `when` — guard clauses (e.g. only on voice channel, one agent, or one domain)
- `match` — what triggers it (regex / literal / semantic / combinations)
- `action` — what to do if matched: `Allow`, `Block`, `Rewrite`, `Escalate`
- `rewrite` — replacement text when action is `Rewrite`
- `severity` — `Low | Medium | High | Critical`

Example: see [`policies/refund-promise.yaml`](../../policies/refund-promise.yaml).
Authoring guide: see [`docs/policies/README.md`](../policies/README.md).

### Policy family

The category a policy document belongs to, selected by a top-level `family:` tag in its YAML: `content` (the existing output/content policies above — also the default when the tag is absent), `flow` (source-to-sink and action-integrity rules), `parameter_source` (allowed-source rules for authority-bearing parameters), `approval` (human approval requirements), and `memory` (write-time memory rules). `tl-policy` parses and validates every family (`load_any_str`), surfaced through `POST /v1/policies/validate` and `tl policy validate`; content documents keep the exact legacy parser behavior. Storage and runtime evaluation of non-content families are not implemented yet — `POST /v1/policies` and `tl policy push` reject them with a clear error.

### Approval rule

A tool-metadata field (`ApprovalRule`) declaring that a tool requires human approval before execution: `required`, optional `approver_roles`, and an optional `reason` surfaced as remediation. Consumed by the `approval` checker, which escalates matching tool calls under its enforcement mode.

### Signal evidence

Advisory evidence from one LLM/classifier signal provider, attached by the event pipeline as `signals` on the event and persisted in traces. Signals never decide action verdicts; they exist so traces show what advisory layers observed alongside deterministic checker findings.

### Environment

A workspace-owned runtime and deployment boundary such as `dev`, `staging`, or `production`. Runtime API keys resolve one environment, runs and traces are stamped with it, analytics can filter/group by it, and policy deployment state is scoped to it. See [environments.md](environments.md).

### Policy deployment

The environment-specific state for a workspace-level `Policy`. It records whether a policy is enabled in an environment and which version is deployed there. Runtime policy loading uses policy deployments instead of the policy definition's legacy enabled flag.

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

### Run

One execution of a customer agent after guardrails have been assigned: a chat session, live call, workflow execution, or background job. A run groups many ordered run events and many `Decision` traces through `run_id`, but enforcement still happens per `CheckRequest`. Runs are described in [runs.md](runs.md).

### Run ID

TrustLoopGuard-generated UUID string that identifies one `Run`. SDKs pass it on `CheckRequest.run_id` so persisted traces can be grouped. This is distinct from `external_id`.

### Monitoring session

An SDK-generated id (`sess_<uuid>`) attached to the principal of every check and event a client emits after opting into monitoring at init (`with_monitoring()` in the Rust SDK). Caller-reported metadata used to isolate one process's traces — promoted to the `session_id` trace column and filterable via `GET /v1/traces?session_id=`. Never an enforcement or trust boundary; the server treats it as an opaque, length-bounded string and a caller-explicit `session_id` always wins over the SDK's.

### Run Event

One ordered moment inside a `Run`, such as a user turn, assistant turn, tool call, workflow step, interruption, retry, or system event. SDKs can pass `run_event_id` on `CheckRequest` so a decision trace is attached to the exact moment that produced it.

### External ID

Optional customer/platform identifier for the same run, such as a Twilio call ID, LiveKit room ID, n8n execution ID, or ticket ID. Used for correlation and support lookup only. Authorization and trace grouping use TrustLoopGuard's `run_id`.

### Run kind

The execution envelope for a run: `chat_session`, `live_call`, `workflow`, `job`, or `other`. This is not the same as `Channel`; a workflow can still contain chat checks, and a live call usually contains voice checks.

### Run status

Flexible lifecycle marker for monitoring: `warming`, `running`, `completed`, `failed`, or `canceled`. v1 allows simple status updates without enforcing a strict transition graph.

### Automated intervention

A TrustLoopGuard `Decision` whose verdict is `block`, `rewrite`, or `escalate`.

### Human review event

An append-only record of a customer reviewer outcome for one trace. The latest event is shown as the current review outcome, while the full event list remains audit history. See [human-review-analytics.md](human-review-analytics.md).

### Human intervention

A human review outcome of `corrected`, `rejected`, or `missed_issue`. This is separate from automated guardrail intervention.

### Action vs Verdict

**Action** lives on a `Policy`. It's the policy's *wish* if it triggers.
**Verdict** lives on a `Decision`. It's the *outcome* the engine actually picked.

When multiple policies trigger, the engine chooses the resulting verdict from
the active tier output and returns the matching decision metadata.

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

The background `tokio` task spawned by `tl-storage::spawn_writer`. Drains an `mpsc::Receiver<Trace>` and flushes to the daily-partitioned `traces` table in batches of up to 50 rows or every 100 ms, whichever comes first. The hot path only does `try_send` — if the channel is full the trace is dropped rather than blocking the request.

### Escalation worker

The background task spawned by `tl-server` that POSTs `Escalate` decisions to `TL_ESCALATION_WEBHOOK_URL`. Retries with the policy `1s, 5s, 30s, 2m` (max 4 attempts) and marks the row `sent` or `failed` in the `escalations` table. On boot, drains any `pending` rows older than five minutes to recover from a process restart.

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

### Workspace member

A user with current access to a workspace. Backed by `workspace_members` (workspace_id + user_id + role). Lifecycle owned by Rust (`crates/tl-storage/src/team_repo.rs`); the dashboard reads through `/v1/team/members`. See [team-and-invites.md](team-and-invites.md).

### OAuth identity

A Google or GitHub account linked to one local TrustLoopGuard user. Backed by `oauth_identities` (`provider`, `provider_subject`, `user_id`). Google/GitHub authenticate the browser user; Rust uses the link only to resolve the local app user that owns workspace memberships. See [authorization.md](authorization.md#oauth-users-google--github).

### Workspace invite

A single-use, time-limited credential that lets a non-member join a workspace at a specified role. The invite `id` doubles as the bearer token (opaque URL-safe random, single-use, 7-day TTL). Status transitions: `pending → accepted | revoked | expired`. See [team-and-invites.md](team-and-invites.md).

### Workspace API key

A `tl_live_...` bearer credential issued from `/api-keys` for customer SDK runtime calls. Rust generates it through `POST /v1/api-keys`, stores only a SHA-256 hash in `workspace_api_keys`, and resolves each request back to exactly one workspace and environment. See [authorization.md](authorization.md#workspace-api-keys).

### Workspace role

The permission level a user holds inside a workspace: `owner | admin | editor | viewer`. Stored on both `workspace_members.role` and `workspace_invites.role`. Distinct from `organization_role` (which gates org-wide membership).

---

## Things that are NOT TrustLoopGuard

Words you might hear that we explicitly do **not** own:

- **Permission / OAuth scope checks** — Clawvisor's territory. We trust that the agent is allowed to act; we judge whether it *should*.
- **Prompt injection detection** — adjacent but separate. May be one matcher type later, but it's not the wedge.
- **Eval / regression suite for prompts** — that's offline, pre-deploy. We're online, runtime.
- **Workflow / agent orchestration** — never our problem.
- **The agent itself** — we don't make the agent smarter; we keep it from saying the wrong thing.
