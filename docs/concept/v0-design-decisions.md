# v0 Design Decisions

This document captures the design decisions made during the v0 architecture review. It extends [architecture.md](architecture.md) and [crates.md](crates.md), and supersedes them where noted.

**Status:** draft, awaiting confirmation. Code work has not started against these decisions.

**Audience:** engineers about to write code, and reviewers pushing back before they do.

---

## 1. Product boundary (locked)

> TrustLoopGuard returns a **Decision** and **reasons**. The caller is responsible for acting on the verdict, sending messages, managing queues, and handling fallback when we are unreachable.

We are a **judgment service, not an enforcement layer.** We never touch the customer. We never own the channel (chat, voice, email, SMS). We never maintain conversation state on the customer's behalf.

| What we do | What we don't do |
|---|---|
| Run policy checks against `(input, draft, context)` | Send the message |
| Return a `Decision` + structured `reasons` | Decide the canned safe response |
| Optionally return a `revised` draft | Rewrite drafts in-flight |
| Persist a trace for audit / replay | Maintain conversation state |
| Fire an escalation webhook | Own the human review queue |

**Why this boundary.** Integration is one function call. We stay off the critical path of message delivery. Our outage is not their outage — they pick fail-open or fail-closed. Liability for "what's an OK promise to make" stays with the customer, not us.

A future **proxy mode** (`enforce()`) is a real product but a different one. Defer until a paying customer asks.

---

## 2. The wedge (locked)

**Customer-facing conversation safety**, starting with `domain: "customer_support"`.

We will resist the temptation to ship a generic policy engine first. We commit to one domain, build the loop end-to-end, and only then expose `domain: "voice_agent"`, `domain: "coding_agent"`, etc. by adding handlers — not by re-architecting.

The contract supports universality (`domain` is a field). The implementation does not pretend to.

---

## 3. The check contract (locked)

The wire shape — once shipped, this does not change without a major version bump.

```
POST /v1/check
{
  "domain": "customer_support",
  "input": "...",                  // what the user said
  "draft": "...",                  // what the agent wants to send
  "context": {
    "docs":     [...],             // for grounding
    "customer": {...},             // optional state
    "agent_id": "...",
    "session_id": "..."
  },
  "policies": ["..."],             // optional override of default set
  "trace_id": "..."                // optional, for caller correlation
}

→
{
  "decision":   "allow" | "revise" | "block" | "escalate",
  "reasons":    [{ "policy", "severity", "message", "source" }, ...],
  "revised":    "..." | null,      // present when decision = "revise"
  "trace_id":   "...",             // UUIDv7
  "elapsed_ms": 42
}
```

Three of four decisions are pure judgment (caller acts):
- `allow` — send the draft
- `block` — don't send the draft (caller picks alternative)
- `escalate` — route to human (caller owns the queue)

One decision produces content:
- `revise` — `revised` field carries our suggested rewrite. Caller still chooses to use it.

---

## 4. Three-tier execution (decision: parallel with cancellation)

> **Supersedes** the earlier serial-cascade-with-short-circuit sketch. The current architecture overview describes the event-centered compatibility path and this parallel-cancel orchestrator.

The cascade order is correct (Tier 1 → Tier 2 → Tier 3). The execution pattern is **parallel with cancellation**, not serial-await-then-decide.

### Tier definitions

| Tier | Kind | Cost | What lives here |
|---|---|---|---|
| **1. Deterministic** | exact match, regex, lookup | <1ms total | stored policy matchers for banned phrases, regex sets, length/format guards, PII, white/blocklists |
| **2. Fuzzy** | embedding similarity, edit distance | 5-20ms | semantic-neighbor search vs known-bad patterns, Levenshtein to bypass attempts (`r3fund`, `refunddd`), perturbation detection |
| **3. LLM** | reasoning, grounding | 200-600ms | hallucination grounding, subtle promise detection, tone/escalation judgment, policy interpretation |

### Execution pattern

```
t=0       all three tiers start in parallel
t=0.5ms   Tier 1 completes
          ├─ block?  → cancel Tier 2+3, return immediately
          └─ no      → continue
t=15ms    Tier 2 completes
          ├─ block?  → cancel Tier 3, return
          └─ no      → wait for Tier 3
t=400ms   Tier 3 completes → aggregate → decision
```

**Why parallel-with-cancel beats serial cascade.** When Tier 1 blocks, both patterns return in <1ms. But on the *common path* where Tier 1 and Tier 2 both pass, parallel execution has Tier 3 already 15ms into its 400ms call by the time we'd otherwise be starting it. The thrown-away work happens in another task and does not extend the critical path.

### Implementation

```rust
let cancel = CancellationToken::new();
let t1 = tokio::spawn(tier1_deterministic(req.clone()));
let t2 = tokio::spawn(tier2_fuzzy(req.clone(), embedder.clone(), cancel.clone()));
let t3 = tokio::spawn(tier3_llm(req.clone(), llm.clone(), cancel.clone()));

let r1 = t1.await?;
if first_block(&r1).is_some() {
    cancel.cancel();
    return CheckResponse::block(...);
}

let r2 = t2.await?;
if first_block(&r2).is_some() {
    cancel.cancel();
    return CheckResponse::block(...);
}

let r3 = t3.await.unwrap_or_else(|_| Tier3::escalate("llm_timeout"));
aggregate(r1, r2, r3)
```

A new module — orchestration logic separate from policy logic — owns this pattern. Lives in `tl-engine` (extending the existing crate, not adding a new one).

---

## 5. Where rules come from (locked)

Policy sources are layered. More specific overrides general.

```
1. Customer policy bundle (loaded at boot)
   - YAML in their repo, parsed by tl-policy
   - their banned phrases, approved promise list,
     escalation triggers, tone targets, doc references
        +
2. Cloud policy definitions and environment deployment state
   - workspace-level policy definitions stored in Postgres
   - environment-scoped enablement controls which policies run
        +
3. Per-request context (in CheckRequest)
   - docs the agent grounded against
   - customer state (VIP, angry, churn-risk)
   - session history
```

There are no hardcoded runtime guardrails in the engine. If no stored or local policy is enabled for the resolved environment, `/v1/check` allows the request. New workspaces receive disabled starter policies for common PII and prompt-injection patterns; users opt into those policies through environment-scoped deployment state.

Per-request context is part of `CheckRequest.context`. Already in the contract.

---

## 6. Latency budget (committed)

These are reproduced from [architecture.md](architecture.md) for reference and remain the contract:

| Channel | Mode | p99 budget | What's allowed |
|---|---|---|---|
| Voice | streaming | < 50 ms | Tier 1 only |
| Chat | sync | < 150 ms | Tier 1 + Tier 2, optional fast Tier 3 |
| Email / async | sync | < 500 ms | All tiers |
| Replay / audit | offline | best-effort | All tiers, full LLM grading |

### Measured numbers (criterion)

Captured on Apple M-series under `cargo bench -p tl-engine --bench check_pipeline`. These are the engine-only numbers (no HTTP overhead); end-to-end RPS belongs to `loadtest/`.

| Scenario | Median |
|---|---|
| `check_sync_empty_policies` | **1.19 µs** |
| `check_async_empty_policies_stub_tiers` | **11.7 µs** |
| `check_async_50_policies_4kb_draft` | **23 µs** |
| `check_sync_empty_policies_4kb` | **6.1 µs** |
| `check_async_cache_hit_path` | **10.9 µs** |
| `check_sync_policy_block_4kb` | **17.8 µs** |

All medians are **at least 6 000× under the 150 ms chat budget**. The async stub path costs ~10 µs over the sync path — the cost of scheduling the Tier 2 + Tier 3 spawn + cancellation token + cache lookup.

End-to-end RPS via `loadtest/run.sh` against a real `tl-server` is the *next* number to lock — see `loadtest/README.md`. Recording the full HTTP-round-trip p95 belongs to a separate dated run.

**Outstanding:** competitor baseline. Until we publish a head-to-head, "we beat them 3–5×" remains an unmeasured claim.

---

## 7. Async work and queues (locked)

No Kafka, no RabbitMQ in v0. The `check()` hot path is direct function dispatch — no queue.

Async side effects use **in-process `tokio::sync::mpsc` channels**:

- **Trace writes**: HTTP handler `try_send`s the trace into a channel, returns immediately. A background task batches inserts (50 traces or 100ms, whichever first) into Postgres. If the channel is full, drop with a metric — better to lose a trace than to add latency.
- **Escalation webhooks**: same pattern. Channel + retry task.

Move to a Postgres outbox table when a single binary outgrows in-memory channels (durability across crashes). Move to Kafka only when we have multiple services consuming the same event stream — not before.

---

## 8. Scaling story (locked)

100k users ≠ 100k RPS. Translation:
- 100k DAU, ~10 checks/interaction, 30 min active/day → ~330 RPS avg, 2-3K RPS peak
- 100k concurrent active chatters → 5-10K RPS peak

Fast-path-dominant traffic: **1-3 boxes** cover 100k users.
LLM-heavy traffic: **5-15 boxes** cover 100k users.

The Rust service is **stateless**. Add replicas behind a load balancer.

### Bottleneck order (not the app)

1. **LLM provider rate limits.** Hit before the app does. Mitigate with caching, multi-provider routing, tiered models.
2. **Postgres trace writes** at 5K RPS × 5KB = 25MB/s. Single Postgres handles this with batched writes + JSONB + daily partitions; ClickHouse migration is the long-term answer.
3. **Postgres connection pool.** Tune to ~20 conns per app instance. Add `pgbouncer` once we have >5 instances.
4. **The app.** Last to break.

### Threshold-driven changes (not pre-built)

| Threshold | Change | Cost |
|---|---|---|
| 2nd instance | In-process cache → Redis (Cache trait swap) | 1 day |
| 5+ instances | Add pgbouncer | ½ day |
| 1K+ RPS sustained | Verify partitioning automation, tune pool | ½ day |
| Voice agent customer | Add `POST /v1/check/stream` (SSE), same handlers | 1-2 days |
| Trace volume hurts Postgres | Move traces to ClickHouse | 3-5 days |
| Multi-region | Regional deploys, async trace replication | 1-2 weeks |

None of these are rewrites. Each is a contained swap behind a trait.

---

## 9. SDK strategy (locked)

Customers always write *some* code. The SDK turns that code from 30 lines into 1-2 lines.

### Layers

```
Layer 4: Framework adapters     (Vercel AI SDK, LangChain, OpenAI Agents)
Layer 3: Helper patterns        (guard(), wrap(), branch helpers)
Layer 2: Typed client           (generated from OpenAPI)
Layer 1: HTTP/JSON              (the contract)
```

Layer 1 is canonical. Layer 2 is generated by `tl-codegen` (already exists). Layer 3 is hand-written. Layer 4 is per-framework adapters, shipped only when a design partner needs one.

### v0 ships

1. **TypeScript SDK** (`@trustloop/sdk`) — typed client + `guard()` helper
2. **Python SDK** (`trustloop`) — typed client + `guard()` helper

Defer:
- Other languages (Go, Ruby, Java)
- Framework adapters — ship the most-requested one *after* design partners reveal which framework they actually use

### Helper shape

```typescript
const reply = await tl.guard({
  input, draft, context,
  onBlock:    () => cannedReply,
  onRevise:   (revised) => revised,
  onEscalate: () => { humanQueue.push(conversation); return holdMessage; },
  onError:    () => draft,                  // fail-open default
});
```

The SDK **cannot enforce** — it cannot stop the customer's code from sending the message. It makes compliance trivial, not mandatory. That boundary is intentional and matches §1.

### SDK design principles

1. Clients (Layer 2) are generated from `docs/openapi.yaml` via `tl-codegen`.
2. No SDK-only features. Anything the SDK does, the HTTP contract supports.
3. Same wire format across all SDKs.
4. SDK never adds latency (no SDK-side rate limit, no client cache).
5. One config object: API key, base URL, timeout, retry, fail-open/closed default.

---

## 10. Crate alignment

We use the existing `tl-` prefix and existing crates. No new crate is added in v0 unless something concrete ships from it that doesn't ship from any other crate.

| Decision area | Lives in | Notes |
|---|---|---|
| Types and contract | `tl-core` | unchanged |
| Policy YAML parsing | `tl-policy` | unchanged |
| Hot path + tier orchestration | `tl-engine` | extend with parallel-cancel orchestrator |
| Streaming for voice | `tl-stream` | unchanged |
| HTTP server | `tl-server` | unchanged |
| Rust SDK | `tl-sdk-rust` | unchanged |
| Trace store | `tl-storage` | add `PostgresStore` impl |
| Replay tooling | `tl-replay` | unchanged |
| Operator CLI | `tl-cli` | unchanged |
| Codegen for OpenAPI / TS / JSON Schema | `tl-codegen` | drives SDK generation |

TypeScript and Python SDKs continue to live under `sdks/`, generated from `docs/openapi.yaml`.

---

## 11. Build order (v0)

Numbered milestones. Each milestone has a goal and a verification step.

| # | Milestone | Goal | Verify |
|---|---|---|---|
| 0 | Competitor latency baseline | Pin a p95 number we will beat 3-5x | Number recorded in this doc |
| 1 | Lock `tl-core` types | Wire shape final | `cargo check` passes; openapi.yaml regenerated |
| 2 | `tl-server` skeleton | Stub `/v1/check` returns `allow` | curl roundtrip works |
| 3 | Tier 1 (deterministic) | banned phrases, regex, length, PII | unit tests; criterion benchmark <1ms |
| 4 | Tier orchestration | parallel-with-cancel pattern | integration test with mock tiers |
| 5 | Tier 2 (fuzzy) | local embedder + HNSW; Levenshtein | benchmark <20ms |
| 6 | LLM client (`tl-llm` or in-engine) | Anthropic + OpenAI behind a trait | mock tests, real call gated by env |
| 7 | Tier 3 (LLM) | hallucination, tone, promise checks | e2e test with recorded fixtures |
| 8 | `tl-storage::PostgresStore` | batched async writer | trace persists; latency unaffected |
| 9 | Escalation webhook | retry/backoff via channel | integration test |
| 10 | TS SDK (generated + helpers) | `guard()` works end-to-end | example app |
| 11 | Python SDK | parity with TS | example app |
| 12 | Load test | p95 numbers vs targets | results recorded in repo |

Estimate: **8-12 working days** for a single engineer to reach milestone 12.
Milestones 0 and 1 are blockers for everything else.

---

## 12. Open questions (need answers before phase 1)

1. **Competitor latency baseline.** What number do we commit to beating? Cannot start phase 12 without this.
2. **LLM provider pick.** Anthropic, OpenAI, or both behind a trait from day 1? (Recommendation: trait from day 1, ship with Anthropic Haiku as default — cheap and fast.)
3. **Embedding model for Tier 2.** Default to `BGE-small` via `fastembed-rs` (~100MB, runs in-process). Confirm or pick alternative.
4. **Fail-open vs fail-closed default in SDK.** Recommendation: fail-open with a loud metric — most customers will not tolerate sudden outage cascading from us. They can opt-in to fail-closed for high-stakes domains.
5. **Auth for v0.** Static bearer token in `Authorization` header is enough until we have multi-tenancy. Confirm.

---

## 13. Things deliberately not in v0

(Reproduced from [architecture.md §What is explicitly NOT in v1](architecture.md#what-is-explicitly-not-in-v1) and extended.)

- Tool / permission / credential layer (Clawvisor's territory)
- Coding-agent diff review
- Browser-agent action approval
- Workflow / orchestration platform
- Non-engineer policy UI
- Multi-tenancy (single API key)
- Admin dashboard
- Policy hot-reload
- Streaming `check()` endpoint (added when first voice customer arrives)
- Eval harness (raw trace queries cover this until we need more)
- Prometheus metrics (tracing JSON logs cover v0)
- Auth beyond static bearer token
- `enforce()` proxy mode

Each is a deliberate "later." The architecture supports adding them; v0 does not include them.

---

## 14. Confirmation checklist

Before phase 1 starts, the following must be answered:

- [ ] Competitor p95 number committed (§6, §12.1)
- [ ] LLM provider strategy confirmed (§12.2)
- [ ] Embedding model confirmed (§12.3)
- [ ] SDK fail-open default confirmed (§12.4)
- [ ] Auth scheme confirmed (§12.5)
- [ ] Parallel-with-cancel execution pattern accepted as supersession of serial cascade (§4)
- [ ] Three-source rule layering accepted (§5)
- [ ] No-queue-in-hot-path accepted (§7)
- [ ] SDK layer plan accepted (§9)

When all are checked, phase 1 begins.

---

## 15. Event-centered runtime (locked)

A chatbot produces text; an agent takes actions. Once a model can send an
email, call a tool, write memory, or mutate a database, the safety question
stops being "does this text look harmful?" and becomes "should this next step
be allowed, given how we got here?" The contract shift:

```text
OLD (output-centered):   check(input, proposed_output) -> decision
NEW (event-centered):    check(GuardEvent)             -> decision
```

**Thesis: the LLM is not the security boundary; the runtime is.** Output
checking did not disappear — it became one event kind (`output.proposed`)
inside a decision system that also guards tool calls, memory writes, and
external actions. Every entry point (legacy `/v1/check`, gateway, direct
`/v1/events`, SDK adapters) normalizes to the same `GuardEvent`, so checkers
reason over one vocabulary instead of N integration shapes.

Evidence collection shipped before enforcement on purpose: labels, provenance,
and tool resolution ran observe-only on real traffic first, so the label
design was validated by traces — not by enforcement incidents — before any
verdict depended on it. Research grounding for the event model, labels,
checkers, and bench dimensions lives in
`docs/research/trustloopguard-runtime-security-architecture/main.pdf`.

How the engine works today is owned by [event-engine.md](event-engine.md);
this section only records why it has this shape.

## 16. Enforcement is an opt-in rollout (locked)

Every checker ships **OFF by default** and is promoted per workspace and
per environment through the OFF → SHADOW → ENFORCE ladder. Reasons:

- A guardrail vendor must never change customer-visible behavior by deploying
  code. Mode is configuration data, not a code fork — the same checker runs in
  shadow and enforce, so shadow traces show exactly what enforce would decide.
- Only **deterministic** findings decide verdicts. LLM/classifier signals are
  advisory evidence and can never block or unblock an action by themselves —
  probabilistic judgment must not be the boundary it is meant to guard.
- **Missing provenance is never clean.** Unprovable control of a high-impact
  action or an authority-bearing parameter escalates or blocks under enforce
  (both the flow and parameter-auth checkers fire on it); treating absence of
  evidence as safety would invert the threat model.
- A rollout-config read failure fails the request rather than silently
  inheriting weaker modes: an environment may be stricter than its workspace.

Deliberate deferrals inherited from the build-out, each waiting on a real
trigger rather than speculation: retrieval-time/cross-session memory analysis,
sandbox enforcement of `constraints`, ClickHouse/OLAP analytics, an external
durable broker, an edge sidecar runtime, and supply-chain signing of tool
registries.

## 17. Labeling strategy: structure-first, fail-closed for authority (locked)

The two hard problems in the event engine are (a) assigning labels and (b) the
provenance of values the model *synthesized*. Both follow one rule:
**structure-first; detect only as a fallback; invert the burden of proof for
authority.** Perfect taint tracking is a losing game — correctness comes from a
fail-closed gate, and quality (few false alarms) comes from how much structure
we can extract.

By mechanism: origin is *reported* by the producer (a fact), trust is a
*deterministic lookup* over origin + config that fails closed, confidentiality
is *declared* first and *content-detected* only as fallback, integrity is
*derived*. Trust propagation is a lattice (`trusted ⊕ untrusted = untrusted`),
which catches laundering — a model summary of an untrusted email stays
untrusted.

For synthesized values, the layered design (in authority order):

1. **Structural** — values carry lineage via labeled handles/capabilities
   (CaMeL-style); exact, free, and grows with SDK-adapter adoption.
2. **Containment** — value appears inside a known untrusted source → tainted
   (signal).
3. **Fail-closed authority gating** — the guarantee (AuthGraph-style): an
   authority-bearing parameter must *prove* it came from an `allowed_source`;
   a synthesized string has no proof, so **the model cannot launder authority
   through synthesis**. We require values be provably clean, not provably
   tainted.
4. **Model attribution** — advisory corroboration; never the boundary.

Why this is the differentiator: we stay correct with an imperfect model
(layer 3), and get *better* as customers climb the gateway → SDK → capability
ladder (layer 1). Measured by TrustLoopGuardBench: parameter-source catch rate
against false-block rate.

Implementation status: layers 1 and 3 are live (declared labels, provenance
maps, the parameter-auth checker's missing-proof escalation). Confidence
scoring, pattern/classifier label detection (layer 2), and model attribution
(layer 4) are design intent, not current behavior — today's resolver is the
three-level cascade in [event-engine.md](event-engine.md).

## 18. Temporal reach: T1/T2/T3 (locked)

Provenance reach decides how much state the engine holds:

| Class | Scope | State needed | Posture |
|---|---|---|---|
| **T1** | unsafe instruction and harmful action in the same turn | none — the `GuardEvent` is self-contained | full |
| **T2** | payload persists across turns in one session | session-scoped, anchored on runs/run-events | full |
| **T3** | payload crosses sessions (plant in session 1, execute in session 7) | durable cross-session provenance store | **write-time block only** |

The T3 decision: stop untrusted content from *becoming* authority-bearing
memory at write time — nearly stateless, shipped with the memory checker — and
defer the retrieval-time cross-session lineage graph. The session-1→session-7
attack is therefore *partially* defanged now (the poison doesn't get stored)
and will be *fully* caught later (at retrieval even if stored). This is the
"retrieval-time/cross-session memory analysis" deferral in §16.
