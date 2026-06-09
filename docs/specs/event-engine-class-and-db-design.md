# Event-Centered Engine — Class & DB Design

Turns the high-level design ([`event-engine-design.md`](./event-engine-design.md)),
the roadmap, and the whitepaper into a concrete **type/class design** (abstract,
trait-based, scalable) and a **database design**, expressed in this repo's existing
idioms so it *extends* rather than reinvents.

Idioms it follows (already in the codebase):
- `tl-core`: one `pub mod` per concept + `serde` wire types (snake_case, `skip_serializing_if`).
- `tl-engine`: behavior behind `Send + Sync` **traits** held as `Arc<dyn _>` in a
  context struct, with `NoOp` default impls (e.g. `ProfileResolver`, `FuzzyChecker`).
- `tl-policy`: policy AST (`Policy`, `WhenClause`, `MatchClause`, `Matcher`, `Action`).
- `tl-storage`: `*Store` traits + Diesel migrations — workspace-scoped, composite PKs
  `(workspace_id, id)`, soft-delete `deleted_at`, partial unique indexes, `now()`.

Status: **draft for review.** Type/SQL sketches are illustrative; exact fields are
subject to the open decisions in `event-engine-design.md` §12.

---

## 1. Principles

- **Abstract core, swappable everything.** Each pipeline step is a trait; the
  engine composes `Arc<dyn _>` impls (mirrors `HandlerCtx`). Lets us swap
  in-proc↔Redis↔service, real↔NoOp, without touching call sites.
- **Wire types are the source of truth** — all new contracts live in `tl-core`;
  storage/SDK/web consume them.
- **Additive & workspace-scoped.** New tables follow `(workspace_id, …)`; trace
  evidence is added, never rewritten. Every new field is optional/defaulted.
- **Fail closed.** Unknown origin → untrusted; unregistered tool → high-impact;
  missing provenance for an authority param → escalate/block.
- **Enforcement is data, not a fork.** Every checker carries an `EnforcementMode`
  (`Off | Shadow | Enforce`) per workspace.

---

## 2. `tl-core` — type/contract design

New modules (one per concept, matching `guard.rs`/`run.rs`/…):

```
tl-core/src/
  event.rs        // GuardEvent, EventKind, Principal, Action, SideEffectClass
  label.rs        // Source, Origin, Trust, Confidentiality, Integrity, Labels
  provenance.rs   // ProvenanceMap
  tool.rs         // ToolMetadata, ParamRole, AllowedSource, ApprovalRule
  guard.rs        // (extend) Decision evidence; keep CheckRequest/Verdict
```

### 2.1 The event envelope

```rust
// event.rs — illustrative
pub struct GuardEvent {
    pub kind: EventKind,
    pub principal: Principal,
    pub action: Action,
    #[serde(default)] pub sources: Vec<Source>,
    #[serde(default)] pub provenance: ProvenanceMap,
    #[serde(default)] pub context: serde_json::Value,
}

#[serde(rename_all = "snake_case")]
pub enum EventKind {
    OutputProposed, ToolCallProposed,
    MemoryWriteProposed, MemoryRetrievalUsedForAction,
    FileActionProposed, ShellActionProposed,
    NetworkRequestProposed, BrowserActionProposed,
    DatabaseMutationProposed, ApiMutationProposed,
    ExternalMessageProposed,
}

pub struct Principal {           // extends today's CheckRequest identity
    pub workspace_id: String,
    pub environment_id: String,
    pub agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub run_event_id: Option<String>,
}

pub struct Action {
    pub operation: String,                 // "send_email", "output", …
    #[serde(default)] pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_effect: Option<SideEffectClass>,   // usually RESOLVED server-side from ToolMetadata
}

#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    None, Read, ExternalCommunication, FileWrite, ShellExec,
    NetworkCall, DbMutation, ApiMutation, MemoryWrite, Publish,
}
```

### 2.2 Sources, labels, provenance

```rust
// label.rs
pub struct Source {
    pub id: String,                         // "src_email_1"
    pub origin: Origin,                     // reported by producer (fact)
    #[serde(default)] pub labels: Labels,   // resolved/propagated (default unknown→fail-closed)
    #[serde(skip_serializing_if = "Option::is_none")] pub kind: Option<String>, // "email_body"…
}

#[serde(rename_all = "snake_case")]
pub enum Origin { User, System, Tool, Memory, File, Web, Email, Api, Unknown }

#[serde(rename_all = "snake_case")] pub enum Trust { Trusted, Untrusted, Unknown }
#[serde(rename_all = "snake_case")] pub enum Confidentiality { Public, Private, Secret, Identity }
#[serde(rename_all = "snake_case")] pub enum Integrity { Low, Medium, High }

pub struct Labels {                         // each carries a confidence in evidence
    #[serde(default)] pub trust: Trust,             // default Unknown (fail-closed)
    #[serde(default)] pub confidentiality: Confidentiality, // default Private for sensitive origins
    #[serde(default)] pub integrity: Integrity,
}

// provenance.rs — param/value path -> source ids
pub struct ProvenanceMap(pub BTreeMap<String, Vec<String>>);  // "action.parameters.to" -> ["src_email_1"]
```

### 2.3 Tool metadata (wire shape; persisted in `tl-storage`)

```rust
// tool.rs
pub struct ToolMetadata {
    pub tool: String,
    pub side_effect: SideEffectClass,
    pub reversible: bool,
    pub params: Vec<ParamSpec>,
    #[serde(skip_serializing_if = "Option::is_none")] pub approval: Option<ApprovalRule>,
    #[serde(skip_serializing_if = "Option::is_none")] pub sandbox_hint: Option<serde_json::Value>,
}
pub struct ParamSpec {
    pub path: String,                       // JSON path inc. nested: "transfer.dest_iban"
    pub role: ParamRole,                    // AuthorityBearing | ContentBearing
    #[serde(default)] pub allowed_sources: Vec<AllowedSource>, // e.g. [UserPrompt, ContactsLookup]
}
#[serde(rename_all="snake_case")] pub enum ParamRole { AuthorityBearing, ContentBearing }
```

### 2.4 Decision (extend existing `guard::Decision`)

Keep `trace_id`, `verdict: Verdict` (already `allow/block/rewrite/escalate`),
`reason`, `triggered_policies`, `tier_results`, `latency_ms`. Add **evidence**:

```rust
pub struct Decision {
    // … existing fields …
    #[serde(skip_serializing_if = "Option::is_none")] pub violated_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub remediation: Option<String>,
    #[serde(default)] pub source_chain: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub risk_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub failure_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub harm_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub constraints: Option<serde_json::Value>, // E1 sandbox output
}
```

**Compatibility:** `CheckRequest` stays; a normalizer maps it to
`GuardEvent { kind: OutputProposed, action.operation: "output", … }`.

---

## 3. `tl-policy` — policy model extension

Extend the existing AST with policy *families* (content stays as-is):

```rust
pub enum PolicyFamily { Content, Flow, ParameterSource, Approval, Memory }
// Flow: action-integrity + destination-permission predicates
// ParameterSource: per-param allowed-source rules (mirror ToolMetadata)
// each violation yields { triggered_policy, violated_rule, remediation }
```

Reuse `WhenClause` scoping (channel/agent/domain) + add event-kind / side-effect
scope. Keep parsing/validation in `tl-policy`.

---

## 4. `tl-engine` — processing classes & traits (abstract, swappable)

The pipeline is a sequence of **stage traits**, composed in an extended context
(same pattern as `HandlerCtx`). Default impls are `NoOp` → that's the Phase-0
skeleton.

```rust
// the spine — each is Send + Sync, held as Arc<dyn _>
pub trait Normalizer        { fn normalize(&self, raw: RawInput) -> GuardEvent; }
pub trait PrincipalResolver { fn resolve(&self, ev: &mut GuardEvent, auth: &AuthCtx); }
pub trait ToolMetadataProvider { fn get(&self, ws: &str, tool: &str) -> Option<ToolMetadata>; } // cached
pub trait LabelResolver     { fn resolve(&self, ev: &mut GuardEvent); }   // origin→trust + declared conf
pub trait ProvenanceResolver{ fn resolve(&self, ev: &mut GuardEvent); }   // structural ⊕ containment ⊕ fail-closed

pub trait Checker: Send + Sync {            // THE key abstraction
    fn id(&self) -> &str;
    fn mode(&self, ws: &str) -> EnforcementMode;          // Off | Shadow | Enforce
    fn check(&self, ev: &GuardEvent, ctx: &CheckCtx) -> CheckerVerdict; // evidence + proposed verdict
}

pub trait SignalProvider    { async fn signals(&self, ev: &GuardEvent) -> Vec<Signal>; } // LLM judges, advisory
pub trait DecisionComposer  { fn compose(&self, verdicts: &[CheckerVerdict], signals: &[Signal]) -> Decision; }
pub trait TracePersister    { fn enqueue(&self, ev: &GuardEvent, d: &Decision); }   // fire-and-forget
```

Concrete `Checker`s (each gated by `EnforcementMode`, run in-process, worst-verdict-wins):

| Checker | Source | Phase |
|---|---|---|
| `ContentChecker` | wraps today's 3 tiers | 0 |
| `InformationFlowChecker` | action-integrity + destination-permission | 4 |
| `ParameterAuthChecker` | vs `ToolMetadata.allowed_sources` | 5 |
| `MemoryChecker` | write-time block | 4 |

Engine flow (deterministic, in-process; signals parallel + deadline-bounded):

```text
RawInput → Normalizer → PrincipalResolver(+ToolMetadataProvider) → LabelResolver → ProvenanceResolver
        → [Checker]* (gated)  ⟂  SignalProvider (sheddable)
        → DecisionComposer → Decision → TracePersister
```

**Extended context** (mirrors `HandlerCtx`):

```rust
pub struct EngineCtx {
    pub tool_metadata: Arc<dyn ToolMetadataProvider>,
    pub label_resolver: Arc<dyn LabelResolver>,
    pub provenance: Arc<dyn ProvenanceResolver>,
    pub checkers: Vec<Arc<dyn Checker>>,
    pub signals: Arc<dyn SignalProvider>,     // LlmRouter today
    pub composer: Arc<dyn DecisionComposer>,
    pub cache: Arc<dyn DecisionCache>,        // moka → Redis (swap, no call-site change)
    pub traces: Arc<dyn TracePersister>,
}
```

Scalability levers all live behind these traits: `DecisionCache` (moka→Redis),
`ToolMetadataProvider` (cached registry read), `TracePersister` (mpsc→broker),
`SignalProvider` (in-proc→service). The decision path stays Rust, in-process, sub-ms.

---

## 5. `tl-storage` — DB design

Additive migrations in the existing numbered style. All workspace-scoped.

### 5.1 New: `tool_metadata` registry (Phase 2) — mirrors `policies`/`agents`

```sql
-- migrations/00000000000019_tool_metadata/up.sql (illustrative)
CREATE TABLE IF NOT EXISTS tool_metadata (
    workspace_id   TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tool           TEXT NOT NULL,
    side_effect    TEXT NOT NULL,
    reversible     BOOLEAN NOT NULL DEFAULT false,
    spec           JSONB NOT NULL,           -- params[] (path, role, allowed_sources), approval, sandbox_hint
    enabled        BOOLEAN NOT NULL DEFAULT true,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at     TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, tool)
);
CREATE INDEX IF NOT EXISTS tool_metadata_active_idx
    ON tool_metadata (workspace_id) WHERE deleted_at IS NULL;
```

### 5.2 New: `source_label_policy` (Phase 3) — the origin→trust table, per workspace

```sql
-- global defaults live in code (fail-closed); this table is for overrides
CREATE TABLE IF NOT EXISTS source_label_policy (
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    origin          TEXT NOT NULL,           -- email/web/user/tool/file/memory/api
    trust           TEXT NOT NULL,           -- trusted/untrusted/unknown
    confidentiality TEXT,                    -- optional default
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, origin)
);
```

### 5.3 Extend: `traces` (Phase 1) — promote evidence for dashboard filtering

Keep the full enriched `Decision` in `payload JSONB` (as today). Add **columns
only for what the dashboard filters/aggregates on** (decision of §12 #2):

```sql
ALTER TABLE traces ADD COLUMN IF NOT EXISTS event_kind   TEXT;
ALTER TABLE traces ADD COLUMN IF NOT EXISTS risk_source  TEXT;
ALTER TABLE traces ADD COLUMN IF NOT EXISTS failure_mode TEXT;
ALTER TABLE traces ADD COLUMN IF NOT EXISTS harm_class   TEXT;
CREATE INDEX IF NOT EXISTS traces_workspace_event_kind_idx
    ON traces (workspace_id, event_kind, created_at);
```
`sources` / `provenance` / `source_chain` live in `payload` (flexible, not filtered).

### 5.4 Deferred: `memory_provenance` (T3, later) — sketched, not built in v1

```sql
-- enables cross-session lineage; anchored on runs. NOT v1 (write-time block covers v1).
-- memory_provenance(workspace_id, mem_key, source_chain JSONB, writer_run_id, trust, created_at)
```

### 5.5 Repository traits (extend the `*Store` pattern)

```rust
pub trait ToolMetadataStore: Send + Sync {
    async fn upsert(&self, ws: &str, m: &ToolMetadata) -> Result<(), StorageError>;
    async fn get(&self, ws: &str, tool: &str) -> Result<Option<ToolMetadata>, StorageError>;
    async fn list(&self, ws: &str) -> Result<Vec<ToolMetadata>, StorageError>;
}
pub trait SourceLabelPolicyStore: Send + Sync { /* get_overrides(ws) */ }
// TraceStore: extend NewTrace/TraceRow with the new columns; writer unchanged otherwise.
```
Both registries are read-heavy/write-rare → wrap in the existing `moka` cache
(→ Redis at multi-replica). `ToolMetadataProvider`/`LabelResolver` read through them.

---

## 6. Scalability & abstraction summary

- **Sharding key:** `workspace_id` on every table (cell-able later).
- **Hot path:** all deterministic, in-process; registries cached; no per-event DB read on cache hit.
- **Traces:** partitioned (exists) + indexed columns for filters + JSONB for flexible evidence; sink behind `TracePersister` (mpsc→broker→OLAP).
- **Swap points (traits):** cache, tool-metadata, label resolver, provenance, signals, trace sink — each `Arc<dyn _>`, `NoOp` default, independently replaceable.
- **Fail-closed defaults** baked into `LabelResolver`/`ParameterAuthChecker`.

---

## 7. Phase mapping

| Phase | tl-core | tl-engine | tl-storage |
|---|---|---|---|
| 0 | event/label/provenance/tool types + Decision evidence + normalizer | stage traits + `NoOp` defaults; `ContentChecker` wraps tiers | — (golden replay) |
| 1 | — | producer surface | `traces` evidence columns (§5.3) |
| 2 | `ToolMetadata` | `ToolMetadataProvider` | `tool_metadata` (§5.1) |
| 3 | — | `LabelResolver` + propagation | `source_label_policy` (§5.2) |
| 4 | — | `InformationFlowChecker` + `MemoryChecker` (write-time) | — |
| 5 | — | `ParameterAuthChecker` | — |
| 6 | policy families + evidence | `DecisionComposer` + approvals | policy schema extend |
| later | constraints (E1) | sandbox adapter | `memory_provenance` (§5.4) |

---

## 8. Dependencies on open decisions (`event-engine-design.md` §12)

- **#1 evidence authority** → whether `Source.labels` is required-on-input (rich SDK)
  or defaulted-then-resolved server-side (recommended split). Affects `#[serde(default)]`
  on `Labels` and where `LabelResolver` runs. *Default here: server-derived.*
- **#2 trace persistence** → which evidence becomes columns vs JSONB (§5.3 reflects
  the "few indexed columns" choice).
- **#4 ToolMetadata ownership** → default for unregistered tool = fail-closed
  (high-impact / escalate); `ToolMetadataProvider.get` returns `None` → conservative.
- **#5 policy families** → §3 shape.

These don't block Phase 0 (the types are additive), but #1 should be confirmed
before the `Source`/`Labels` fields are finalized.
