# Red-team harden (policy synthesis)

Harden turns the attacks that **landed** in a completed red-team job into
guardrail policies — verified before they are recommended. It is the closed loop
behind the product promise: run the same attacks guarded and watch the success
rate fall.

## Where it sits

```text
Attacks tab ──POST /api/redteam/jobs/{id}/harden──▶ Next proxy ──▶ Rust
                                                                    │
   POST /v1/redteam/jobs/{id}/harden  (crates/tl-server redteam::harden)
                                                                    │
        load job results ─▶ classify ─▶ synthesize ─▶ verify ─▶ recommend
```

Synthesis is guardrail business logic, so Rust owns it: the pure classification
and policy construction live in `tl-policy` (`synthesis`); the orchestration,
verification, and persistence live in `tl-server` (`redteam::harden`). The web
app is a thin proxy.

## What it does

For each landed, non-control attack in the job:

1. **Classify** the harm mechanism from the attack text — credential, PII,
   system-prompt, action-claim, or protected-info. Landed attacks produced no
   blocking finding by definition, so the agent reply is the richest signal.
2. **Synthesize** a candidate generalized to that *class*, not the exact string
   it leaked: a [semantic matcher](glossary.md#matcher) whose clause the runtime
   LLM judge evaluates, plus a regex backstop for credentials. One policy per
   class (stable id, so re-hardening upserts in place).
3. **Verify** the candidate through the *real* evaluator (`evaluate_event_policies`)
   against the landed replies, generated obfuscation variants, and the benign
   control cases. A candidate is kept only if it blocks every landed case and
   false-blocks no control — a policy that protects nothing is never recommended.
4. **Recommend** the survivors. They persist `enabled = false`; an operator opts
   in via `PATCH /v1/policies/{id}/enabled`, exactly like `guardrails:generate`.

## Inputs and outputs

- **Input** — a completed job's per-attack results (`redteam_job_results`), plus
  an optional `persist` flag (preview vs. save).
- **Output** — `HardenResponse`: a list of `HardenCandidate`s (the persisted
  policy, its substrate, the evidence cases, and the verify result), plus an
  `unreachable` list naming substrates a landed attack needed but the job's
  traces could not reach.

## Reachable substrates

Today the chat red-team produces output-level events, so candidates use the
text-matcher substrate (semantic, with a regex backstop). Action-level defenses
(approval gates, parameter-source authorization) require structured tool-call
traces; until those exist, an action attack is covered by a semantic matcher on
its claim, and any class needing an event-level defence is reported as
`unreachable` rather than silently approximated.

## Ownership

- Wire types — `crates/tl-core` (`HardenRequest`, `HardenResponse`,
  `HardenCandidate`, `VerifyResult`).
- Classification + synthesis — `crates/tl-policy` (`synthesis`).
- Endpoint, verify loop, persistence — `crates/tl-server` (`redteam::harden`,
  `redteam::verify`).
- Verification reuses the engine evaluator and the runtime semantic judge, so a
  candidate's verdict matches production. See [event-engine.md](event-engine.md).

Related: [redteam-dispatch.md](redteam-dispatch.md) (the jobs harden reads from).
