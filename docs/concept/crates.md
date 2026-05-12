# Crates

The workspace has 9 crates. Each one exists because something concrete ships from it. None are "utility bag" crates. Read them in this order — it follows the dependency graph from the bottom up.

## Dependency graph

```
                    tl-cli         tl-server         tl-sdk-rust
                       │              │                   │
                       ▼              ▼                   │
                   tl-policy      tl-storage              │
                       │     ┌──┐     │                   │
                       │     │tl-replay│                  │
                       ▼     ▼  ▼      ▼                   │
                   tl-engine ◄─────────┐                   │
                       │               │                   │
                       ▼               │                   │
                   tl-stream ──────────┘                   │
                       │                                    │
                       ▼                                    ▼
                   tl-core ◄─────────────────────────────────┘
```

`tl-core` is at the bottom. Everything depends on it. `tl-core` depends on nothing of ours.

---

## `tl-core` — the type backbone

**File:** [`crates/tl-core/src/lib.rs`](../../crates/tl-core/src/lib.rs)

Pure data types. No I/O, no async, no logic. If a type appears in more than one crate, it lives here.

**Exports:**
- `CheckRequest` — what the customer sends in
- `Decision` — what TrustLoopGuard sends back
- `Verdict` — the four possible outcomes (`Allow`, `Block`, `Rewrite`, `Escalate`)
- `Channel` — `Voice`, `Chat`, `Email`, `Other(String)`
- `Severity` — `Low`, `Medium`, `High`, `Critical`
- `TriggeredPolicy` — record of which policies fired and why
- `TlError` — top-level error enum
- `new_trace_id()` — UUIDv4 helper

**Why it's its own crate:** so the SDK, server, engine, and storage layers can all share types without forcing the SDK consumer to pull in axum or sqlx.

**How it grows:** when a new field is needed across crates, add it here. Never put logic in `tl-core` unless it operates only on its own types.

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
  channels: [voice, chat]
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

The synchronous decision engine. Given a `CheckRequest` and a set of `Policy` values, returns a `Decision`. **This is the moat.**

**Exports:**
- `Engine::new(policies)` — build an engine from a policy set
- `Engine::empty()` — engine with no policies (always `Allow`)
- `Engine::check(&req) -> Decision` — the one function customers transitively call

**Internal:**
- `engine_match::policy_matches` — runs the matcher graph against `proposed_output`

**Why it's its own crate:** so embedded users can pull this without a server. So benchmarks can target it without HTTP overhead. So the unit-of-work that needs to be fast is isolated and measurable.

**Performance posture:** every change to this crate is a latency-sensitive change. Run `criterion` benches before merging anything that touches the hot path. No `Box<dyn ...>` in the inner loop without a bench-justified reason.

**How it grows:** Layer 2 classifiers (ONNX/candle) and Layer 3 remote LLM judges become async sister methods (`Engine::check_async`) that compose with the sync hot path, never replace it.

---

## `tl-stream` — incremental checks

**File:** [`crates/tl-stream/src/lib.rs`](../../crates/tl-stream/src/lib.rs)

For voice and token-by-token text agents: feed chunks in, get `Continue` or `Interrupt` out the moment a block fires.

**Exports:**
- `StreamingChecker` — stateful buffer with a sliding window
- `StreamDecision` — `Continue | Interrupt { verdict, reason }`

**Why it's its own crate:** streaming has different state semantics from sync `check()`. Mixing them complicates `tl-engine`. Keeping them separate keeps the sync path uncluttered.

**How it grows:** integrations with LiveKit / Pipecat / Vapi / Retell live in `examples/`, not here. This crate stays provider-neutral.

---

## `tl-server` — the HTTP binary

**Files:** [`crates/tl-server/src/main.rs`](../../crates/tl-server/src/main.rs)

Axum server. The thing customers POST to in production.

**Routes:**
- `GET /health` — liveness
- `POST /v1/check` — the main API

**Why it's its own crate:** binaries should be tiny glue. All the logic is in `tl-engine` and `tl-storage`; this crate just wires them to HTTP. Easy to swap for a gRPC variant later.

**How it grows:** new endpoints (`/v1/decisions/:id`, `/v1/policies`, `/v1/metrics`) get their own handler functions and one line each in the `Router::new()` call. No file-based magic — see the routing question in onboarding.

---

## `tl-sdk-rust` — the Rust client

**File:** [`crates/tl-sdk-rust/src/lib.rs`](../../crates/tl-sdk-rust/src/lib.rs)

Async HTTP client over `reqwest`. Wraps `POST /v1/check` so customers don't hand-roll JSON.

**Exports:**
- `Client::new(base_url)` — build a client
- `Client::with_api_key(key)` — attach bearer auth
- `Client::check(&req).await` — the one method customers call

**Why it's its own crate:** SDK consumers don't want to compile `axum` or `tokio::net` — only `reqwest` and `tl-core`. Keeping the dep surface small matters for adoption.

**How it grows:** retries, timeouts, fail-open vs fail-closed config, local short-circuit on connection loss. None of that exists yet.

---

## `tl-storage` — decision log

**Files:** [`crates/tl-storage/src/`](../../crates/tl-storage/src/)

Persists `Decision`s so they can be queried, audited, and replayed.

**Exports:**
- `DecisionStore` — async trait: `put` and `get`
- `MemoryStore` — `HashMap`-backed in-process implementation, useful for tests and local dev
- `StorageError`

**Why it's its own crate:** the storage backend is the most likely thing to change (memory → Postgres → Postgres + ClickHouse). Trait-first design means the engine and server never know which one is plugged in.

**How it grows:** add a `PostgresStore` impl that satisfies `DecisionStore`. Server boots the right one based on config. No engine change.

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

The `tl` binary. Single subcommand today: `tl policy-lint <path>`. Validates a policy YAML file.

**Why it's its own crate:** binaries get their own crate. CLI dependencies (clap) shouldn't be pulled in by the server.

**How it grows:** `tl replay`, `tl bench`, `tl policy diff`, `tl decisions list`. Each one is a new clap subcommand that calls into the relevant library crate.

---

---

## `tl-codegen` — derived-artifact generator

**File:** [`crates/tl-codegen/src/main.rs`](../../crates/tl-codegen/src/main.rs)

A build-time binary that reads `tl-core` types (with `schemars` / `utoipa` / `ts-rs` derives enabled) and writes:

- `docs/openapi.yaml` — OpenAPI 3.1 spec
- `policies/check-request.schema.json` and `policies/decision.schema.json` — JSON Schema for editor autocomplete and dashboard validation
- `sdks/typescript/src/generated/*.ts` — TypeScript types

**Usage:**
```bash
cargo run -p tl-codegen           # write
cargo run -p tl-codegen -- --check # CI mode: fail on drift
```

**Why it's its own crate:** the codegen-time deps (`schemars`, `utoipa`, `ts-rs`) shouldn't pollute the SDK or server. Putting them behind `tl-core`'s `codegen` feature flag and only enabling them here keeps the runtime crates lean.

**How it grows:** when `tl-policy::Policy` gains the `JsonSchema` derive, this crate emits `policies/policy.schema.json`. When the TS SDK lands, this crate writes its types. When Pydantic models are needed for Python, run `datamodel-code-generator` against the JSON Schema this crate emits.

---

## Adding a new crate

Don't, until you have to. The bar is: **something concrete ships from it that doesn't ship from any other crate.** A new client SDK is a new crate. A new transport (gRPC) is a new crate. A new shared utility module is **not** a new crate — it's a module inside an existing one.

When you do add one:
1. Add a member entry to root [`Cargo.toml`](../../Cargo.toml).
2. Add it to the dependency graph in this file.
3. Add a section here describing why it exists, what it exports, and how it grows.
4. Write at least one test in the new crate before merging.
