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
        load job sessions ─▶ classify ─▶ synthesize ─▶ verify ─▶ recommend
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
4. **Recommend** the survivors. A survivor either creates the stable harden
   policy for that agent + harm class, or tightens the existing one with the same
   id. New survivors persist `enabled = false`; tightened policies keep their
   previous environment enabled state.

Candidates that fail verification are returned as rejections with a public
reason (`missed_landed`, `missed_variant`, `false_blocked_control`,
`semantic_judge_unavailable`, and similar). Rejections are not HTTP errors; they
explain why the UI should hand the operator to manual policy authoring instead
of pretending a reliable automatic guardrail exists.

## Inputs and outputs

- **Input** — a completed job's landed attack sessions and their `target_reply`
  events, plus an optional `persist` flag (preview vs. save).
- **Output** — `HardenResponse`: a list of `HardenCandidate`s (the persisted
  policy, whether it will `create` or `tighten`, its substrate, the evidence
  cases, and the verify result), a list of rejected attempts with reasons, plus
  an `unreachable` list naming substrates a landed attack needed but the job's
  traces could not reach.

## Outcome model

| Outcome | Meaning | UI action |
|---|---|---|
| `create` candidate | The stable harden policy id does not exist yet. | Show a new guardrail and let the operator turn it on + test again. |
| `tighten` candidate | The stable harden policy id already exists. | Show that the existing guardrail will be tightened; preserve its enabled state. |
| rejection | A synthesized candidate did not pass verification. | Show the reason and route to policy authoring. |
| unreachable | The landed case needs a substrate this job could not verify. | Make the coverage gap explicit. |

The dashboard renders this model only. It does not infer whether to create or
tighten a rule, and it does not synthesize fallback policies.

## Reachable substrates

Today the chat red-team produces output-level events, so candidates use the
text-matcher substrate (semantic, with a regex backstop). Action-level defenses
(approval gates, parameter-source authorization) require structured tool-call
traces; until those exist, an action attack is covered by a semantic matcher on
its claim, and any class needing an event-level defence is reported as
`unreachable` rather than silently approximated.

## Ownership

- Wire types — `crates/tl-core` (`HardenRequest`, `HardenResponse`,
  `HardenCandidate`, `HardenRejection`, `VerifyResult`).
- Classification + synthesis — `crates/tl-policy` (`synthesis`).
- Endpoint, verify loop, persistence — `crates/tl-server` (`redteam::harden`,
  `redteam::verify`).
- Verification reuses the engine evaluator and the runtime semantic judge, so a
  candidate's verdict matches production. See [event-engine.md](event-engine.md).

Related: [redteam-dispatch.md](redteam-dispatch.md) (the jobs harden reads from).
