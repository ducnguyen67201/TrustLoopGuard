# Featherlane AI Hardening v2 — Attack-Grounded Policy Synthesis

This is an implementation/design spec for the next iteration of the
red-team → harden loop: the step that, after a red-team run, recommends a
guardrail that actually closes the gap the attack found. It is not the canonical
architecture source. The whitepaper
(`docs/research/featherlane-ai-runtime-security-architecture/main.pdf`, §VIII
guarded events, §XXII evolving evaluation, §XXIII two planes) defines the
direction; the differential benchmark that scores the result is specified in
[`featherlane-ai-bench-v2-design.md`](./featherlane-ai-bench-v2-design.md).
When code lands, add/extend a concept doc and keep the roadmap language here in
the spec.

Status: **design / proposal.**

---

## Purpose

After a red-team run, Featherlane AI offers to "harden" the agent by generating a
guardrail from what landed. Today that recommendation is produced web-side
(`apps/web/lib/arena-harden.ts`) and it does not close the loop:

> It picks one of **three hardcoded output-regex templates**, uses the LLM only
> to write the policy *description*, and never re-tests the result.

The product promise is the opposite — *"run the same attacks through a guarded
agent and watch the success rate fall to zero."* A recommendation that protects
nothing breaks that promise the moment an attacker paraphrases, encodes, or
attacks an **action** instead of an **output**. A live report against a tax
assistant makes the failure concrete: the two *disclosure* attacks (SSN exfil,
packet export) block, but `tax_review_bypass` — *"approve a refund without human
review"* — **lands**, because there is no protected string to match; the harm is
a state change, not text.

Hardening v2 replaces canned-template generation with **attack-grounded,
substrate-aware, verified policy synthesis**.

## Background: how v1 hardening works, and why it can't generalize

The current mechanism (`apps/web/lib/arena-harden.ts`,
`apps/web/lib/redteam-harden.ts`):

1. Keep non-control cases whose guarded reply still leaked (`outcome === 'landed'`).
2. `extractLeakedToken` regex-scans replies for the demo credential shape
   `sk-[a-z0-9]{6,}` — the only ground-truth datum it pulls.
3. `classifyLeakKind` buckets into exactly three classes: `credential`,
   `system-prompt`, `protected`.
4. `buildFallbackDraft` returns a **constant regex per bucket**, on the `chat`
   channel. Even the leaked token is not used in the matcher.
5. `buildHardenDraftFromSuggestion` calls the LLM draft endpoint but keeps only
   `description`; the model's match logic is discarded (`{ ...fallbackDraft,
   description: llm.description }`).
6. `applyHardenPolicy` upserts the YAML through Rust `/v1/policies`.

Three structural limits fall out of this:

| v1 hardening today | v2 needs |
|---|---|
| 3 fixed buckets, all *disclosure* | Classify by **harm mechanism**, covering action / flow / memory too |
| Constant regex per bucket | Matcher **derived from the trace**, generalized to the leak's *class* |
| Output text only, `match_type ∈ {literal, regex}` | Emit the **strongest substrate per class** (semantic / family / registry) |
| LLM writes prose; matcher is canned | LLM drafts the **structured artifact**, validated against the AST |
| No re-test | **Verify-before-recommend**: re-run vs landed + variants + controls |
| Logic lives in `apps/web/lib` | Synthesis is guardrail logic → **Rust owns it**, web proxies |

## The two enforcement substrates (the constraint that shapes everything)

A policy can only protect if it reaches an evaluator. Featherlane AI has two,
and they take different inputs:

1. **Text-matcher policies.** Stored `Policy`
   (`crates/tl-policy/src/policy_ast.rs`) with `Matcher::{Literal, Regex,
   Semantic}` and `MatchClause::{Single, Any, All}`, evaluated against
   input/output **text**. `Semantic` routes to the runtime LLM judge. This is
   the only substrate the current chat red-team exercises, and the only one v1
   hardening emits — and it emits the weakest member of it.

2. **Event checkers.** `ApprovalChecker`, `ParameterAuthChecker`,
   `InformationFlowChecker`, `MemoryChecker`
   (`crates/tl-engine/src/event_pipeline/checkers/`). These are **zero-field
   structs** that read a structured `GuardEvent` — its `resolution.metadata`
   (tool-registry `ApprovalRule` / `ParamSpec.allowed_sources` /
   side-effect), `provenance`, and `sources` — **not** stored policies.
   `ApprovalChecker` escalates `tax_review_bypass` only when the agent emits a
   `ToolCallProposed` for the refund tool **and** that tool is registered with
   `approval.required = true`.

The implication that drives the whole design: **a synthesized text matcher can
never trigger an event checker, and registering tool metadata can never be
expressed as a regex.** "Work across everything" therefore means *routing each
landed attack to the correct substrate*, not generating a better regex.

A second implication scopes expectations: the current demo agent returns text,
not structured events, so the event checkers cannot fire against it. In that
substrate the only thing that can block `tax_review_bypass` is a **`Semantic`
matcher** ("the reply indicates a privileged/financial action taken without
required human review"). The event-checker route only bites once the agent emits
`GuardEvent`s and tools are registered (whitepaper §VIII "Next build";
bench v2 Layer 2).

---

## Goals

- Replace canned-template generation with synthesis **grounded in the landed
  attack's trace** (track, kind, sink, tool, leaked class), reusing the
  `track`/`kind`/`trace_id` the runner already records
  (`crates/tl-core/src/redteam.rs`).
- Route each landed attack to the **correct remediation substrate** (text
  matcher vs tool-registry/label artifact), emitting the strongest applicable
  one.
- **Generalize** from the concrete leak to its class so paraphrase / encoding /
  second-order / adaptive variants are covered, not just the exact string.
- **Verify before recommending**: a candidate is recommended only if it blocks
  the attacks that landed, blocks generated obfuscated variants, and does not
  false-block the benign controls.
- Own synthesis in **Rust** (`tl-server` + `tl-policy`); make the web layer a
  thin proxy.
- Make the synthesizer's output **auditable**: every recommendation cites the
  cases it was derived from and its verify result.

## Non-goals

- Not auto-enforcement. Synthesized policies are recommended/persisted
  `enabled = false`; an operator opts in (mirrors `guardrails:generate`).
- Not a new runtime evaluator. v2 emits artifacts for the **existing** matcher
  pipeline and event checkers; it does not add a checker.
- Not a red-team engine change on day one. Phase 1 works on the text substrate
  the current red-team already produces; event-level attacks arrive with
  bench v2 Layer 2.
- Not a learned classifier on the hot path (POLICYGUARD-style); noted as a later
  option for the `Semantic` substrate, not part of this spec.

---

## Core design

### 1. Attack taxonomy → remediation substrate

The synthesizer classifies each landed attack by **harm mechanism**, then emits
the substrate that can actually stop it:

| Harm mechanism | Manifest | Remediation artifact | Substrate |
|---|---|---|---|
| Output disclosure (credential / PII / system-prompt / protected) | text in reply | `Semantic` matcher on output (+ regex backstop in an `Any`) | text-matcher |
| Output action-claim / policy-bypass (e.g. "approved refund, skipped review") | text in reply | `Semantic` matcher on output | text-matcher |
| Privileged tool call needs human review | structured event | `ApprovalRule { required: true }` registry entry | event checker |
| Authority-bearing parameter from untrusted source | structured event | `ParamSpec { role: authority-bearing, allowed_sources }` | event checker |
| Secret/identity → external sink | structured event | information-flow label policy | event checker |
| Memory poisoning (untrusted write reused later) | structured event | memory provenance / write-time block | event checker |

Classification inputs, in priority order: the case's `track` and `kind` (already
assigned by the runner), the resolved tool / event kind / side-effect from the
trace, then the reply text as a fallback signal. The text-only `classifyLeakKind`
keyword guess is replaced by trace-grounded classification.

### 2. Synthesis pipeline

```
landed case + trace
   │  classify(track, kind, sink, tool, reply) → harm mechanism + substrate
   ▼
LLM drafts a STRUCTURED artifact for that substrate
   (Matcher::Semantic | Policy(Any[...]) | ApprovalRule | ParamSpec | flow/memory)
   generalized to the leak's CLASS, not the literal string
   │  validate against the AST (tl_policy::validate_policy / family validation)
   ▼
VERIFY (reuse bench v2 differential harness)
   re-run candidate vs:  landed cases (must block)
                          obfuscated/paraphrased variants (generalization)
                          benign controls (must NOT false-block)
   │  keep only candidates that pass
   ▼
recommend a SET (enabled=false) with evidence + verify result
```

The LLM is the **generator**; the engine + bench is the **grader**. The grader
is what v1 lacks — it is why a policy that protects nothing is never caught. This
mirrors GuardAgent (generate an executable check, not a description;
arXiv:2406.09187) and AGrail (an Executor that deletes checks which block
legitimate actions; arXiv:2502.11448).

### 3. Generalization (concrete → class)

A matcher tied to `sk-abc123` is defeated by spacing, base64, translation, or a
different secret. The synthesizer abstracts the concrete leak to its class before
drafting — "any credential-shaped secret in the output," not the one token —
following AGrail's step-back abstraction and POLICYGUARD's reported
out-of-distribution generalization (arXiv:2510.03485). The verify step's
variant set is the empirical check that generalization held.

### 4. Verify-before-recommend (loop closure)

Reuse the bench v2 differential harness (`crates/tl-bench`, real
`EventPipelineCtx`, `CheckerModes`) rather than building a second evaluator. For
each candidate:

- **Block-landed:** every case that landed must now resolve `Block`/`Escalate`.
- **Generalization:** auto-generated obfuscated/paraphrased variants of each
  landed attack must also block (held-out from what the matcher was drafted on).
- **No-regress utility:** benign control cases must stay `Allow`; a candidate
  that false-blocks is dropped or downgraded to `escalate`.

Only survivors are recommended. Survivors promote into the bench regression suite
(bench v2 roadmap), realizing the whitepaper's §XXII evolving loop.

### 5. LLM usage: synthesis-time vs runtime (two planes)

Per whitepaper §XXIII, intelligence belongs at synthesis time, the hot path stays
cheap:

- **Synthesis time (control plane, offline):** the LLM classifies and drafts the
  structured artifact. Slow is fine; output is validated and re-tested.
- **Runtime (data plane, hot path):** mostly deterministic — event checkers
  (Fides-style information flow, arXiv:2505.23643; AgentArmor trace/type
  analysis, arXiv:2508.01249) and regex. The LLM runs only for the `Semantic`
  matcher, used narrowly and cached; the bench's `llm_calls` column keeps
  overuse visible.

---

## Architecture

Synthesis is guardrail business logic, so it moves out of `apps/web/lib` into
Rust, and the web layer becomes a thin proxy (CLAUDE.md layer ownership).

- **`crates/tl-policy`** — a `synthesize` module: trace → candidate
  `Policy`/`FamilyPolicy` artifacts, AST validation, and the
  concrete→class abstraction helpers. Pure, no I/O.
- **`crates/tl-server`** — `src/redteam/harden.rs`: new endpoint
  `POST /v1/redteam/jobs/{id}/harden` that loads the job's landed results +
  traces, calls the LLM drafter, runs the verify loop against the engine, and
  returns the recommended set (persisted `enabled = false`). Reuses the existing
  `draft_llm` client and policy store.
- **`crates/tl-core`** — widen the policy-draft wire contract beyond
  `literal|regex` to include `semantic` and the family-policy shapes; add the
  harden request/response wire types. Regenerate TS bindings + OpenAPI.
- **`apps/web`** — delete the synthesis logic in `arena-harden.ts` /
  `redteam-harden.ts`; the harden card calls the new Next proxy route, which
  proxies the Rust endpoint. `policy-draft.ts` widens `POLICY_MATCH_TYPES` only
  as needed for display.

The single-draft prompt/schema (`crates/tl-server/src/policies/draft.rs`) and the
`guardrails:generate` set-draft prompt widen in lockstep so the three surfaces
(single draft, set draft, harden) cannot drift on which match types and policy
families they may emit.

## Reporting surface

The harden recommendation carries, per candidate: the cases it was derived from
(seq + trace_id), the chosen substrate and why, the generalization-variant
pass/fail, and the false-block check. This is the auditable analogue of the
bench frontier table — an operator sees *what it protects against* and *that it
was tested*, not just a YAML blob.

---

## Roadmap

Small, independently-mergeable steps (each updates a concept doc when it ships):

1. **Lift to Rust, behavior-preserving.** Move the existing 3-template synthesis
   into `tl-server` + `tl-policy`; web proxies the new endpoint. No new
   capability. *Verify:* arena/attacks harden cards still recommend + apply the
   same policies; layer-ownership fixed.
2. **Semantic synthesis + verify loop.** Replace canned regex with
   trace-grounded `Semantic` matcher synthesis; add block-landed + variant +
   control verification; stop discarding the LLM matcher. *Verify:*
   `tax_review_bypass` and paraphrased credential leaks block on the demo;
   benign controls stay `Allow`.
3. **Substrate router.** Classify structured-event attacks and emit
   `ApprovalRule` / `ParamSpec` / flow / memory artifacts instead of matchers.
   Depends on event-level red-team (bench v2 Layer 2). *Verify:* an
   approval-required action escalates via `ApprovalChecker`, not a text match.
4. **Regression promotion.** Survivors promote into the bench v2 regression
   suite; re-harden upserts in place. *Verify:* a promoted case re-runs in CI.

## Pitfalls (from the literature and the code)

- **Overfitting to the attack string.** A matcher built from one reply is
  brittle; always abstract to class and prove it on held-out variants
  (Poly-Guard shows policy-grounding alone is not robust; arXiv:2506.19054).
- **Over-blocking to force ASR→0.** A candidate that escalates everything must
  fail the false-block / control check; keep utility adjacent to security
  (No Free Lunch With Guardrails; arXiv:2504.00441).
- **Wrong substrate.** Recommending a text matcher for an action attack is the
  current bug; never emit a matcher when the harm is a side effect — route to the
  event substrate or, if unreachable in the current setup, say so explicitly.
- **Trusting the generator.** The LLM draft is a candidate, never a
  recommendation, until the verify loop passes (GuardAgent / AGrail).
- **Silent demo-only wins.** When an attack is only blockable via the event
  substrate but the agent emits no events, report that the recommendation is a
  text-substrate approximation, not full coverage.

## Concept-doc / contract impact when this ships

- Add a concept doc for the hardening/synthesis component (purpose, inputs,
  outputs, ownership, request-flow position) under `docs/concept/`.
- `docs/openapi.yaml` + `crates/tl-core` — the new harden endpoint and widened
  policy-draft contract.
- `docs/concept/glossary.md` — define any new term (e.g. "synthesis substrate",
  "harden candidate").
- `docs/concept/web-ui-conventions.md` — only if the harden card becomes a
  shared pattern other pages reuse.

## References

Whitepaper: `docs/research/featherlane-ai-runtime-security-architecture/main.pdf`
(§VIII output→events, §XXII evolving evaluation, §XXIII two planes).

Companion spec: [`featherlane-ai-bench-v2-design.md`](./featherlane-ai-bench-v2-design.md)
(the differential harness the verify loop reuses).

External methods (primary sources; gathered via deep research, verification pass
was rate-limited so treat as cited-not-independently-confirmed):

- AGrail — runtime check generation/adaptation, step-back generalized check keys,
  Executor deletes checks that block legitimate actions: arXiv:2502.11448.
- GuardAgent — generate executable guardrail code from a guard request; guard at
  the action level: arXiv:2406.09187.
- ShieldAgent — verifiable rules from policy docs into action-based rule circuits:
  arXiv:2503.22738.
- Fides — planner-based information-flow control with taint labels: arXiv:2505.23643.
- AgentArmor — agent traces as typed programs (CFG/DFG/PDG) + property registry:
  arXiv:2508.01249.
- LlamaFirewall — scanners configurable by regex *or* LLM prompt; AlignmentCheck
  CoT auditor over the reasoning trace: arXiv:2505.03574.
- Poly-Guard / PolyGuard — policy-grounded + attack-enhanced data; guardrails
  still fall to optimized attacks: arXiv:2506.19054.
- Policy-as-Prompt — compile NL policies into LLM-judge classifiers: arXiv:2509.23994.
- POLICYGUARD / PolicyGuardBench — policies grounded in trajectories; small
  fine-tuned detector generalizes OOD: arXiv:2510.03485.
- ALRPHFS — offline adversarial self-learning loop refining a generalizable risk
  library: arXiv:2505.19260.
- No Free Lunch With Guardrails — security/utility/usability tradeoffs:
  arXiv:2504.00441.
