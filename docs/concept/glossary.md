# Glossary

Every domain term defined once. If you find yourself explaining a term in a PR review, add it here instead.

---

## Domain terms

### Agent

An AI program that takes actions or produces outputs on behalf of a customer's product. Examples: customer-support chatbot, sales assistant, internal IT helper, coding agent. Featherlane AI does not run the agent — it sits in the agent's output path.

### Agent profile

A YAML or JSON document registered once per agent (via `POST /v1/agents`) and referenced by `agent_id` on every check. Carries `scope` (`in_scope` / `out_of_scope`), `authority` (`can_promise` / `cannot_promise`), `tone` (target + forbidden), and approved `knowledge_sources` (`local` or `web`). Also carries optional hardening-loop inputs captured at import: `system_prompt`, `workflow_definition`, and `target_url` (the loopback endpoint the agent is reachable at, so the Attacks page targets it without re-typing — loopback-only, enforced by the dispatch SSRF guard). Tier 3 LLM judges read this profile to know what the agent is *permitted* to claim — see `crates/tl-llm/src/prompts/`. Without a profile, Tier 3 reports `Skipped` (no grounding context).

### Channel

The medium an agent is operating on: `chat` or `email`. Channel drives the latency budget and which matchers are eligible; chat carries the stricter budget, email the loosest. The `voice` variant remains in the wire contract for backward compatibility but is **deprecated and not a supported channel** — new integrations should use `chat` or `email`.

### Decision

`AuthorizationDecision` is what Featherlane AI returns from every runtime authorization domain. It contains a trace id, domain, one canonical effect, a human reason, all findings, optional durable intent status, transformed value, approval/grant/lease references, receipt id, and latency. Findings retain the rule, source, severity, requirement, remediation, and evidence even when a stronger effect wins.

### GuardEvent

The normalized event envelope for one proposed agent step and the public runtime request body for `POST /v1/events`. The contract is described in [event-engine.md](event-engine.md).

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

The evidence the event pipeline attaches after looking up an event's `action.operation` in the tool metadata registry: `resolved` carries the matched metadata and makes the registry's side-effect class authoritative for the event; `unregistered` is the conservative default for unknown or disabled tools; `resolution_failed` records that the registry itself could not be consulted (e.g. a storage outage), discards the collector-supplied side-effect class, and always defers execution. Resolved and unregistered outcomes are evidence for checkers and policies; resolution failure is a fail-closed pipeline invariant.

### Label resolution

The evidence the event pipeline attaches after resolving every source's labels: per-source resolved labels with a label basis, derived labels per provenance path, and a policy status (`not_configured`, `applied`, or `unavailable` when the policy store could not be consulted — fail open, defaults apply). Label resolution is evidence; checkers and policies decide whether that evidence changes the decision. See [event-engine.md](event-engine.md).

### Label basis

Why one resolved label family value was chosen for a source: `origin_default` (built-in default for the source origin), `workspace_override` (an enabled source label policy applied), or `declared` (the producer declared the value on a trusted user/system channel and it was accepted). Declarations on externally controlled origins never produce the `declared` basis.

### Derived labels

Labels computed for a parameter path by deterministically folding the resolved labels of every source the provenance map lists for that path. Any untrusted contributor makes the path untrusted, the highest confidentiality claim wins, and integrity is capped by the weakest contributor. A path with no provenance entry has unknown derivation — absence is never treated as clean.

### Source label policy

A workspace-scoped per-origin label override managed via `/v1/label-policies`. Each row may override trust, confidentiality, and/or integrity for one origin; families left unset inherit the built-in origin defaults. Disabled rows stay manageable but are skipped at runtime. Workspace runtime keys cannot mutate these governing rows.

### Checker

A deterministic, in-process, pure evaluation of the resolved event in the event pipeline — no I/O, no clock, no LLM. Five exist: `information_flow` (sensitive-data-to-external-sink and untrusted-control rules), `memory` (write-time memory protection), `parameter_auth` (parameter-source authorization against tool registry `allowed_sources`), `value_limit` (numeric bounds on a parameter value against a registry [Value limit](#value-limit)), and `approval` (an explicit `require_approval` requirement for tools whose registry metadata requires human sign-off). Each runs under an enforcement mode resolved per workspace and environment; `value_limit` shares the `parameter_auth` mode. The pure-checker contract is also a boundary: a per-call cap is a checker, but a rate/quota limit (needs state and a clock) is not. See [event-engine.md](event-engine.md).

### Checker finding

One rule violation observed by a checker: the stable rule id, recommended effect, source chain, severity, requirement, remediation, and evidence. Findings persist in shadow mode without affecting the final effect.

### Enforcement mode

Per-checker rollout state: `off` (not evaluated), `shadow` (hypothetical findings persisted, decision unchanged), or `enforce` (findings contribute to canonical effect composition). Per-environment overrides win over workspace defaults.

### Environment checker-mode override

A per-environment row in `environment_checker_modes` overriding individual checker enforcement modes for one environment. `NULL` columns inherit the workspace mode, so an override can tighten or loosen one checker without restating the rest. Managed via `GET`/`PUT /v1/environments/{environment_id}/checker-modes`; a failed override lookup fails the request rather than silently weakening enforcement. See [event-engine.md](event-engine.md).

### Attack success rate

ASR: the share of attack cases whose adversarial objective landed. Benign
controls are excluded from the denominator.

### Benign utility

BU: the share of benign tasks that still completed successfully.

### Utility under attack

UA: the share of legitimate task completion preserved when adversarial content
is present.

### Authority-bearing parameter

A tool parameter whose value controls what an action does or where its effects land — a recipient, destination, file path, or payment target — declared with role `authority_bearing` in tool metadata, in contrast to `content_bearing` parameters that only carry payload. The `parameter_auth` checker requires every authority-bearing parameter to carry provenance whose sources all match the tool's `allowed_sources`: a wrong source produces `deny` and missing proof produces `defer` in enforce mode, because missing provenance is never treated as clean.

### Value limit

A tool-metadata field (`ParamLimit` on a `ParamSpec`) declaring inclusive numeric bounds on a parameter's value: `max`, `min`, and an `on_breach` effect (`deny` by default, or `require_approval`). Bounds are integers in the tool's own minor units. A present but unverifiable value produces `defer`; a configured cap is never silently passed.

### Redaction

Replacement of sensitive values in check content with typed placeholders such as `[EMAIL_1]`, `[SIN_1]`, or `[PERSON_NAME_1]`. Raw-to-token maps remain local to the redactor and are not sent to hosted Featherlane AI.

### Workspace Data Handling Mode

Workspace-level runtime setting that controls how `/v1/events` may handle request bodies. `raw_allowed` is the default for event submissions. `redacted_only`, `no_body_retention`, and `private_deployment` are reserved modes for deployments with different processing or persistence rules.

### Authorization effect

The one runtime outcome on an `AuthorizationDecision`: `permit`, `transform`, `deny`, `require_approval`, or `defer`. Composition precedence is `deny > defer > require_approval > transform > permit`. Only `require_approval` may be satisfied by a matching grant; `defer` means evidence or system state must change.

### Severity

How bad a finding is: `Low`, `Medium`, `High`, `Critical`. Used for sorting and dashboards. It does not by itself determine the authorization effect.

### Policy

One rule, written in YAML by the customer and stored in their git repo or the
cloud policy store. Has:
- `id` — unique within a workspace
- `description` — human-readable purpose for reviewers and dashboard users
- `when` — guard clauses (e.g. only on chat channel, one agent, or one domain)
- `match` — what triggers it (regex / literal / semantic / combinations)
- `effect` — what finding to emit if matched: `permit`, `deny`, `transform`, `require_approval`, or `defer`
- `transform` — replacement value when the effect is `transform`
- `severity` — `Low | Medium | High | Critical`

Example: see [`policies/refund-promise.yaml`](../../policies/refund-promise.yaml).
Authoring guide: see [`docs/policies/README.md`](../policies/README.md).

### Policy family

The category a policy document belongs to, selected by a top-level `family:` tag in its YAML: `content` (the existing output/content policies above — also the default when the tag is absent), `flow` (source-to-sink and action-integrity rules), `parameter_source` (allowed-source rules for authority-bearing parameters), `approval` (human approval requirements), `memory` (write-time memory rules), `financial` (typed money-action controls), `source_label` (source-label overrides), and `tool` (deterministic executable-tool controls). `tl-policy` parses and validates every family (`load_any_str`), surfaced through `POST /v1/policies/validate` and `tl policy validate`; content documents keep the exact legacy parser behavior. Families share the Rust policy registry, versioning, and environment deployment lifecycle.

### Tool policy

A `family: tool` policy evaluated against an exact executable tool subject. It can scope by agent, operation, side-effect class, and structured tool identity, then match analyzer facts or a JSON Pointer into action parameters. It emits `deny`, `defer`, or `require_approval` findings through the common authorization kernel. See [command-safety.md](command-safety.md).

### Coding-agent tool gate

A user-owned Claude Code, Codex, or OpenCode adapter that blocks a host-emitted tool call while the existing Rust runtime evaluates its normalized `GuardEvent`. The gate is installed outside the guarded workspace, fails closed for registered projects, and owns execution-lease reconciliation but no policy logic. See [coding-agent-tool-gates.md](coding-agent-tool-gates.md).

### Host-emitted coverage

The set of tool calls for which a coding-agent host invokes its blocking before-tool extension point. `universal` means every call emitted through that extension is gated; `host_emitted_only` means the host has built-in handlers that do not expose the event and therefore cannot be intercepted by the adapter. Configuration presence alone does not prove coverage.

### Shell command fact

A neutral key/value observation produced by the pure bounded shell analyzer, such as `shell.risk=filesystem_recursive_delete` or `shell.target_scope=workspace`. Facts never decide an effect by themselves; only an enabled tool policy interprets them. See [command-safety.md](command-safety.md#shell-facts).

### Command analysis status

Whether deterministic shell analysis was `complete`, `partial`, or `unavailable`. Partial and unavailable analysis prevent an unproven scoped fact policy from silently permitting a command; proven findings and parameter-only policies still evaluate. See [command-safety.md](command-safety.md#bounds-and-incomplete-analysis).

### Financial action

A typed domain command for money-bearing or regulated work, such as a refund, payment, payout, purchase, invoice approval, or treasury transfer. It carries integer-minor-unit money, action kind, principal, rail, optional counterparty, and metadata. Authorization is supplied separately as a common `AuthorizationClaim`. See [financial-authorization.md](financial-authorization.md).

### Financial policy family

A `family: financial` policy applying only to typed [Financial action](#financial-action) requests. Selectors include agent ids, action kinds, operations, currencies, and rails. Controls include hard caps, approval thresholds, grant requirements, counterparty rules, and trusted eligibility preconditions. It emits common findings and authority requirements; the financial service supplies live ledger and evidence state.

### Financial spending control

The dashboard-facing authoring surface for a `family: financial` policy. A spending control is created from Financial -> Spending controls, posted as typed JSON to `POST /v1/financial/policies`, stored in the unified Rust policy registry, enabled per environment, and evaluated before financial action execution. It is a different policy family from content protection rules, not a separate policy system.

### LLM spending cap

A `family: financial` policy with `meter: llm_usage`. Before an OpenAI-compatible Gateway request
reaches its provider, Featherlane AI prices the bounded maximum token usage and atomically reserves
that amount against the applicable daily, weekly, and monthly caps for the runtime-key principal.
The provider is not called when that reservation would exceed a cap. The cap counts only
`customer_inference` usage; Featherlane AI's own semantic-judge overhead is accounted separately.

### LLM usage event

A durable, precisely priced model invocation in the Rust-owned usage ledger. Its `kind` is either
`customer_inference` for the customer's Gateway provider call or `guardrail` for Featherlane AI's
semantic judge. Exact nano-USD values are serialized as decimal strings to avoid JavaScript integer
loss; legacy minor-unit fields remain compatibility projections. Provider invoices are still the
authoritative billing record.

### Authorization intent

The durable lifecycle record for one executable proposed action. It is scoped to a workspace, environment, principal, domain, operation, subject, and fingerprint and moves through `evaluating`, `pending_approval`, `authorized`, `denied`, `deferred`, `canceled`, or `expired`. An intent records what is trying to happen; it is not itself permission to execute.

### Authorization approval

A pending human decision tied to one [Authorization intent](#authorization-intent) and its immutable [Approval envelope](#approval-envelope). It is the only item shown in the actionable `/approvals` queue and is labeled by domain (`tool`, `financial`, or `content`). Approving creates a grant; it does not execute the action. Denied, canceled, expired, and already-approved records are not pending queue work.

### Authorization grant

Database-backed, revocable authority for one principal, domain, capability, set of requirement IDs, and typed scope. A grant is either `exact_once` or reusable `scoped`, may expire or be use-limited, and comes from authenticated user intent, reviewer approval, or a workspace administrator. It can satisfy matching approval requirements but never widens current policy.

### Authorization claim

The caller's explicit reference to a `grant_id` and stable `attempt_id` when retrying an action. The claim is not trusted on its own: the kernel loads the grant, verifies its tenant, principal, domain, capability, requirements, scope, expiry, and use limits, then re-evaluates current policy and live state before execution.

### Approval envelope

The immutable reviewer view of one authorization intent: domain, principal, subject and fingerprint, capability, requirement IDs, proposed reusable scope, policy versions, and expiry. The server hashes the canonical envelope as `sha256:v1:<hex>`; the reviewer decision must echo that hash.

### Financial grant scope

The typed financial variant of an authorization grant. It can constrain action kind, operation, rail, currency, maximum amount, counterparties, x402 host/resource/network/asset/payee, and required preconditions.

### Financial action eligibility

Evidence-backed business legitimacy for a financial action. For example, a refund may require proof that the order exists, payment was captured, the refund window is open, the amount is within refundable balance, the destination is the original payment method, and the refund is not a duplicate. AI output may draft the candidate action, but it is not trusted evidence. `family: financial` policies can require preconditions and the financial service evaluates them from trusted `EvidenceRef.metadata` before execution.

### Evidence ref

An opaque reference to trusted evidence used by financial eligibility or proof generation. It records the evidence source, source id, kind, optional observation time, and metadata without making the AI agent the authority for the fact.

### Financial receipt

A tenant-scoped execution proof for a financial action. It links the action and common authorization receipt to ledger entry ids and provider proof.

### Financial action outcome

The operational and risk result of a financial action after authorization or execution. Outcomes record provider status, provider reference, reversal capability, recovery status, dispute/loss metadata, and final loss amount when known. Outcomes do not replace ledger entries: ledger entries answer spend/reservation questions, while outcomes answer whether the action succeeded, failed, reversed, recovered, was disputed, or caused loss.

### Action underwriting

A separately agreed commercial service that assigns a risk price and coverage terms to a proposed agent action before execution. The open-source Featherlane AI runtime supplies authorization, receipts, and outcome/recovery records; it does not by itself bind coverage, issue an insurance policy, or guarantee payment.

### Financial action state

The Rust-derived product projection of a financial action's trusted evidence, authorization, and execution lifecycle. It lets callers distinguish `not_executable` eligibility failures, policy `blocked` actions, `held_for_approval` actions, authorized work, active execution, and terminal outcomes without inferring meaning from raw `defer`, `evaluating`, or `not_started` fields. It is computed from existing records and is not a separate durable state machine.

### Agentic payment

An agent-initiated typed [Financial action](#financial-action) that needs authorization before a payment credential, signature, or settlement attempt is released. In the x402 path, the agent submits the payment requirement to Featherlane AI first, receives an action-bound authorization/reservation, then either commits with settlement proof or rolls the reservation back before settlement.

### x402 payment requirement

The x402 payment details a resource server asks the agent to satisfy: amount, payee, and optional network, asset, scheme, resource, method, host, facilitator, and raw provider payload. Featherlane AI normalizes those fields into a canonical hash so the later settlement proof can be checked against the exact authorized requirement.

### Payment session

A time-boxed budget context for agentic payments. A payment session is bound to one workspace principal and currency and tracks maximum amount, reserved amount, committed amount, and released amount. It is the concurrency boundary that prevents parallel agent payments from overspending the same budget.

### Payment reservation

An action-bound hold against a [Payment session](#payment-session), keyed by normalized x402 payment requirement hash. Reserved budget can be committed after settlement proof or released before settlement. Releasing a reservation is not the same thing as reversing a settled payment.

### Reversal capability

Provider-aware description of how an executed or pending financial action can be unwound, if at all: cancel before capture, cancel a pending refund, provider reversal, compensating charge, internal balance adjustment, manual recovery, or none. Featherlane AI does not promise a universal undo button.

### Recovery status

The current recovery state for a financial action outcome: not needed, unavailable, available, started, recovered, failed, or requiring manual work. This vocabulary supplies outcome evidence for separately agreed [action underwriting](#action-underwriting) without making the runtime itself an insurance or guarantee system.

### MCP OAuth

OAuth 2.1 authentication for the hosted MCP resource. `tl-server` owns discovery, dynamic client registration, PKCE-bound single-use codes, token exchange, and refresh rotation. Dashboard consent binds one current workspace member and registered agent; access, code, and refresh records retain that binding. The audience-bound access JWT carries signed `workspace_id`, `agent_id`, OAuth client, and `mcp:tools` scope. The `/mcp` resource-server lane revalidates membership and agent existence and stamps trusted workspace identity from the token. A `401` advertises resource metadata through `WWW-Authenticate` so clients can discover the flow.

### Approval rule

A tool-metadata field (`ApprovalRule`) declaring that a tool requires human approval before execution: `required`, optional `approver_roles`, and an optional `reason` surfaced as remediation. Consumed by the `approval` checker, which emits an explicit `require_approval` requirement for matching tool calls under its enforcement mode.

### Invocation id

A caller-generated stable identifier for one proposed action. Initial transport retries retain it; a changed action under the same workspace, environment, and invocation id conflicts. See [authorization-kernel.md](authorization-kernel.md).

### Tool identity

The exact execution target bound to an approval: downstream server id, tool name, and canonical schema hash.

### Action fingerprint

A Rust-computed versioned SHA-256 binding for one generic tool invocation, including server-resolved scope, principal/run identity, invocation id, operation, tool identity, and parameters. Unlike a reusable financial approval fingerprint, it authorizes no family of later actions.

### Execution lease

The one-attempt execution right claimed after current authorization returns `permit` or an executable transformed result. It is `claimed`, `consumed`, `canceled`, or `expired`; same-attempt retries return the same claimed or consumed lease. A lease prevents retries from turning one authorization into duplicate side effects. See [authorization-kernel.md](authorization-kernel.md).

### Authorization receipt

The common audit record written for an authorization evaluation. It preserves the final effect, intent status, reason, findings, policy versions, subject fingerprint, domain evidence, optional principal/operation/run linkage, and any approval, grant, or lease references. A receipt explains a decision but grants no authority and does not replace domain evidence such as a financial execution receipt.

### Authorization activity

The environment-scoped, newest-first view of authorization receipts shown on the Authorization screen. It includes permits, transforms, denials, approval requirements, and deferrals. Activity is audit evidence; it is not an approval queue and a permit row does not mean a human approved the action.

### MCP proxy

The separate `apps/mcp-proxy` process that mirrors one downstream stdio MCP server and uses durable exact-action approval before forwarding a tool call. It is not the operator-facing Featherlane AI MCP server.

### Hosted MCP access gateway

The Rust-owned, OAuth-authenticated `/mcp` endpoint that presents each signed
member-and-agent pair only its active assigned tools. It applies the common
runtime policy and authorization kernel before contacting an
administrator-approved remote server and again before disclosing the result.
It is distinct from both `apps/mcp-server` and `apps/mcp-proxy`.

### MCP catalog

The durable, administrator-accepted snapshot of tools exposed by one remote MCP
server. Runtime discovery uses this snapshot without upstream I/O. A tool is
executable only while its catalog status is `active`.

### MCP tool assignment

A workspace-scoped entitlement binding one active catalog tool to one current
member and one registered agent. The exact signed tuple is required for
discovery and execution. A legacy member-only row is unbound and inactive until
an administrator selects an agent. Assignment does not bypass runtime policy.

### Governance context

The required `__featherlane_ai` object added to each hosted MCP tool schema. It
declares the latest user intent, a purpose, and an optional destination. Rust
validates and removes it before the upstream call, then uses a normalized copy
for policy. Standard MCP does not transport the surrounding chat prompt, so the
context is client-declared rather than cryptographic proof of that prompt.

### Signal evidence

Advisory evidence from one LLM/classifier signal provider, attached by the event pipeline as `signals` on the event and persisted in traces. Signals never decide authorization effects; they exist so traces show what advisory layers observed alongside deterministic checker findings.

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

One execution of a customer agent after guardrails have been assigned: a chat session, live call, workflow execution, or background job. A run groups many ordered run events and many `Decision` traces through `run_id`, but enforcement still happens per `GuardEvent`. Runs are described in [runs.md](runs.md).

### Run ID

Featherlane AI-generated UUID string that identifies one `Run`. SDKs pass it on `GuardEvent.principal.run_id` so persisted traces can be grouped. This is distinct from `external_id`.

### Monitoring session

An SDK-generated id (`sess_<uuid>`) attached to the principal of every event a client emits after opting into monitoring at init (`with_monitoring()` in the Rust SDK). Caller-reported metadata used to isolate one process's traces — promoted to the `session_id` trace column and filterable via `GET /v1/traces?session_id=`. Never an enforcement or trust boundary; the server treats it as an opaque, length-bounded string and a caller-explicit `session_id` always wins over the SDK's.

### Run Event

One ordered moment inside a `Run`, such as a user turn, assistant turn, tool call, workflow step, interruption, retry, or system event. SDKs can pass `run_event_id` on `GuardEvent.principal` so a decision trace is attached to the exact moment that produced it.

### External ID

Optional customer/platform identifier for the same run, such as a Twilio call ID, LiveKit room ID, n8n execution ID, or ticket ID. Used for correlation and support lookup only. Authorization and trace grouping use Featherlane AI's `run_id`.

### Run kind

The execution envelope for a run: `chat_session`, `live_call`, `workflow`, `job`, or `other`. This is not the same as `Channel`; a workflow can still contain chat checks, and a live call usually groups realtime checks.

### Run status

Flexible lifecycle marker for monitoring: `warming`, `running`, `completed`, `failed`, or `canceled`. v1 allows simple status updates without enforcing a strict transition graph.

### Automated intervention

A Featherlane AI decision whose effect is `deny`, `transform`, `require_approval`, or `defer`.

### Human review event

An append-only record of a customer reviewer outcome for one trace. The latest event is shown as the current review outcome, while the full event list remains audit history. See [human-review-analytics.md](human-review-analytics.md).

### Human intervention

A human review outcome of `corrected`, `rejected`, or `missed_issue`. This is separate from automated guardrail intervention.

### Policy effect vs authorization effect

A policy or checker emits a finding effect. The coordinator composes every finding, resolves only explicit requirements against matching grants, and returns the final authorization effect without discarding weaker evidence.

---

## Technical terms

### Hot path

The synchronous `Engine::check` call. Must complete in microseconds for streaming, low-milliseconds for chat. No allocation in the steady state, no locks, no I/O. **The product's competitive moat lives here.**

### Cold path

Anything off the request path: policy compilation at boot, decision logging (best-effort async), replay, audit. Can take whatever time it needs.

### Static matcher

A matcher whose decision does not depend on a model: regex, literal, fixed PII rules. Fast, deterministic, no network. Always eligible regardless of channel.

### LLM judge

A semantic matcher evaluator that calls the configured `semantic_policy` model route to decide whether a policy fires. If that route is absent, semantic matchers are skipped. High-confidence matches apply the policy action; ambiguous or unavailable evidence produces `defer` for high and critical policies and fails open for lower severities. A reviewer cannot approve around missing evidence.

### Tier orchestrator

The parallel-with-cancellation runner inside `tl-engine`. Spawns Tier 1 (Deterministic), Tier 2 (Fuzzy), and Tier 3 (LLM) concurrently against the same draft; the first non-`None` `BlockSignal` wins and cancels the rest via a shared `CancellationToken`. The v0 behaviour is fully described in [`v0-design-decisions.md` §4](v0-design-decisions.md).

### Judge

One LLM-backed check inside Tier 3. v0 ships three: `Hallucination` (is the draft grounded in the supplied docs?), `Tone` (does it match the agent profile's voice?), `Authority` (does it promise something the profile says the agent cannot promise?). Each judge is one round-trip through the `LlmRouter`, fanned out via `tokio::join!`. Compare with **LLM judge** above — that entry describes the *category* of matcher; this entry describes the *specific judges* the engine implements.

### LlmRouter

The single chokepoint for all outbound LLM traffic. Lives in `tl-llm`. Routes each `JudgeKind` to a configured primary provider (OpenAI / OpenRouter), retries on the fallback when the primary 5xx's or times out, charges the call to a per-tenant `TokenBudget`, and records `llm.provider` / `llm.model` / `llm.fallback_used` / token counts on the current `tracing` span. Budget admission is atomic: because actual token usage is available only after the provider responds, capped tenants allow one evaluation session in flight while the judges within that session still fan out concurrently; unlimited tenants remain concurrent. Failed or cancelled sessions release admission and only completed calls charge usage. Configured via `config/llm-routing.toml`. Engine code never touches a provider directly — always through the router.

### Cache key

`BLAKE3(canonical_json(domain || agent_id || input || draft || sorted_doc_ids))`, computed in `tl-cache`. Same inputs → identical key → cached `Decision` is reused for the moka TTL window (5 min default). The cache lookup happens *before* any tier runs.

### Trace writer

The background `tokio` task spawned by `tl-storage::spawn_writer`. Drains an `mpsc::Receiver<Trace>` and flushes to the daily-partitioned `traces` table in batches of up to 50 rows or every 100 ms, whichever comes first. The hot path only does `try_send` — if the channel is full the trace is dropped rather than blocking the request.

### Operational escalation worker

The background task spawned by `tl-server` that POSTs `defer` decisions to `TL_ESCALATION_WEBHOOK_URL`. Retries with the policy `1s, 5s, 30s, 2m` (max 4 attempts) and marks the delivery row `sent` or `failed` in the `escalations` table. Authorization decisions with `require_approval` enter the approval queue instead; they are not operational escalation deliveries. On boot, the worker drains pending deliveries older than five minutes to recover from a process restart.

### Embedded mode

Customer pulls `tl-engine` directly as a Rust dependency and calls `Engine::check` in-process. No HTTP. Lowest possible latency; highest integration cost.

### Hosted mode

Customer hits our `tl-server` over HTTP from their Rust/TS/Python/whatever code. Default integration. Adds one network hop's worth of latency.

### Streaming mode

Used for token-by-token text streaming. The customer feeds chunks into a `StreamingChecker`; if a block fires, the customer interrupts the agent's output mid-sentence. Lives in `tl-stream`.

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
- **Fail-open**: caller proceeds as if the effect were `permit`. Better availability, worse safety.
- **Fail-closed**: caller treats it as `deny` or `defer`. Better safety, worse availability.

SDK callers choose this behavior with their error handler. Server-side semantic policy judge uncertainty is handled by the policy evaluator: high and critical semantic policies defer, while lower-severity semantic policies fail open.

### Shadow mode

A policy or checker that evaluates but does not enforce. It records hypothetical findings without affecting the authorization effect.

### Workspace member

A user with current access to a workspace. Backed by `workspace_members` (workspace_id + user_id + role). Lifecycle owned by Rust (`crates/tl-storage/src/team_repo.rs`); the dashboard reads through `/v1/team/members`. See [team-and-invites.md](team-and-invites.md).

### Platform administrator

A user with support and debugging access across every active workspace. Backed by the default-false
`users.is_platform_admin` flag and enforced by Rust; it is distinct from the per-workspace
`admin` role and does not create workspace membership. See
[web dashboard authentication](web-dashboard-authentication.md).

### OAuth identity

A Google or GitHub account linked to one local Featherlane AI user. Backed by `oauth_identities` (`provider`, `provider_subject`, `user_id`). Google/GitHub authenticate the browser user; Rust uses the link only to resolve the local app user that owns workspace memberships. See [authorization.md](authorization.md#oauth-users-google--github).

### Workspace invite

A single-use, time-limited credential that lets a non-member join a workspace at a specified role. The invite `id` doubles as the bearer token (opaque URL-safe random, single-use, 7-day TTL). Status transitions: `pending → accepted | revoked | expired`. See [team-and-invites.md](team-and-invites.md).

### Workspace API key

A `tl_live_...` bearer credential issued from `/api-keys` for customer SDK runtime calls. Rust generates it through `POST /v1/api-keys`, stores only a SHA-256 hash in `workspace_api_keys`, and resolves each request back to exactly one workspace and environment. See [authorization.md](authorization.md#workspace-api-keys).

### Workspace role

The permission level a user holds inside a workspace: `owner | admin | editor | viewer`. Stored on both `workspace_members.role` and `workspace_invites.role`. Distinct from `organization_role` (which gates org-wide membership).

### Red-team job

A durable, Rust-owned record of one single-target attack run, dispatched via `POST /v1/redteam/dispatch` and tracked through a `JobStatus` lifecycle (`queued → running → complete | error | canceled`). The job persists in `redteam_jobs`; each independent test case persists as a `redteam_attack_sessions` row with ordered `redteam_session_events`. The dashboard Attacks tab dispatches, polls, and cancels it. See [redteam-dispatch.md](redteam-dispatch.md).

### Attack runner

A stateless executor that runs red-team attacks against a target agent and returns scored attack sessions to Rust. Rust reaches it over HTTP at `REDTEAM_RUNNER_URL`, owns the durable job, and persists the sessions/events; the runner persists no product data and is outside the public wire contract. See [redteam-dispatch.md](redteam-dispatch.md).

### Vulnerability report

A presentation-ready view of a completed [red-team job](#red-team-job) — the findings, severity, and aggregates derived from its attack sessions, optionally a same-agent before/after comparison. Computed by Rust (`build_report`) and served by `GET /v1/redteam/jobs/{id}/report`; the dashboard renders it (and the shared variant) as a branded PDF. See [redteam-report-sharing.md](redteam-report-sharing.md).

### Report severity

The classification Rust assigns a report finding: `critical | high | medium | low | info`. Only *landed* attacks are live vulnerabilities (credential/prompt-leak disclosures are `critical`); blocked and clean control cases are `info`. The report's overall `risk_level` is the worst landed severity.

### Report share token

A durable, expiring, revocable capability that grants public, read-only access to one vulnerability report. The unguessable `rpt_`-prefixed token is the sole bearer credential for the public endpoint (`GET /v1/redteam/reports/{token}`); a prospect can open the link without a dashboard account. Stored in `redteam_report_shares`. See [redteam-report-sharing.md](redteam-report-sharing.md).

### Harden candidate

A guardrail policy synthesized from a landed red-team attack and *verified* before it is offered. Rust classifies the harm mechanism, builds a [matcher](#matcher) (a semantic clause generalized to the leak's class, plus a regex backstop for credentials), and re-runs it against the landed cases, obfuscation variants, and benign controls; only candidates that block what landed without false-blocking a control are recommended (`enabled = false`). See [redteam-harden.md](redteam-harden.md).

### Hardening loop

The repeatable product cycle: import an agent → derive tailored [attack vectors](#attack-vector) from its definition → run them → synthesize [verified](#harden-candidate) guardrail policies from what lands → refine and repeat. It stitches the attack-vector planner (new) onto the existing [dispatch](redteam-dispatch.md) and [harden](redteam-harden.md) steps. The exploit proves the policy — there is no blank policy page. See [agent-hardening-loop.md](agent-hardening-loop.md).

### Attack vector

A single tailored attack derived from an agent's own definition by `POST /v1/agents/{id}/redteam/plan`: a `goal` (what a successful attack makes the agent do, scored against observed behavior), a `technique` class, a `target_operation` (the sink it aims at, or `chat_reply`), an `injection_payload` seed, and the [`source → sink` path](#sourcesink-path) it exploits. Vectors are saved as part of an [attack plan](#attack-plan); the dashboard feeds a selected plan's vectors into a dispatch as seeds, which the [attack runner](#attack-runner) strengthens — so attacks are gray-box, not generic. See [agent-hardening-loop.md](agent-hardening-loop.md).

### Attack plan

A saved, named set of [attack vectors](#attack-vector) (plus the analyzer's `source → sink` paths) for one agent, persisted Rust-owned in `redteam_plans`. Generating a plan saves it; an agent's plans are listed newest-first and can be re-selected to seed a run or deleted. The body is stored as a JSONB blob, so a plan is re-run rather than regenerated (which would re-pay the LLM). See [agent-hardening-loop.md](agent-hardening-loop.md).

### Source→sink path

An injectable data path the static `workflow_analyzer` finds in an imported [workflow definition](#workflow-definition): an untrusted **source** node (webhook, form trigger, inbound email, uploaded document) that can reach a dangerous **sink** node (HTTP egress, outbound email, database, code execution) through the workflow's `connections`. The workflow graph *is* the provenance graph — these paths ground attack generation (what to inject, which sink to drive) and seed static preventive policies (what flow to block). See [agent-hardening-loop.md](agent-hardening-loop.md).

### Workflow definition

An optional machine-readable agent definition (an n8n workflow export today) imported on an [agent profile](#agent-profile) alongside or instead of the chat `system_prompt`. The hardening loop analyses it for [`source → sink` paths](#sourcesink-path) to tailor attacks; absent ⇒ a plain chat agent. Kept verbatim as `{ source, definition }`.

### GitHub installation

A selected-repository Featherlane AI GitHub App installation linked to one workspace. It is separate from dashboard GitHub OAuth login and is used only for repository automation. See [github-assisted-installation.md](github-assisted-installation.md).

### Repository connection

A Rust-owned mapping from one GitHub repository/root to one Featherlane AI agent and environment. It records the integration recipe version and is the activation marker's durable identity. See [github-assisted-installation.md](github-assisted-installation.md).

### Integration job

A durable GitHub-assisted installation lifecycle record: analysis queued/running, awaiting approval, applying, draft PR open, awaiting verification, verified, closed unmerged, error, or cancelled. See [github-assisted-installation.md](github-assisted-installation.md).

### Integration recipe

A versioned set of constraints for generating repository edits. The first recipe, `typescript-nextjs-v1`, targets TypeScript/Next.js repositories and emits a marked SDK integration. See [github-assisted-installation.md](github-assisted-installation.md).

---

## Things that are NOT Featherlane AI

Words you might hear that we explicitly do **not** own:

- **Permission / OAuth scope checks** — Clawvisor's territory. We trust that the agent is allowed to act; we judge whether it *should*.
- **Prompt injection detection** — adjacent but separate. May be one matcher type later, but it's not the wedge.
- **Eval / regression suite for prompts** — that's offline, pre-deploy. We're online, runtime.
- **Workflow / agent orchestration** — never our problem.
- **The agent itself** — we don't make the agent smarter; we keep it from saying the wrong thing.
