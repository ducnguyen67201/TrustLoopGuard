# Event Engine

The event engine is the Rust-owned contract for deciding whether a proposed agent step may happen. It keeps TrustLoopGuard SDK-first: customer runtimes call the SDKs, SDKs call Rust, and every adapter converges on the same `tl-core` vocabulary.

## Ownership

| Surface | Owner | Responsibility |
|---|---|---|
| Event and decision wire types | `crates/tl-core` | Defines `GuardEvent`, `EventKind`, labels, provenance, tool metadata, and additive `Decision` evidence fields. |
| Runtime evaluation seams | `crates/tl-engine` | Normalizes compatibility requests, resolves event context, runs checks, composes decisions, and exposes no-op stage traits. |
| HTTP entry point | `crates/tl-server` | Accepts `/v1/check`, resolves workspace/environment, applies redaction policy, loads enabled policies, and returns a `Decision`. |
| Trace persistence | `crates/tl-storage` | Persists decision traces through the existing trace writer. |
| Tool metadata registry | `crates/tl-storage` | Durable workspace-scoped `tool_metadata` table behind the cached `ToolMetadataRepo`. |
| Source label policies | `crates/tl-storage` | Durable workspace-scoped `source_label_policy` table behind the cached `SourceLabelPolicyRepo`. |

`apps/web` may display traces and call same-origin proxy routes, but it does not own event-engine contracts, runtime checks, or trace storage.

## Contract Vocabulary

`CheckRequest` is the public `/v1/check` compatibility request. `GuardEvent` is the normalized event shape that SDKs, gateway code, and host adapters can share internally.

A `GuardEvent` contains:

- `kind` - the dotted event taxonomy, such as `output.proposed`, `tool.call.proposed`, or `database.mutation.proposed`.
- `principal` - resolved workspace, environment, agent, user/session/task, and optional run/run-event identity.
- `action` - the operation being proposed, its parameters, and the side-effect class.
- `sources` - inputs that influenced the proposed step, with origin and labels.
- `provenance` - a map from output or parameter paths to source ids.
- `resolution` - registry resolution evidence attached by the pipeline: `resolved` with the matched tool metadata, `unregistered`, or `resolution_failed` when the registry lookup itself errored.
- `label_resolution` - label evidence attached by the pipeline: per-source resolved labels with a basis (`origin_default`, `workspace_override`, or `declared`), derived labels per provenance path, and the policy read status.
- `signals` - advisory signal evidence attached by the pipeline: one entry per LLM/classifier signal with provider id, message, and optional severity. Server-populated and never decision-bearing.
- `context` - caller-supplied JSON that travels with the event.

Tool metadata describes known tools independently of a specific event: side-effect class, reversibility, parameter roles, allowed sources, approval requirements, and sandbox hints. On the wire, a registry row is a `ToolMetadataEntry` (the metadata plus its `enabled` flag).

`Decision` remains the result contract. Evidence fields such as `violated_rule`, `remediation`, `source_chain`, `risk_source`, `failure_mode`, `harm_class`, and `constraints` are optional and omitted when empty, so existing `/v1/check` callers keep the same response shape.

## Current Runtime Flow

```text
--------------------+        +-------------------------+
| SDK / gateway /    | -----> | POST /v1/check          |
| embedded caller    |        | CheckRequest            |
+--------------------+        +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Server redaction, auth, |
                              | workspace/environment  |
                              +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Existing parallel tier |
                              | orchestrator           |
                              | -> Decision            |
                              +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Event pipeline         |
                              | GuardEvent-only input  |
                              | action resolution +    |
                              | label resolution +     |
                              | provenance propagation |
                              | mode-gated checkers +  |
                              | decision composition   |
                              +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Decision JSON + async  |
                              | trace with event       |
                              | evidence               |
                              +-------------------------+
```

Every `/v1/check` request routes through the event pipeline (`tl-engine::event_pipeline`), and `POST /v1/events` enters it directly with a caller-built `GuardEvent` (see Collection Points below). The pipeline contract is `GuardEvent`-only: collectors translate their raw traffic into a `GuardEvent` before entering it. Legacy `/v1/check` requests are translated by a standalone compatibility adapter (`legacy_check_to_event`, slated for removal once direct event ingestion is the only entry point) into `GuardEvent { kind: output.proposed, action.operation: "output", ... }`. Events pass through the pipeline with their sources and provenance preserved verbatim; the pipeline always overwrites the principal's workspace and environment with server-resolved values so callers cannot spoof workspace identity.

The pipeline is observe-only by default: checker enforcement modes default to `off` per workspace, missing evidence never blocks, and no blocking I/O joins the decision path. Live stages: `ToolMetadataProvider` resolves `action.operation` against the workspace tool metadata registry, `LabelResolver` resolves source labels against built-in defaults and workspace label policies, `ProvenanceResolver` derives per-path labels over the provenance map, and three deterministic checkers evaluate the resolved event under per-workspace enforcement modes (see Checkers And Enforcement Modes). For a workspace with all modes `off`, the normalized event's only effect is trace enrichment and the decision passes through unchanged.

## Tool Metadata Registry and Action Resolution

The registry gives actions structured semantics before checkers rely on them: a tool name alone cannot decide safety.

- **Storage.** Workspace-scoped `tool_metadata` table (primary key `(workspace_id, tool)`), owned by `tl-storage::ToolMetadataRepo`. The full `ToolMetadata` lives in a `spec JSONB` column with `side_effect`/`reversible` promoted for queries; rows soft-delete via `deleted_at` and carry an `enabled` flag.
- **Caching.** Repo reads go through a moka cache (1K entries, 60s TTL) that also caches misses, so unregistered tools — the common case on the per-event hot path — never become repeated Postgres round trips.
- **Control plane.** `POST/GET /v1/tool-metadata` and `GET/DELETE /v1/tool-metadata/{tool}` manage entries. Upserting with `enabled: false` keeps a tool manageable while hiding it from runtime resolution; CRUD reads still return disabled rows.
- **Runtime resolution.** The pipeline resolves `action.operation` through the provider seam. A registered, enabled tool attaches `resolution: resolved` with its metadata and overwrites `action.side_effect` with the registry value — the registry is authoritative over collector-claimed side effects, and later pipeline stages (checkers, signal providers) see the resolved event. Unknown or disabled tools resolve as `resolution: unregistered`: conservative evidence, never a gate. A registry outage is recorded as `resolution: resolution_failed` — distinct from absence so traces stay accurate forensic evidence — and fails open with a warning; the collector-claimed side effect survives unchanged.

No checker blocks because metadata exists or is missing. The information-flow and parameter-authorization checkers consume the resolved side effect and parameter specs (see Checkers And Enforcement Modes); sandbox enforcement is a future consumer.

## Label Resolution And Provenance Propagation

Label resolution makes event evidence legible: every source gets deterministic trust/confidentiality/integrity labels, and provenance propagation derives labels for each parameter path. Both stages are evidence-only — no verdict changes because of labels.

- **Built-in origin defaults.** User and system sources are the only trusted ones (trusted/private/high). Everything that enters from outside the operator's control — tool output, memory, files, web, email, external APIs — is untrusted with low integrity. Web content defaults to public confidentiality; tool output to unknown. An unknown origin resolves to untrusted with unknown confidentiality and integrity: conservative evidence for later enforcement phases.
- **Workspace overrides.** The `source_label_policy` table (primary key `(workspace_id, origin)`, owned by `tl-storage::SourceLabelPolicyRepo`) stores per-origin overrides; each row may set any subset of the three families. `POST/GET /v1/label-policies` and `GET/DELETE /v1/label-policies/{origin}` manage rows; disabled rows stay manageable but are skipped at runtime. Repo reads go through a moka cache (1K workspaces, 60s TTL) keyed by workspace — the runtime read is list-shaped, and an empty list is cached too, so workspaces without policies stay off Postgres.
- **Per-family precedence.** For each source, each of the three label families (trust, confidentiality, integrity) is resolved independently through the same cascade — the first level with an opinion wins:

  ```text
  1. Declared      — the collector set this family on the source
                     (source.labels.X != unknown; unknown, the serde
                     default, means "not declared").
  2. Workspace     — an enabled source label policy row exists for this
     override        source's origin and sets this family. Rows with
                     enabled=false stay manageable in the control plane
                     but are invisible here.
  3. Origin        — the built-in default table for the source's origin
     default         (user/system trusted; everything external untrusted).
  ```

  Each resolved family records which level won as its basis (`declared`, `workspace_override`, `origin_default`), so traces show why a source was trusted, untrusted, or private — never just the value. Resolved labels are written back onto the event source: later stages see resolved values, mirroring how the registry side effect overwrites the collector-claimed one. Because the cascade is per family, a policy row that sets only confidentiality leaves trust and integrity on their origin defaults, and a collector that declares only trust leaves the other two to the override/default levels.
- **Propagation.** For each path in the provenance map, the derived labels fold the resolved labels of every referenced source: any untrusted contributor makes the path untrusted; the highest confidentiality claim wins (unknown outranks public only); integrity is the weakest contributor, and any unknown poisons the path to unknown. A source id with no matching event source contributes all-unknown, and a path with no provenance entry gets no derived value — missing provenance is unknown, never clean.
- **Fail open.** If the policy store cannot be consulted, resolution applies built-in defaults and records `policy_status: unavailable` — distinct from `not_configured`, so a storage outage never masquerades as "no overrides exist". The decision is unaffected.

Classifier and LLM signals remain advisory and are not part of label resolution. The checkers consume this evidence (see Checkers And Enforcement Modes); cross-session memory tracking is a future consumer.

## Checkers And Enforcement Modes

Checkers are deterministic, in-process, pure functions over the resolved event: no I/O, no clock, no LLM. They read what earlier stages resolved — registry side effects, source labels, derived path labels, provenance — and emit findings. A finding names the violated rule, a recommended verdict, the offending source chain, and forensic fields (`risk_source`, `failure_mode`, `harm_class`).

Four checkers exist:

- **`information_flow`** — applies to high-impact side effects (external communication, publish, network call, file write, shell exec, db/api mutation). Rules: `destination-permission` blocks sensitive data (private/secret/identity confidentiality, on contributing sources or derived path labels) flowing to external sinks; `action-integrity` blocks high-impact actions controlled by untrusted sources; `missing-provenance` escalates conservatively when control provenance is absent, dangling, or unknown-trust.
- **`memory`** — applies to memory writes (`memory.write.proposed` or a `memory_write` side effect). `memory-write-untrusted` blocks untrusted content becoming authority-bearing memory; `memory-write-unverified` escalates writes with unverifiable provenance. Retrieval-time, cross-session memory analysis is deliberately not implemented.
- **`parameter_auth`** — applies to events whose tool resolved against the registry. `parameter_source.{path}` requires every authority-bearing parameter to carry provenance whose sources all match the tool's `allowed_sources`; wrong source blocks, missing proof escalates. Unregistered tools on action events yield verdict-free evidence only.
- **`approval`** — applies to events whose tool resolved against the registry with an approval rule. `approval.{tool}` escalates when the registry's `ApprovalRule` marks the tool as requiring human approval; the remediation names the registry reason or the approver roles. Unregistered or rule-free tools emit nothing — unknown-tool conservatism belongs to the flow and parameter checkers.

Each checker runs under an **enforcement mode** scoped by workspace and environment. Workspace-level modes live in `workspace_settings` (`flow_checker_mode`, `memory_checker_mode`, `param_checker_mode`, `approval_checker_mode`, all `TEXT` with CHECK constraints, default `'off'`); per-environment overrides live in `environment_checker_modes` (same four columns, nullable — `NULL` inherits the workspace mode, so an override row can tighten or loosen individual checkers without restating the rest). The service layer resolves the effective mode per request: environment override wins per checker, and an override-lookup failure falls back to workspace modes so rollout configuration can never take the hot path down. Operators manage rollout through `GET`/`PATCH /v1/settings` (partial update, absent fields unchanged) and `GET`/`PUT /v1/environments/{environment_id}/checker-modes` (PUT replaces the override row; unknown environments return 404). Mode writes require a workspace Owner/Admin user and reject workspace runtime keys outright — a running agent must never be able to weaken the controls that govern it:

| Mode | Evaluation | Decision effect | Trace effect |
|---|---|---|---|
| `off` (default) | none | none | none |
| `shadow` | yes | none | full hypothetical evidence |
| `enforce` | yes | worst verdict wins | full evidence |

Mode handling lives in the pipeline, not the checkers: a checker in `off` mode is never invoked; in `shadow` mode its findings are recorded as evidence and their verdicts stripped before composition; in `enforce` mode verdicts stand. The `ModeAwareDecisionComposer` folds enforced findings into the decision — worst verdict wins (`block > escalate > rewrite > allow`), the winning finding's evidence fields are copied onto the `Decision`, and a seeded decision is never downgraded (an engine block survives). Signals never decide action safety: LLM/classifier output cannot override a deterministic checker in either direction.

Checker evidence persists on the event as `checks`: one `CheckerRun` per evaluated checker with the mode at evaluation time and the full findings including `recommended_verdict` — shadow traces show exactly what enforce would have decided. Advisory signals persist the same way as `signals`: one `SignalEvidence` per provider signal, trace-visible but never verdict-bearing. The pipeline resets both before evaluating, so collector-submitted evidence never survives, mirroring the workspace-identity overwrite.

The advisory signal path is **sheddable by contract**: the pipeline awaits the `SignalProvider` under a hard budget (`signal_budget`, default 250 ms) and continues without signals when the budget is exceeded, logging the shed. Deterministic checkers run before the signal call, so an over-budget provider costs trace evidence, never availability or safety — the deterministic core stays available under overload.

An `escalate` verdict — from a checker, a policy, or the tier engine — routes to the existing escalation worker on both entry points: `/v1/check` payloads carry the request's agent and domain, `/v1/events` payloads carry the event principal's agent and `domain: "event"`.

**Trust boundary.** Checkers evaluate resolved labels, and label resolution gives producer-declared values the highest precedence. Enforcement is therefore cooperative, not adversarial-resistant: a collector that declares its sources `trusted`/`public` neutralizes the flow and memory rules for its own events. This is the documented producer-reported-facts model — declarations are recorded as `basis: declared` in trace evidence, so they are auditable; the same applies to source origins, which the parameter-auth checker compares against the operator-owned registry. Enforcement against untrusted collectors requires the trusted-adapter path, where the SDK adapter — not the agent under guard — produces sources, origins, and labels.

## Collection Points

Each collection point translates raw runtime traffic into the same abstract `GuardEvent`. Fidelity differs by where the collector sits:

| Collection point | Fidelity | What it can see | What it cannot prove |
|---|---:|---|---|
| Legacy `/v1/check` | medium | input text, proposed output, agent/run identity | source labels, parameter provenance |
| Gateway proxy | low | model I/O, proposed tool calls, provider metadata | actual execution, parameter provenance |
| Direct ingestion (`POST /v1/events`) | as declared | whatever the producer collected: full sources, labels, provenance | the producer's claims (origin and provenance are producer-reported facts) |
| SDK adapter | high | the actual execution boundary | — |
| MCP proxy | medium | protocol-level tool requests and responses | host-side execution context |

### Gateway (low fidelity)

Gateway-proxied traffic reaches the check path as a `CheckRequest` whose context carries `integration_mode: "gateway"`. The normalizer records explicitly low-fidelity sources for it: `input.observed` and `model.output`, both `origin: unknown` with default labels. The gateway sees model I/O but cannot prove what actually executed, so its evidence is never upgraded beyond observed labels.

The context marker is caller-supplied and therefore untrusted. It only selects this lower-fidelity labeling — spoofing it downgrades the caller's own trace evidence and nothing else. It must never gate enforcement or elevate trust; when an enforcement phase needs authentic gateway identity, it derives it from server-authenticated principal context instead of the request body.

### Direct ingestion (observe-only)

`POST /v1/events` accepts the canonical `GuardEvent` verbatim — the entry point SDK adapters will use. The event runs through the same pipeline (action resolution, label resolution, provenance propagation, mode-gated checkers) and its evidence persists as a trace with `domain: "event"`. For a workspace with default settings the response verdict is always `allow` with the reason `observe-only: event recorded; checkers not yet enforcing`; a workspace that opts a checker into `enforce` receives live verdicts from the same endpoint with no contract change (the seeded reason is rewritten only when a finding changes the verdict). Submitted events are bounded (sources, provenance paths, payload bytes, session id length), run/run-event links are validated like `/v1/check`, and workspaces not in `raw_allowed` data-handling mode are rejected because event redaction does not exist yet. All three SDKs expose this as `submit_event`. No tier engine runs on this path and run check-stats are not recorded — ingested events are evidence, not checks.

The Rust SDK additionally supports opt-in monitoring sessions: `with_monitoring()` generates a session id (`sess_<uuid>`) that the client attaches to the principal of every outgoing check and event that does not already carry one, and `record_event` gives a fire-and-forget capture path (single attempt, failures logged, never blocking the caller). The session id is caller-reported metadata — opaque to the server, passed through the pipeline verbatim like the rest of the principal's identity fields, and never an enforcement input. See the glossary's "Monitoring session" entry.

### SDK adapter (high fidelity)

The SDK adapter is the full-fidelity product path. An adapter hooks the host framework's tool/function boundary and collects, before the function runs:

- operation name and parameters,
- source ids with origin and known labels,
- parameter-to-source provenance,
- run/session/task context,
- redaction state when applicable.

The adapter translates that into a `GuardEvent` and enters the pipeline directly, which preserves its sources and provenance verbatim. Core engine code never depends on host framework types (LangChain, OpenAI Agents SDK, LiveKit, MCP SDK); the adapter owns the translation.

### MCP proxy (medium fidelity)

An MCP boundary can collect tool server identity, tool call name and parameters, and response sources at protocol level. It enters through the same event-shaped path. No Rust collection point exists for MCP yet.

## Trace Evidence

The trace payload is the full `Decision` plus an additive `event` object when event evidence was collected. The `event` object carries `kind`, `principal`, `action`, `sources` (with resolved labels), `provenance`, `resolution`, `label_resolution`, `checks` (checker evaluations with mode and findings), and `signals` (advisory signal evidence); run/run-event links travel inside `principal`. Existing consumers parse the payload as a `Decision` and ignore the extra key, so enrichment is backward compatible.

Evidence stays in the JSON payload. An evidence field is promoted to a trace table column only when an API or dashboard filter requires it: `session_id` is promoted (indexed per workspace, filterable via `GET /v1/traces?session_id=`, surfaced on `TraceSummary`); `event_kind`, `risk_source`, `failure_mode`, and `harm_class` remain promotion candidates. Trace writing remains fire-and-forget: the request path uses a non-blocking enqueue, and a full queue drops the trace with a warning rather than delaying the decision.

## Stage Seams

The event pipeline exposes small trait seams so each concern can be implemented independently:

- `Normalizer` builds the canonical event.
- `PrincipalResolver` attaches workspace, environment, and identity context.
- `ToolMetadataProvider` resolves the operation against the workspace tool metadata registry (live since the registry shipped).
- `LabelResolver` attaches trust, confidentiality, and integrity labels (live: `PolicyLabelResolver` reads workspace label policies through the cached `LabelPolicyProvider` seam).
- `ProvenanceResolver` derives per-path labels from sources and the provenance map (live: `ProvenancePropagator`, pure and deterministic).
- `Checker` produces findings from the resolved event (live: `InformationFlowChecker`, `MemoryChecker`, `ParameterAuthChecker`, `ApprovalChecker`, each gated by its workspace enforcement mode).
- `SignalProvider` adds advisory signals; the pipeline records them as `signals` evidence on the event.
- `DecisionComposer` turns findings and signals into a `Decision` (live: `ModeAwareDecisionComposer`, worst verdict wins; signals never decide).
- `TracePersister` enqueues trace side effects.

The no-op context wires inert implementations; the server replaces `ToolMetadataProvider` with the registry-backed adapter, `LabelResolver`/`ProvenanceResolver` with the live label stages, and registers the checkers and mode-aware composer at boot. Default-off modes keep the customer-visible runtime unchanged until a workspace opts in.

## Compatibility Rules

- Old `CheckRequest` JSON must keep deserializing.
- Empty evidence on `Decision` must not appear in serialized `/v1/check` responses.
- `/v1/check` verdict, reason, policy, trace, run, redaction, cache, escalation, and latency semantics stay owned by the existing Rust runtime path.
- New SDK-visible capabilities start in `tl-core`, then flow through OpenAPI and generated SDK types.
- Durable event storage is introduced only when the owning Rust storage path and trace API are defined.
