# Event-Centered Engine — High-Level Design

This is a **high-level design** for revamping the TrustLoopGuard runtime from an
output-centered check (`check(input, proposed_output)`) into an event-centered
guard (`check(GuardEvent)`), as proposed in
[the runtime security architecture whitepaper](../research/trustloopguard-runtime-security-architecture/main.pdf).

It is a design spec, **not** the canonical architecture source. `docs/concept/`
remains the source of truth for shipped product behavior; this doc describes the
target shape and the vocabulary we are committing to define. Detailed phase
breakdown and field-level contracts are **deferred on purpose** — the goal here
is to get the system shape right and agree on *what we want to define* before we
cut any types.

Status: **Draft / for review.** Nothing here is built yet.

**Related specs:**
- [`event-engine-roadmap.md`](./event-engine-roadmap.md) — phases, per-phase research grounding, infra decisions, end goal.
- [`event-engine-phased-implementation.md`](./event-engine-phased-implementation.md) — independently executable phase documentation with inputs, outputs, testing, and done gates.
- [`event-engine-class-and-db-design.md`](./event-engine-class-and-db-design.md) — concrete `tl-core` types, `tl-engine` traits, and DB tables.
- [`integration-interception.md`](./integration-interception.md) — how an agent is hooked (tool-calling mechanics, proxy vs adapter).

---

## 1. Why event-centered

A chatbot produces text; an agent takes actions. Once a model can send an email,
call a tool, write memory, or mutate a database, the safety question stops being
"does this text look harmful?" and becomes "should this next step be allowed,
given how we got here?"

The contract shift:

```text
OLD (output-centered):   check(input, proposed_output) -> decision
NEW (event-centered):    check(GuardEvent)             -> decision
```

Output checking does not disappear. It becomes **one event kind**
(`output.proposed`) inside a broader runtime decision system that also guards
tool calls, memory writes/retrievals, file/shell/network/browser actions,
database/API mutations, and external messages.

**Thesis we are designing toward:** the LLM is not the security boundary; the
runtime is. TrustLoopGuard makes agent actions *inspectable, enforceable, and
auditable before they create harm.*

---

## 2. Design goals and non-goals

**Goals**

- One stable event contract that all integrations (SDK, gateway, embedded) speak.
- A pipeline whose *shape* is fixed early, so capability lands by filling slots,
  not by re-cutting the contract.
- Decisions backed by **evidence** (who acted, what they tried, where each value
  came from, which policy fired), not just a verdict.
- Zero forced behavior change for existing `/v1/check` users until a workspace
  deliberately opts in.

**Non-goals (this design does not try to be)**

- A replacement for model alignment, OS/container/browser sandboxing, least
  privilege, identity management, or secure tool registries. TrustLoopGuard is
  the *policy-and-evidence layer before execution*; enforcement of physical
  constraints belongs to adapters it instructs.
- A bigger LLM judge. The security core is deterministic; model/classifier
  checks are optional *signals*, never the boundary.

---

## 3. The core model

Two things anchor the whole system.

### 3.1 GuardEvent (the input)

A normalized envelope describing one proposed agent step. Conceptually:

- **kind** — what the agent wants to do (the event taxonomy in §6).
- **principal** — on whose behalf (workspace, environment, agent, user, session,
  task, run linkage).
- **action** — the concrete operation: tool/operation name, parameters, and its
  side-effect class.
- **sources** — the pieces of data/context in play, each carrying labels.
- **provenance** — which parameter/value came from which source.
- **context** — free-form additional context (carried, not trusted).

Output checking maps in as `kind = output.proposed` with the proposed text as the
content-bearing value.

### 3.2 Decision (the output)

The four verdicts we already have are sufficient and stay as-is:

| Verdict | Meaning |
|---------|---------|
| `allow` | proceed as proposed |
| `block` | must not execute |
| `rewrite` | proceed only after safe transformation |
| `escalate` | runtime can't decide; human/extra verification required |

What grows is the **evidence attached to the verdict**: reason, violated
policy/rule, remediation, source chain, labels, and diagnostic annotations (risk
source, failure mode, harm class). A block without diagnosis is not
production-grade.

---

## 4. Pipeline architecture (the skeleton)

The runtime is a fixed sequence of **stages**, each a trait with a no-op default.
We build the whole skeleton first; then we implement and wire stages in one at a
time. A stage that isn't implemented yet passes through (`allow`, attach
nothing).

```text
GuardEvent
  → Normalize          (build/validate the GuardEvent; map legacy CheckRequest)
  → Resolve principal + action   (identity; resolve action against tool metadata)
  → Enrich             (attach/resolve labels; propagate provenance)
  → Check []           (composable checkers, each independently gated):
       • Content       (wraps today's deterministic/fuzzy/LLM tiers)
       • Information-flow
       • Parameter-source authorization
       • Memory
  → Compose decision   (merge checker verdicts; attach evidence)
  → Persist trace      (decision + evidence as an audit artifact)
```

### 4.1 Enforcement lifecycle (a design property, not just rollout)

Every checker must support three states, per workspace:

- **OFF** — not evaluated.
- **SHADOW** — evaluated; verdict + evidence recorded to the trace; **not acted
  on.** Lets us measure false-block rate against real traffic before enforcing.
- **ENFORCE** — verdict acts on the decision.

This is structural: shadow-ability is a requirement of the checker interface, so
every capability can be observed safely before it ever changes a customer's
result.

### 4.2 Hot path stays in-process

The decision path (normalize → … → compose) stays Rust, in-process, deterministic
where it matters. We do not split flow/authorization/metadata lookups into
network services on the hot path. Model/classifier calls remain optional,
parallel signals with deadlines, exactly as the current LLM tier behaves.

### 4.3 LLM / classifier is a signal — and that's a behavior change

**Decision (made):** the LLM/classifier is an **optional signal, never the
boundary** (whitepaper §III, §V.2). This is a **change from today's engine**, where
the LLM tier can directly `Block` (hallucination-not-grounded → block,
authority-out-of-scope → block). The precise new rule:

- **Content safety** (`output.proposed`) — the classifier may still **gate
  content** (the existing Llama-Guard-style value). Keep it.
- **Actions** (tool call, memory, file, …) — the LLM is **signal-only**; the
  deterministic checkers + escalation decide. Uncertain/conflicting evidence →
  **escalate**; an authority-creating action requires **corroboration**, never a
  single LLM judgment (§V.2).

Net: the LLM goes from "can block anything" → "can gate content, but for actions it
only advises." Treat this as a deliberate step in the engine refactor, not a silent
relabel.

---

## 5. Layer ownership (maps onto existing crates)

| Concern | Owner | Today |
|---------|-------|-------|
| Event/decision wire vocabulary | `tl-core` | has `CheckRequest`/`Decision`/`Verdict` |
| Policy DSL parse/validate/compile | `tl-policy` | has matchers; extend for new families |
| Runtime pipeline + checkers | `tl-engine` | has 3-tier content pipeline |
| HTTP, auth, resolution, orchestration | `tl-server` | has `/v1/check`, workspace/env/key resolution |
| Durable persistence + registries | `tl-storage` | has traces, policies, agents, runs/run_events |
| Evidence producers (sources/provenance) | SDKs / gateway | thin HTTP wrappers today |

The **runs / run_events** tables already model a session and its ordered turns —
this is the anchor we extend for session-scoped state, not a new build.

---

## 6. What we are committing to define (the vocabulary)

This is the heart of "decide what we want to define." Each item is a concept with
a clear responsibility; exact fields are deferred.

| Concept | Responsibility | New? |
|---------|----------------|------|
| **GuardEvent** | the normalized proposed-step envelope | new |
| **EventKind** | the taxonomy: `output.proposed`, `tool.call.proposed`, `memory.write.proposed`, `memory.retrieval.used_for_action`, `file.action.proposed`, `shell.action.proposed`, `network.request.proposed`, `browser.action.proposed`, `database.mutation.proposed`, `api.mutation.proposed`, `external_message.proposed` | new |
| **Principal** | who is acting: workspace, environment, agent, + user/subject, session, task, run linkage | extend existing |
| **Action** | operation name, parameters, side-effect class | new |
| **Source** | a unit of data/context in the event, with labels | new |
| **Label** | trust (trusted/untrusted/unknown), confidentiality (public/private/secret/identity), integrity (low/med/high), origin (user/system/tool/memory/file/web/email/api) | new |
| **ProvenanceMap** | `parameter/value → [source ids]` | new |
| **ToolMetadata** | per-operation: side-effect, reversibility, authority-bearing vs content-bearing params, allowed sources, approval rule, sandbox hint | new (durable registry) |
| **Decision evidence** | violated rule, remediation, source chain, risk source, failure mode, harm class | extend existing |

We are **not** committing yet to: trace-graph node/edge schema, cross-session
memory store schema, or the eval-harness format. Those are named but deferred.

---

## 7. Labeling subsystem (conceptual)

Labeling is mostly **not** an inference engine. It splits by mechanism:

- **Origin** — *reported* by the producer (it knows it called `read_email`). Fact.
- **Trust** — *deterministic lookup* from origin + config; fail closed (unknown
  external tool output → untrusted).
- **Confidentiality** — *declared* (a source/path/tool marked private/secret) **+**
  *content-detected* (does this blob contain PII/secrets?). The content part is
  the only place a classifier is needed — and we already have one in the
  redaction/PII detector and the LLM tier, used here as a **signal**.
- **Integrity** — *derived* from trust + verification.

Two deterministic components do the real work:

- **LabelResolver** — origin/config → default labels (cheap, in-process).
- **Label propagation** — labels travel with derived values over the provenance
  map. (e.g. `body ⟵ read_file(private_file)` ⇒ `body` is private.) This is pure
  logic over the provenance graph, no model.

### 7.1 Resolution strategy — the two hardest problems (the differentiator)

The two hard parts are (a) assigning labels and (b) the provenance of values the
model **synthesized**. Both follow one rule: **structure-first; detect only as a
fallback; invert the burden of proof for authority.** Perfect tracking is a losing
game — correctness comes from the fail-closed gate, *quality* (few false alarms)
comes from how much structure we can extract.

**Labels — structure-first.** Trust is a deterministic lattice over `origin` plus
propagation (`trusted ⊕ untrusted = untrusted`, which catches laundering — a model
summary of an untrusted email stays untrusted). Confidentiality is a 3-layer
resolver:
1. **declared-source inheritance** (exact — most private data is private *by origin*),
2. **pattern detectors** (cards / keys / SSN / IBAN — reuse the redaction/PII pipeline),
3. **classifier / NER** (signal only — the fallback for unlabeled free text).
Each label carries a **confidence** (declared 1.0 · pattern 0.95 · classifier ~0.6);
policy sets the **threshold by sink impact** (a money-movement sink demands
high-confidence-trusted; a log sink doesn't). The fragile classifier is the
backstop, never the primary mechanism.

**Synthesized-value provenance — 4 layers, fail-closed backbone.**
1. **Structural** (gold) — values carry lineage via labeled handles/capabilities
   (CaMeL). Exact and free; we ship tool wrappers so adopters get it.
2. **Containment** (signal) — the value appears in a known untrusted source → tainted (~0.7).
3. **Fail-closed authority gating** (the guarantee — AuthGraph) — an
   authority-bearing param must **prove** it came from an `allowed_source`; no proof
   → escalate/block. A synthesized string has no proof, so **the model cannot
   launder authority through synthesis.** We require the value be provably *clean*,
   not prove it is *tainted*.
4. **Model attribution** (advisory) — a judge corroborates; never the boundary.

**Why this is the differentiator:** correctness comes from layer 3 (fail-closed),
*not* from perfect tracking; quality (few false escalations) comes from maximizing
structural coverage (layer 1 + declared labels). So we stay correct even with an
imperfect model, and we get *better* as customers adopt structural provenance — the
gateway → SDK → capability ladder. Measured by `TrustLoopGuardBench`:
parameter-source violation catch rate × false-escalation rate.

**Limits** (whitepaper §XXV): same-source pollution, paraphrase evasion, classifier
false positives — contained by confidence scoring + fail-closed + human escalation,
not eliminated.

---

## 8. State and temporal model

Provenance reach is the variable that decides how much state we hold.

| Class | Scope | State needed | v1 posture |
|-------|-------|--------------|------------|
| **T1** | unsafe instruction and harmful action in the same turn | none — the GuardEvent is self-contained | **full** |
| **T2** | payload persists across turns in one session | session-scoped, anchored on `runs`/`run_events` | **full (extend runs)** |
| **T3** | payload crosses session boundaries (plant in session 1, execute in session 7) | durable cross-session provenance/memory store | **write-time block only** |

**v1 T3 decision (agreed):** stop untrusted content from *becoming*
authority-bearing memory at **write time** — nearly stateless, lands with the
flow checker, and removes most of the cross-session attack surface. The full
*retrieval-time* cross-session lineage graph (catch it even if it was stored) is
deferred to a later release. So the session-1→session-7 attack is *partially*
defanged early (don't let the poison get stored) and *fully* caught later (catch
it at retrieval even if stored).

---

## 9. Two planes

- **Runtime / data plane** — low-latency decision path: `/v1/check`, event checks,
  policy/flow/authorization evaluation, provenance checks, trace enqueue. Stays
  in-process.
- **Control plane** — slower management: policy CRUD, tool-metadata registry CRUD,
  API keys, environment/workspace settings, dashboard analytics, (later) eval
  jobs. Registry populated here, consumed by the data plane.

---

## 10. Compatibility principle

- Keep `/v1/check`. Legacy `input + proposed_output` normalizes to
  `output.proposed`; the existing tiers **are** the content checker, so behavior
  is byte-identical until a flag flips.
- The contract is additive — new fields optional/defaulted; old SDKs unaffected.
- New event kinds and checkers land dark (OFF), graduate through SHADOW, then
  ENFORCE per workspace.
- A later `/v2/check` (or `/v1/events/check`) is introduced only once the event
  contract is stable.

The build philosophy in one line: **skeleton of every stage first → each stage a
`no-op` → wire real logic in one at a time, gated and shadow-tested.**

---

## 11. Coverage strategy & adapter model

Goal: support as many agents as possible — **voice or chat, any framework**
(LiveKit, Mastra, OpenAI Agents SDK, LangChain, custom) — **without the core ever
knowing they exist.**

**The principle: one abstract contract, framework knowledge only in thin adapters.**
- `GuardEvent` stays deliberately **abstract/universal** — it describes *actions*
  (tool call, message, memory write…), never a framework or a modality.
- All framework-specific code lives in **thin adapters** that translate
  *framework X → `GuardEvent`*. New framework = new adapter; the core never changes.
- **Enforcement discipline:** `tl-core` / `tl-engine` must **never import or
  reference any framework**. Framework code ships as separate adapter packages.
  (Building the core against synthetic events keeps this honest.)

**Modality-agnostic by hooking the *action* layer.** Voice vs chat differ only in
the I/O channel (STT/TTS); at the action layer the event is identical, so we are
modality-agnostic by construction. Voice adds only a latency constraint — already
met by the sub-ms deterministic core + sheddable LLM signal.

**Coverage ladder** — every agent lands on at least one rung; fidelity scales with
how deeply they integrate:

| Rung | Mechanism | Covers | Fidelity |
|------|-----------|--------|----------|
| 0 — Provider-API gateway | one integration to the OpenAI/Anthropic format | any framework calling those providers (broad, one shot) | low |
| 1 — MCP proxy | one adapter at the MCP protocol boundary | any agent using MCP tools | medium |
| 2 — Framework adapters | LiveKit · Mastra · LangChain · OpenAI Agents SDK | the popular frameworks | high |
| 3 — Generic SDK primitive | raw `check(event)` / `@guarded` | custom / niche / home-grown agents | scales with wrapping |

Keep adapters cheap (so coverage stays wide):
- Build the SDK around a few **universal hook points** — `before_tool_call`,
  `after_tool_result`, `before_llm_call`, `on_memory_op` — that map onto each
  framework's existing callback/middleware system.
- **Never require restructuring** — wrap / callback / config, never "rewrite your agent."
- Keep the **adapter interface open and documented** so the ecosystem extends coverage.
- Ship **language SDKs** (TS / Python / Rust) — tiny universal core + idiomatic sugar.

One-line rule: **abstract contract; framework knowledge only in thin adapters;
hook the action layer (modality-irrelevant); standards (provider API, MCP) for
breadth, adapters for depth, generic primitive as the escape hatch.**

---

## 12. Open design decisions (decide before cutting types)

These shape the contract and must be resolved at design time, not discovered
during build:

1. **Evidence authority** — are `Source`/`Label`/provenance *supplied by the SDK*,
   *re-derived by the runtime*, or *split* (origin/trust reported, confidentiality
   re-derived when data-handling mode permits)? Determines which fields are
   required-on-input vs computed, and interacts with existing redaction modes
   (`SdkLocal`/`Server`).
2. **Trace evidence persistence** — promote which evidence fields to indexed
   columns vs keep in the JSONB payload? (driven by what the dashboard filters on.)
3. **SDK responsibility line** — how much session-state tracking lives in the SDK
   (T2 client map) vs the runtime?
4. **ToolMetadata ownership/onboarding** — who authors it, how is it validated,
   and what is the default when a tool is unregistered (fail-closed?).
5. **Policy family modeling** — how content/flow/parameter/approval/memory policies
   coexist in `tl-policy` and how violated rules + remediation are represented.

---

## 13. Roadmap

Phasing, the build timeline, the extended (optional) track, `TrustLoopGuardBench`,
and the end-state goal live in [`event-engine-roadmap.md`](./event-engine-roadmap.md).
The gap items from the paper (sandbox/adapter constraints, multi-agent delegation,
ecosystem/supply-chain, hallucination corroboration) are tracked there as the
**extended track — possible but not required** for the first product.

Still deferred at the schema level until needed: trace-graph schema, cross-session
memory store schema, and the `TrustLoopGuardBench` data format.

---

## Appendix A — Conceptual Phase 0 contract (illustrative, not final)

Sketch of the `tl-core` shape so reviewers can react to the *structure*. Field
sets are intentionally minimal and **subject to the §12 decisions.**

```text
GuardEvent {
  kind: EventKind
  principal: Principal
  action: Action
  sources: [Source]
  provenance: ProvenanceMap            // param -> [source_id]
  context: Json
}

Principal {
  workspace_id, environment_id, agent_id
  user_id?, session_id?, task_id?
  run_id?, run_event_id?
}

Action {
  operation: String                    // e.g. "send_email", "output"
  parameters: Json
  side_effect: SideEffectClass         // resolved from ToolMetadata
}

Source {
  id: String
  origin: Origin                       // reported by producer
  labels: Labels                       // resolved + propagated
}

Labels { trust, confidentiality, integrity }

Decision {                             // extends today's Decision
  trace_id, verdict, reason
  triggered_policies: [...]            // existing
  violated_rule?, remediation?         // new
  source_chain?, risk_source?, failure_mode?, harm_class?   // new evidence
  latency_ms, tier_results, ...        // existing
}
```

Legacy mapping: `CheckRequest{input, proposed_output}` →
`GuardEvent{ kind: output.proposed, action.operation: "output",
action.parameters.text: proposed_output, sources: [input...] }`.
