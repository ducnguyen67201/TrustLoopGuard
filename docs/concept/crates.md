# Crates

The workspace has 13 Rust crates plus example apps. Each crate exists because
something concrete ships from it. None are "utility bag" crates. Read them in
this order — it follows the dependency graph from the bottom up.

## Dependency graph

```
                                                tl-cli      tl-server      tl-sdk-rust
                                                  │            │                │
              ┌───────────────────────────────────┼────────────┤                │
              ▼                                   ▼            ▼                │
        tl-policy                            tl-engine ───► tl-llm              │
              │                                ▲    │                           │
              │                                │    └──► tl-fuzzy               │
              │                                │    └──► tl-cache               │
              │                                │    └──► tl-storage             │
              │                                │                                │
              │                            tl-stream     tl-replay              │
              │                                │             │                  │
              └────────────────────────────────┼─────────────┘                  │
                                               ▼                                ▼
                                           tl-core ◄────────────────────────────┘
```

`tl-core` is at the bottom. Everything depends on it. `tl-core` depends on nothing of ours. Three crates landed after the initial 9 — `tl-fuzzy`, `tl-llm`, `tl-cache` — each because the engine reaches a tier where it needs new vocabulary, and that vocabulary either ships from its own crate or pollutes `tl-engine`.

---

## `tl-core` — the type backbone

**Files:** [`crates/tl-core/src/`](../../crates/tl-core/src/)

Pure data types. No I/O, no async, no business logic. If a type appears
in more than one crate, it lives here.

`tl-core` is also the only home for public HTTP contract DTOs. Any
request, response, or schema type that appears in OpenAPI must be defined
here, then imported by `tl-server`. This keeps Rust, OpenAPI, Python, and
TypeScript on one source of truth.

**Exports:**
- `GuardEvent` — what the customer sends in for runtime decisions
- `Decision` — what TrustLoopGuard sends back
- `EventKind` — dotted event taxonomy such as `output.proposed` and `tool.call.proposed`
- `Principal`, `Action`, `SideEffectClass` — event identity and proposed operation vocabulary
- `Source`, `Labels`, `ProvenanceMap` — source attribution and data classification vocabulary
- `ToolMetadata` — tool side-effect, reversibility, parameter-role, and approval metadata
- `AgentListResponse` — `GET /v1/agents` response
- `PolicyValidateResponse` — `POST /v1/policies/validate` response
- `PolicyValidationIssue` — one policy authoring parse/validation error
- `Verdict` — the four possible outcomes (`Allow`, `Block`, `Rewrite`, `Escalate`)
- `Channel` — `Voice`, `Chat`, `Email`
- `Severity` — `Low`, `Medium`, `High`, `Critical`
- `TriggeredPolicy` — record of which policies fired and why
- `RunnerDispatch`, `RunnerHandle`, `RunnerReport`, `RunnerAttack`, `RunnerStatus` — the private
  red-team **runner wire contract** (TrustLoopGuard → HackAgentOrchestration via `REDTEAM_RUNNER_URL`).
  Source of truth for the runner's generated Pydantic models; `tl-codegen` emits
  `docs/contracts/redteam-runner.schema.json` from these. Not part of the served OpenAPI.
- `TlError` — top-level error enum
- `new_trace_id()` — UUIDv4 helper

**Why it's its own crate:** so the SDK, server, engine, and storage layers can all share types without forcing the SDK consumer to pull in axum or Diesel.

**How it grows:** when a new field is needed across crates, add it here.
When a new endpoint needs a public request/response DTO, add that DTO here
first and import it from the server route. Never put I/O or route logic in
`tl-core`; methods are only acceptable when they operate on the core type
itself.

---

## `tl-policy` — the policy DSL

**Files:** [`crates/tl-policy/src/`](../../crates/tl-policy/src/)

Parses YAML policy files into a typed AST. That's the whole job.

**Exports:**
- `Policy` — one rule, with id, description, when-clause, match clause, action, optional rewrite
- `MatchClause` — `Single(Matcher) | Any { any: [...] } | All { all: [...] }`
- `Matcher` — `Regex(String) | Literal(String) | Semantic(String)`
- `Action` — what the policy wants if it triggers
- `load_str(yaml)` — parse one policy from a string
- `PolicyError` — parse / validation errors

**Example input:**
```yaml
id: refund-promise
description: Prevents unsupported refund promises.
when:
  channels: [chat]
  domains: [customer_support]
  agents: [acme-support-v3]
match:
  any:
    - regex: "(?i)\\b(refund|guarantee)\\b"
    - literal: "I promise"
action: rewrite
rewrite: "I'll connect you with a teammate."
severity: high
```

**Why it's its own crate:** the CLI lints policies offline; the server loads them at boot; the replay tool re-runs them. All of them want the same parser, none of them want the engine.

**How it grows:** new matcher types (`pii`, `length`, `language`) become new variants of `Matcher`. New top-level features (e.g. policy versioning, A/B groups) become new fields on `Policy`.

---

## `tl-engine` — the hot path

**Files:** [`crates/tl-engine/src/`](../../crates/tl-engine/src/)

The decision engine. Given a `GuardEvent` plus enabled policies and checker
configuration, returns a `Decision`. **This is the moat.**

**Exports:**
- `Engine::new(policies)` — build an engine from a policy set
- `Engine::empty()` — engine with no policies (always `Allow`)
- `Engine::check(&req) -> Decision` — the one function customers transitively call
- `Engine::check_async_with_policies(&req, ctx, policies) -> Decision` — full runtime path with deterministic, fuzzy, and LLM tiers

**Internal:**
- `engine.rs` — public `Engine` entry points
- `pipeline/` — orchestration, cancellation, cache scope, and tier runners
- `event_pipeline/` — event-stage traits for metadata resolution, label resolution, provenance propagation, checkers, signals, and decision composition
- `tiers/` — deterministic, fuzzy, and LLM tier execution
- `context/` — handler context and resolver traits
- `engine_match::policy_matches` — runs the matcher graph against `proposed_output`

**Why it's its own crate:** so embedded users can pull this without a server. So benchmarks can target it without HTTP overhead. So the unit-of-work that needs to be fast is isolated and measurable.

**Performance posture:** every change to this crate is a latency-sensitive change. Run `criterion` benches before merging anything that touches the hot path. No `Box<dyn ...>` in the inner loop without a bench-justified reason.

**How it grows:** new event-engine concerns plug into `event_pipeline/` through
explicit stage traits. Real stage implementations must preserve the synchronous
deterministic hot path unless a benchmark proves the cost is acceptable.

---

## `tl-stream` — incremental checks

**File:** [`crates/tl-stream/src/lib.rs`](../../crates/tl-stream/src/lib.rs)

For token-by-token text agents: feed chunks in, get `Continue` or `Interrupt` out the moment a block fires.

**Exports:**
- `StreamingChecker` — stateful buffer with a sliding window
- `StreamDecision` — `Continue | Interrupt { verdict, reason }`

**Why it's its own crate:** streaming has different state semantics from sync `check()`. Mixing them complicates `tl-engine`. Keeping them separate keeps the sync path uncluttered.

**How it grows:** integrations with LiveKit / Pipecat / Vapi / Retell live in `examples/`, not here. This crate stays provider-neutral.

---

## `tl-server` — the HTTP binary

**Files:** [`crates/tl-server/src/`](../../crates/tl-server/src/)

Axum server. The thing customers POST to in production.

**Routes:**
- `GET /health` — liveness
- `POST /v1/events` — the main runtime API

**Why it's its own crate:** binaries should be tiny glue. All the logic is in `tl-engine` and `tl-storage`; this crate just wires them to HTTP. Easy to swap for a gRPC variant later.

**Contract rule:** `tl-server` does not define public API DTOs. Route
modules may parse input, call stores/engines, and return JSON, but any
OpenAPI schema type must come from `tl-core`. CI enforces this with
`make lint-api-contracts`.

**Internal:**
- `app/` — router construction, OpenAPI registration, API errors, and middleware
- `api/` — thin HTTP handlers
- `services/` — request orchestration between API, engine, storage, and workers
- `state/` — app state, environment parsing, memory wiring, Postgres wiring, and storage adapters
- `gateway/` — gateway API, provider forwarding, normalization, credential sealing, and memory store
- `redteam/` — red-team dispatch orchestrator: job store trait, handlers, in-process worker, and attack-runner client. See [redteam-dispatch.md](redteam-dispatch.md).

**How it grows:** new endpoints (`/v1/decisions/:id`, `/v1/policies`,
`/v1/metrics`) get thin handlers under `api/` and any non-trivial workflow
goes into `services/`. Their public request/response structs are added to
`tl-core` first.

---

## `tl-sdk-rust` — the Rust client

**File:** [`crates/tl-sdk-rust/src/lib.rs`](../../crates/tl-sdk-rust/src/lib.rs)

Async HTTP client over `reqwest`. Wraps `POST /v1/events` so customers don't hand-roll JSON.

**Exports:**
- `Client::new(base_url)` — build a client
- `Client::with_api_key(key)` — attach bearer auth
- `Client::submit_event(&event).await` — the low-level runtime method customers call

**Why it's its own crate:** SDK consumers don't want to compile `axum` or `tokio::net` — only `reqwest` and `tl-core`. Keeping the dep surface small matters for adoption.

**How it grows:** retries, timeouts, fail-open vs fail-closed config, local short-circuit on connection loss. None of that exists yet.

---

## `tl-storage` — durable state

**Files:** [`crates/tl-storage/src/`](../../crates/tl-storage/src/)

Persists runtime decisions and cloud-authored configuration so they can be
queried, audited, replayed, and loaded by the server.

**Exports:**
- `DecisionStore` — async trait: `put` and `get`
- `MemoryStore` — `HashMap`-backed in-process implementation, useful for tests and local dev
- `PostgresStore` — Postgres-backed decision log implementation
- `AgentRepo` — Postgres-backed agent profile repository
- `PolicyRepo` — Postgres-backed policy repository; stores source YAML plus parsed JSONB and supports enabled/disabled runtime loading
- `HumanReviewRepo` — Postgres-backed append-only review-event repository and human review analytics aggregator
- `TeamRepo` — Postgres-backed workspace members + invites; see [team-and-invites.md](team-and-invites.md)
- `RedteamJobRepo` — Postgres-backed red-team job + per-attack result repository; see [redteam-dispatch.md](redteam-dispatch.md)
- `StorageError`

**Why it's its own crate:** the storage backend is the most likely thing to change (memory → Postgres → Postgres + ClickHouse). Trait-first design means the engine and server never know which one is plugged in.

**How it grows:** add repository types here when the server needs durable
state. Keep parsing/validation in the owning domain crate (`tl-policy` for
policies); storage accepts typed data and persists it.

---

## `tl-replay` — what-if analysis

**File:** [`crates/tl-replay/src/lib.rs`](../../crates/tl-replay/src/lib.rs)

Re-runs stored decisions against a new engine snapshot. Lets customers tune policies confidently — "if I add policy X, what would have happened to yesterday's traffic?"

**Exports:**
- `ReplayDiff` — verdict before/after, plus a `changed: bool`
- `diff(original, replayed)` — pure diff helper
- `replay_against(engine, original, request)` — run the new engine, diff the result

**Why it's its own crate:** replay is a separate workflow with its own UI surfaces (CLI today, dashboard later). Keeping it apart from `tl-engine` keeps the engine focused on the live path.

**How it grows:** batch replay over a date range. Stratified sampling (replay 1% of all traffic). Per-policy lift reports.

---

## `tl-cli` — operator command line

**File:** [`crates/tl-cli/src/main.rs`](../../crates/tl-cli/src/main.rs)

The `tl` binary. Policy commands include:

- `tl policy validate <path>` — validate local policy YAML.
- `tl policy push <path> --url <server>` — publish local YAML to the policy API.
- `tl policy pull <id> --output <path> --url <server>` — write the saved cloud YAML to disk.

`tl policy-lint <path>` remains as a legacy validation alias.

**Why it's its own crate:** binaries get their own crate. CLI dependencies (clap) shouldn't be pulled in by the server.

**How it grows:** `tl replay`, `tl policy diff`, `tl decisions list`. Each one is a new clap subcommand that calls into the relevant library crate.

---

---

## `tl-codegen` — derived-artifact generator

**File:** [`crates/tl-codegen/src/main.rs`](../../crates/tl-codegen/src/main.rs)

A build-time binary that reads `tl-core` types (with `schemars` / `utoipa` / `ts-rs` derives enabled) and writes:

- `docs/openapi.yaml` — OpenAPI 3.1 spec
- `policies/guard-event.schema.json` and `policies/decision.schema.json` — JSON Schema for editor autocomplete and dashboard validation
- `sdks/typescript/src/generated/*.ts` — TypeScript types

**Usage:**
```bash
cargo run -p tl-codegen           # write
cargo run -p tl-codegen -- --check # CI mode: fail on drift
```

**Why it's its own crate:** the codegen-time deps (`schemars`, `utoipa`, `ts-rs`) shouldn't pollute the SDK or server. Putting them behind `tl-core`'s `codegen` feature flag and only enabling them here keeps the runtime crates lean.

**How it grows:** when `tl-policy::Policy` gains the `JsonSchema` derive, this crate emits `policies/policy.schema.json`. When the TS SDK lands, this crate writes its types. When Pydantic models are needed for Python, run `datamodel-code-generator` against the JSON Schema this crate emits.

---

## `tl-fuzzy` — embedder + HNSW + Levenshtein

**Files:** [`crates/tl-fuzzy/src/`](../../crates/tl-fuzzy/src/)

The Tier 2 primitives. Three small pieces:

- **`Embedder` trait** + `MockEmbedder` (word-bag, deterministic, no I/O) — always built.
- **`FastEmbedder`** — real BGE-small (384-dim) via fastembed-rs + ONNX. Behind `--features fastembed` so the default build doesn't pull in ~100 MB of model weights.
- **`HnswIndex`** — labelled-vector cosine kNN over `hnsw_rs`. Sub-ms query for medium pattern sets.
- **`fuzzy_contains` / `distance`** — `strsim` wrappers for typo-bypass detection (`refund` → `refunddd` / `r3fund`).

**Why it's its own crate:** the heavy ML deps (`fastembed`, ONNX runtime) shouldn't bleed into every tier of the engine. Gating them here means `tl-engine` builds in seconds when Tier 2 is mocked.

**How it grows:** new embedder backends become new structs implementing `Embedder` (`OpenAIEmbedder`, `LocalCandleEmbedder`). HNSW tuning lives here too.

---

## `tl-llm` — LLM provider clients + router

**Files:** [`crates/tl-llm/src/`](../../crates/tl-llm/src/)

The Tier 3 surface. Two layers:

1. **`LlmClient` trait** with concrete implementations: `OpenAiClient`, `OpenRouterClient`. Both speak OpenAI-compatible chat completions with `response_format: { json_schema, strict: true }`.
2. **`LlmRouter`** — the *chokepoint*. Routes by `JudgeKind` (Hallucination, Tone, Authority), handles primary/fallback failover, enforces per-tenant `TokenBudget`, emits structured tracing fields per call. Tier 3 calls one method: `router.judge(kind, tenant, prompt, schema)`.

Routing is configured in TOML (`config/llm-routing.toml` is the canonical example).

**Why it's its own crate:** keeping LLM transport, prompts, and the router together means swapping providers or adding a third never touches `tl-engine`. The trait is the seam.

**How it grows:** new providers (Anthropic, Cohere, local llama.cpp) become new `LlmClient` impls. Per-tenant BYOK lands as a `tenant:*` provider kind in the config schema.

---

## `tl-cache` — decision cache

**Files:** [`crates/tl-cache/src/`](../../crates/tl-cache/src/)

The Moka-backed in-process decision cache plus the BLAKE3 key derivation. Two files:

- **`MokaCache`** — `moka::future::Cache<String, Decision>` with TTL (default 5 min) and bounded entry count (10 K). `disabled()` constructor for tests and "no caching wanted" deploys.
- **`for_check_request`** — canonical-JSON-then-BLAKE3 over `(domain, agent_id, input, proposed_output, context)`. Trace IDs deliberately don't affect the key so retries hit.

**Why it's its own crate:** the engine *needs* a cache, but Moka isn't the only candidate (Redis when we have multiple replicas; in-memory LRU for embedded users). Putting it behind a thin crate means swapping is mechanical.

**How it grows:** a Redis-backed impl behind the same shape lands when we go multi-replica. The trait stays; only the constructor changes.

---

## Current Boundary Decisions

- `tl-cache` stays independent because it owns cache key derivation and the
  Moka-backed implementation without forcing cache dependencies into `tl-core`
  or storage.
- `tl-fuzzy` stays independent because it owns HNSW, edit distance, and optional
  embedder dependencies that should not make the deterministic engine path
  heavier.
- `tl-llm` stays independent because provider clients, routing config, and
  live-provider test gates are separate from engine orchestration.
- `tl-stream` stays independent because incremental stream state is a different
  runtime surface from one-shot checks.
- `tl-replay` stays independent because replay is an offline workflow that
  depends on the engine and storage rather than belonging inside either one.

---

## Adding a new crate

Don't, until you have to. The bar is: **something concrete ships from it that doesn't ship from any other crate.** A new client SDK is a new crate. A new transport (gRPC) is a new crate. A new shared utility module is **not** a new crate — it's a module inside an existing one.

When you do add one:
1. Add a member entry to root [`Cargo.toml`](../../Cargo.toml).
2. Add it to the dependency graph in this file.
3. Add a section here describing why it exists, what it exports, and how it grows.
4. Write at least one test in the new crate before merging.
