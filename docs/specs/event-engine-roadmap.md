# Event-Centered Engine — Roadmap & Timeline

Companion to [`event-engine-design.md`](./event-engine-design.md). The design doc
fixes the system *shape* and vocabulary; this doc sequences the build.

It separates a **required critical path** (the core product) from an **extended
track** (capabilities that are possible but not required for the first product),
and calls out **TrustLoopGuardBench** as its own deliverable. Exact ordering of
later items depends on the open design decisions in design-doc §11.

Status: **Draft / for review.** Nothing built yet.

---

## Timeline at a glance

```text
                         ════════════  REQUIRED CRITICAL PATH  ════════════

  Phase 0 ── Phase 1 ── Phase 2 ── Phase 3 ── Phase 4 ── Phase 5 ── Phase 6 ──►  ★ CORE PRODUCT
  Skeleton   Capture    Tool       Labels     Info-flow  Param-     Policy +        DONE
  + Contract + Observe  Metadata   (shadow)   + Mem      source     Approvals
                                              write-     auth
                                              block
  └─ ships dark (no customer impact) ─┘   └── first enforcement (opt-in, shadow→enforce) ──┘

                                              traces become rich enough here
                                                          │
                                                          ▼
                         ════════  REQUIRED, PARALLEL  ════════
                                 TrustLoopGuardBench
                  (3 tracks · metric suite · evolving eval loop)
                  starts in shadow after P4, matures after P6

                         ════════  EXTENDED TRACK (possible, not required)  ════════
   E1  Sandbox / adapter enforcement   (file · shell · network · browser events)
   E2  Cross-session memory graph      (full T3 retrieval-time, beyond write-time block)
   E3  Trace graphs                    (program-analysis view over traces)
   E4  Multi-agent / delegation        (inter-agent message provenance, delegation graph)
   E5  Ecosystem / supply-chain        (MCP / tool-registry signing, manifests)
   E6  Hallucination corroboration     (conflicting evidence → escalate)

                         ════════  OUT OF SCOPE (not TrustLoopGuard's job, paper §XXV)  ════════
   model alignment · OS/container sandbox *enforcement* kernel · code signing ·
   package security · identity management
```

**Posture rule (from the design):** every checker ships `OFF → SHADOW → ENFORCE`.
Phases 0–3 ship completely dark. The first behavior change any customer can see is
Phase 4, and it arrives in shadow, per-workspace, measured before enforcement.

---

## Required critical path

| Phase | Goal | What must be fulfilled | Exit gate | Customer impact |
|-------|------|------------------------|-----------|-----------------|
| **0 — Skeleton & Contract** | Freeze the shape, change nothing | `GuardEvent` vocabulary in `tl-core` (incl. a constraint-output field on `Decision`, see E1); pipeline stages stubbed as no-ops; legacy `CheckRequest → output.proposed` adapter | **Golden replay**: old vs new path produce byte-identical `Decision`s | None |
| **1 — Capture & Observe** | See the new events | SDK provenance surface + new event kinds (accepted, recorded, not enforced); trace enrichment (event_kind, principal, sources, provenance) | New events visible in traces/dashboard | None |
| **2 — Tool Metadata Registry** | Tools get structured semantics | Registry table + repo (`tl-storage`); control-plane CRUD (`tl-server`); `ToolMetadata` type; action resolution attaches metadata | A workspace can register tools; events carry side-effect class + param roles | None |
| **3 — Labels (shadow)** | Evidence becomes legible | `LabelResolver` (origin/config → labels, fail-closed) + propagation over provenance; reuse redaction/LLM as confidentiality *signal* | Labels appear on traces; nothing blocks | None |
| **4 — Information-flow + memory write-time block** | First real teeth | FlowChecker (action-integrity + destination-permission); block untrusted content from *becoming* authority-bearing memory | Private→external blocked in enforce mode with measured low false-block rate | First blocks (opt-in) |
| **5 — Parameter-source authorization** | Catch "right tool, wrong source" | ParamAuthChecker vs `ToolMetadata` allowed-sources | Cross-tool parameter pollution blocked (shadow→enforce) | opt-in |
| **6 — Explainable policy + approvals** | Production-grade verdicts | Policy families in `tl-policy`; `violated_rule` + `remediation` in `Decision`; approvals wired into existing escalation infra | Every block explains itself; `escalate` routes to human review | richer reasons |

★ **After Phase 6 the core product is done** (see end goal below).

---

## TrustLoopGuardBench (required, parallel deliverable)

Not optional — but it depends on real traces, so it starts once Phase 4 produces
structured evidence and matures alongside Phase 6.

**v1 — three tracks** (the paper's strongest, buildable set):
1. **Indirect prompt injection** — benign task, external content tries to become instruction.
2. **Private-data flow** — agent tries to move private/identity data to an unauthorized sink.
3. **Delayed memory risk** — untrusted content written in one session, used later.

**Metric suite** (what keeps the guard honest — a guard that blocks everything,
or can't explain itself, fails the bench):
- Security: attack success rate, unsafe source-to-sink rate, parameter-source violation catch rate, unsafe-memory catch rate.
- Utility: benign task completion, utility under attack, **false-block rate**, false-escalation rate, approval burden.
- Runtime: latency overhead, LLM calls/decision, cost/request, cache hit rate.
- Trace/audit: source-chain accuracy, risk-source/failure-mode/harm-class attribution accuracy, explanation quality.
- Coverage: % tools with side-effect metadata, % sources with trust labels, % policies with tests.

**Evolving loop:** import customer policy/regulation → atomic rules → generate
tests → run vs guard + agent → analyze misses → promote failures to regression
suite. The guard improves as the agent system grows.

---

## Extended track (possible, not required for first product)

These are real and on the roadmap, but the first product does not depend on them.
Pull any forward when a customer need justifies it.

| Item | What it adds | Why deferred | Note |
|------|--------------|--------------|------|
| **E1 — Sandbox / adapter enforcement** | "allow *with constraints*" for file/shell/network/browser events; adapters enforce allowed paths, network mode, timeouts | TLG decides; an external adapter enforces. The decision-output *contract* should be defined in Phase 0; the adapters + environment-event checkers are the deferred work | Recommend defining the constraint field early even if enforcement lands here |
| **E2 — Cross-session memory graph** | Full T3: catch poisoned memory at *retrieval* time, even if it was stored | Needs a durable cross-session provenance store; v1 already takes the cheap win (write-time block in P4) | The riskiest/most stateful piece — last on purpose |
| **E3 — Trace graphs** | Program-analysis view: did tainted input reach a dangerous sink across the trace? | Needs reliable structured traces first (P1–P6) | Mechanism 5 in the paper |
| **E4 — Multi-agent / delegation** | Inter-agent message provenance + delegation graph (L5) | `Principal` already carries agent identity; the graph is additive | |
| **E5 — Ecosystem / supply-chain** | MCP / tool-registry signing, manifest verification (L6) | Overlaps with code-signing/supply-chain tooling outside TLG | Partly an integration, not pure runtime |
| **E6 — Hallucination corroboration** | "conflicting evidence → escalate"; require corroboration for high-impact actions | Partly folds into P5 authorization + P6 escalate already | Smallest item |

---

## Research grounding per phase

Each phase maps to a section of the whitepaper, which in turn synthesizes specific
prior work. The paper's per-mechanism sections read as design specs — use them as
the primary reference when building each phase. (`§` = whitepaper section;
bracketed numbers are its references.)

### Phase 0 — Skeleton & Contract
- **Paper:** §I (thesis: *the LLM is not the boundary, the runtime is*), §VIII (Output
  Checks → Guarded Events; the `check(event)` reframe + event taxonomy table),
  §IX (runtime architecture), §XXIII.2 (contract migration steps 1–5).
- **Builds on:** Llama Guard / ShieldGemma [24, 25] and NeMo Guardrails [17] —
  content moderation becomes the *compatibility adapter* (`output.proposed`), one
  event kind and a signal, not the boundary.
- **Build takeaway:** define `GuardEvent` + `EventKind` (the §VIII.1 table), keep
  `/v1/check`, stages no-op. The migration is literally §XXIII.2.

### Phase 1 — Capture & Observe
- **Paper:** §IX (components: action interceptor, principal resolver, provenance
  tracker, trace store), §XI.2 (provenance), §XIX (monitoring; *traces are security
  artifacts*; trace-detail list).
- **Builds on:** AgentArmor [7] (capture a structured, analyzable trace first),
  ATBench [10] (full action history matters), R-Judge [16] (risk is behavioral,
  not textual — you need the steps recorded).
- **Build takeaway:** the producer (SDK/gateway) is the interceptor; persist
  event_kind/principal/sources/provenance as evidence. Observe-only is justified by
  §XIX — *a block without diagnosis is not enough.*

### Phase 2 — Tool Metadata Registry
- **Paper:** §X (Mechanism 1: Tool and Action Metadata; the field table).
- **Builds on:** ToolSword [23] (tool safety across input/execution/output stages),
  ToolSafe [22] (proactive, *check before invocation*), ShieldAgent [9] (group
  policy by action type), AGrail [27] (store reusable checks by action type;
  *what concrete check would prove this action safe?*).
- **Build takeaway:** implement the §X metadata fields — side effect, reversibility,
  authority-bearing vs content-bearing params, allowed source, flow constraint,
  approval rule, sandbox hint. This gates Phases 4/5/E1.

### Phase 3 — Labels (shadow)
- **Paper:** §XI (Mechanism 2: Source Labels and Provenance), §XI.1 (label families),
  §XI.2 (provenance map).
- **Builds on:** FIDES [5] and CaMeL [4] — *keep security labels attached to data
  as it moves and enforce them in the runtime before sensitive tools execute*; move
  the boundary outside the model.
- **Build takeaway:** label families (trust/confidentiality/integrity/origin) from
  §XI.1; fail-closed defaults (unknown external → untrusted; sensitive → private/
  secret). Propagation is the FIDES/CaMeL "labels travel with data" idea —
  deterministic over the provenance map. Content classifier = signal only.

### Phase 4 — Information-flow + memory write-time block
- **Paper:** §XII (Mechanism 3: Information-Flow Control), §XVII (Memory Security),
  §VII.2 (temporal model T1/T2/T3).
- **Builds on:** CaMeL [4], FIDES [5] (information-flow control as the deterministic
  core); LASM [14] / ATBench [10] (delayed, cross-session risk).
- **Build takeaway:** the two rules verbatim from §XII — *action-integrity*
  (high-impact ops authorized by trusted context, not attacker-writable
  observations) and *destination-permission* (sensitive data only to allowed sinks).
  Memory write-time block = §XVII MVP control #2 (*block/escalate memory writes from
  untrusted content unless allowed*) — the cheap T3 win.

### Phase 5 — Parameter-source authorization
- **Paper:** §XIII (Mechanism 4: Parameter-Level Authorization; the per-parameter
  table + verdict table).
- **Builds on:** AuthGraph [6] — *Aligning Provenance with Authorization*; catches
  "correct tool, wrong parameter source" (the `book_flight(flight_id="EVIL-123")`
  case).
- **Build takeaway:** per-parameter allowed-source policy (§XIII: `flight_id` ←
  search result, `send_email.to` ← user prompt / trusted contact, `file.write.path`
  ← user / workspace policy). Verdict reports expected vs actual source.

### Phase 6 — Explainable policy + approvals
- **Paper:** §XV (Mechanism 6: Policy as a Reasoning Layer; the 6-step flow + example
  verdict JSON), §IX.1 (decision semantics), §V.2 (conflicting evidence → escalate),
  §XIX (diagnosis).
- **Builds on:** GuardAgent [8], ShieldAgent [9] (policy reasoning / verifiable safety
  policy over actions), AgentDoG [21] + ATBench [10] (diagnostic labels: risk source,
  failure mode, harm class), Poly-Guard [20] (policy-grounded, multi-domain).
- **Build takeaway:** the §XV reasoning steps; verdict carries `violated_rule` +
  `remediation`; approval engine for high-impact/ambiguous actions. *A block without
  diagnosis is not production-ready* (§XIX).

### TrustLoopGuardBench
- **Paper:** §IV (benchmark landscape + 3 baseline threat models), §XX (the three
  tracks + dimensions), §XXI (metric suite), §XXII (evolving evaluation loop, Fig 6).
- **Builds on:** AgentDojo [1] + InjecAgent [18] (track 1, indirect injection),
  FIDES [5] / CaMeL [4] (track 2, private-data flow), ATBench [10] / LASM [14]
  (track 3, delayed memory), AgentHarm [2] (malicious-user workflows), ToolEmu [3]
  (high-stakes emulated env), GuardAgent [8] / ShieldAgent [9] (policy-explanation
  labels), AgenticEval [19] + Poly-Guard [20] (evolving, policy-grounded).
- **Build takeaway:** 3 tracks (§XX), full metric suite (§XXI), evolving loop (§XXII
  Fig 6). Caveat from §XXII: *generated evals cannot be blindly trusted* — high-risk
  policy tests need inspection.

### Extended track grounding
- **E1 Sandbox/adapter** → §XVIII (Sandbox Hooks; verdict + constraints contract,
  example decision payload), ToolEmu [3]. TLG decides constraints; adapter enforces.
- **E2 Cross-session memory graph** → §XVII (full memory security), §VII.2 (T3),
  ATBench [10] / LASM [14].
- **E3 Trace graphs** → §XIV (Mechanism 5; node/edge taxonomy), AgentArmor [7]
  (*treat the trace like a program; did tainted input reach a dangerous sink?*).
- **E4 Multi-agent / delegation** → §VI, §VII.1 layer L5; surveys [12, 13, 14].
- **E5 Ecosystem / supply-chain** → §VI, §VII.1 layer L6; tool-use survey [15].
- **E6 Hallucination corroboration** → §V.2 (Reliability Scope), R-Judge [16],
  NeMo Guardrails fact-checking rails [17].

---

## Infrastructure & scaling decisions

Recorded so the roadmap reflects the agreed "now vs later" split for supporting
infrastructure.

**Beava — evaluated, deferred.** Beava (a live windowed risk-feature engine) is
on-topic for future T2 risk-accumulation / behavioral signals, but is not adopted
now: it does not replace cache / queue / OLAP (different category), and at v0.0.x
it is too young to be load-bearing in a security product. If revisited, it goes
behind a `RiskFeatureProvider` interface, advisory-only, off the deterministic hot
path — exit cost is then low (ephemeral window state, no data migration).

**OLAP (ClickHouse) — deferred to scale.** Trace analytics stays on Postgres
(partitioned, recent) for now. ClickHouse enters the roadmap when trace volume or
analytical query load outgrows Postgres (the whitepaper §XXIII names a trace-ingest
service as a future candidate). This is a **trigger, not a date.**

**Now: queue + worker → Postgres, plus Redis cache.**
- *Trace path:* a queue + worker writing to Postgres. Note the current in-process
  mpsc batched writer already *is* a single-instance queue+worker→Postgres; an
  external broker (RabbitMQ / NATS / SQS) becomes necessary only at multi-replica
  scale-out, where the queue must survive a crash and decouple across processes.
- *Cache:* Redis as the shared read cache for policies / tool-metadata / profiles.
  Likewise, the current in-process moka cache is enough for a single instance;
  Redis becomes necessary when N replicas need shared hits + coordinated
  invalidation.

**Sequencing rule:** start in-process (moka + mpsc + partitioned Postgres) →
graduate to external Redis + broker when you add horizontal replicas → add
ClickHouse when analytics outgrows Postgres. Each step is gated by a real trigger
(replica count, volume), not the calendar. The "PREP NOW" seams (cache abstraction,
trace-write-as-event-emission) are exactly what make each graduation a config swap
rather than a rewrite.

---

## The goal after everything is done

When the critical path + bench are complete (and the extended track filled as
needed), TrustLoopGuard is:

> A **provenance-aware runtime enforcement layer** sitting between agent intent
> and real-world action. Every meaningful agent step — output, tool call, memory
> write/retrieval, file/shell/network/browser action, database/API mutation,
> external message — is checked against principal, action, source labels,
> provenance, information-flow rules, parameter authority, and explicit policy,
> then allowed / blocked / rewritten / escalated **with evidence** before any side
> effect occurs. Every decision is an auditable trace, and TrustLoopGuardBench
> continuously regression-tests the guard as the customer's agent gains tools,
> memory, and permissions.

The end is not a perfectly trustworthy LLM. The end is **agent systems that stay
safe and auditable even when the LLM is imperfect** — the runtime, not the model,
is the security boundary.
