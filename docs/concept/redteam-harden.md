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
        load job sessions ─▶ classify ─▶ draft/synthesize ─▶ verify/replay ─▶ recommend
```

Synthesis is guardrail business logic, so Rust owns it: the pure classification
and policy construction live in `tl-policy` (`synthesis`); the orchestration,
LLM draft seam, verification, and persistence live in `tl-server`
(`redteam::harden`). The web app is a thin proxy.

## What it does

For each landed, non-control attack in the job:

1. **Classify** the harm mechanism from the attack text — credential, PII,
   system-prompt, workflow-integrity, action-claim, or protected-info. When the
   job targets an agent with `workflow_requirements`, those requirements are
   used as domain context before the built-in heuristics. Landed attacks produced
   no blocking finding by definition, so the agent reply is the richest signal.
2. **Draft or synthesize** a candidate generalized to that *class*, not the exact
   string it leaked. When the configurable `harden_draft` LLM route is present,
   the model drafts a strict JSON policy candidate from the landed evidence.
   Rust then enforces the stable policy id, validates the policy AST, and drops
   invalid output. When the route is absent, deterministic synthesis remains the
   fallback for the content-policy substrate. Either path emits one policy per
   class (stable id, so re-hardening upserts in place).
3. **Verify** the content candidate through the *real* evaluator (`evaluate_event_policies`)
   against the landed replies, generated obfuscation variants, and the benign
   control cases. A candidate is kept only if it blocks every landed case and
   false-blocks no control — a policy that protects nothing is never recommended.
4. **Replay event artifacts** when the job includes structured action evidence.
   For workflow/action attacks with captured `guard_event` tool calls, harden
   can build approval metadata, parameter-source metadata, or source-label
   policy candidates. It overlays the proposed `ToolMetadata` onto each landed
   action event and runs the real event checker (`ApprovalChecker` or
   `ParameterAuthChecker`). For label policies, it replays captured labels and
   provenance through `InformationFlowChecker` and `MemoryChecker`, using
   label-resolution basis evidence so workspace-owned overrides can be tightened
   while producer-declared labels stay authoritative. The artifact is returned
   only if it newly stops the landed actions and does not stop matching benign
   controls.
5. **Recommend** the survivors. A content survivor either creates the stable harden
   policy for that agent + harm class, or tightens the existing one with the same
   id. New survivors persist `enabled = false`; tightened policies keep their
   previous environment enabled state. Event metadata and source-label policy
   survivors persist disabled when new and preserve the existing enabled state
   when tightening a registry row. Each candidate reports its `source` (`llm` or
   `deterministic`) so the UI can show how the draft was produced.
6. **Promote** verified survivors into regression cases when explicitly
   requested. `promote_regression = true` upserts one durable
   `redteam_regression_cases` row per survivor, keyed by source job, substrate,
   artifact, and evidence seqs. Re-hardening refreshes the same case instead of
   duplicating it.

Candidates that fail verification are returned as rejections with a public
reason (`missed_landed`, `missed_variant`, `false_blocked_control`,
`semantic_judge_unavailable`, and similar). Rejections are not HTTP errors; they
explain why the UI should hand the operator to manual policy authoring instead
of pretending a reliable automatic guardrail exists.

## Inputs and outputs

- **Input** — a completed job's landed attack sessions and their `target_reply`
  events, optional agent-profile workflow requirements, plus an optional
  `persist` flag (preview vs. save) and `promote_regression` flag (durable eval
  case promotion).
- **Output** — `HardenResponse`: `candidates` for content policies,
  `event_candidates` for verified tool-metadata artifacts such as approval
  `ToolMetadata` or parameter-source `ParamSpec.allowed_sources`,
  `label_policy_candidates` for verified `SourceLabelPolicy` artifacts, a list
  of rejected attempts with reasons, an `unreachable` list naming substrates a
  landed attack needed but the job's traces could not reach, and
  `regression_cases` when promotion was requested.

## Outcome model

| Outcome | Meaning | UI action |
|---|---|---|
| `create` candidate | The stable harden policy id does not exist yet. | Show a new guardrail and let the operator turn it on + test again. |
| `tighten` candidate | The stable harden policy id already exists. | Show that the existing guardrail will be tightened; preserve its enabled state. |
| event candidate | A structured event artifact, such as approval or parameter-source tool metadata, passed replay verification. | Show the artifact, upsert it enabled, then test again. |
| label-policy candidate | A source-label policy artifact passed information-flow replay verification. | Show the artifact, upsert it enabled, then test again. |
| rejection | A synthesized candidate did not pass verification. | Show the reason and route to policy authoring. |
| unreachable | The landed case needs a substrate this job could not verify. | Make the coverage gap explicit. |

The dashboard renders this model only. It does not infer whether to create or
tighten a rule, and it does not synthesize fallback policies.

## Reachable substrates

Chat-only red-team jobs produce output-level events, so content candidates use
the text-matcher substrate (semantic, with a regex backstop). Action-level
defenses require structured tool-call traces. Runner session events can carry
optional `guardEvent` evidence, which Rust persists as
`RedteamSessionEvent.guard_event`.

Approval hardening, parameter-source hardening, and source-label policy
hardening are implemented for those structured action traces:

- output-only workflow/action jobs report `unreachable = ["approval"]`;
- jobs with action `guard_event` evidence emit `event_candidates`;
- each approval candidate is replay-verified with `ApprovalChecker`;
- each parameter-source candidate derives allowed origins/kinds from benign
  control provenance and is replay-verified with `ParameterAuthChecker`;
- label-policy candidates tighten workspace-owned label families by replaying
  current labels, proposed labels, and derived provenance through
  `InformationFlowChecker` or `MemoryChecker`;
- `persist: true` stores new tool metadata and source-label policies disabled,
  and the dashboard can enable the returned artifact before rerunning the
  red-team job.

Missing-provenance remediation and graph-derived event artifacts remain planned.

## LLM Configuration

LLM drafting is a control-plane helper, not the final runtime authority. It is
configured through the same `tl-llm::LlmRouter` used by semantic policy
matching:

```toml
[routes.harden_draft]
primary = { provider = "openai", model = "gpt-4o-mini", deadline_ms = 5000 }
```

The model output is only a candidate. The server converts it to a `Policy`,
validates it through `tl-policy::validate_policy`, then runs the verify loop
before the candidate can be returned or persisted. Swapping model/provider means
changing the route config or adding a new `LlmClient` implementation; harden
logic calls `JudgeKind::HardenDraft` and never depends on provider-specific code.

`GET /v1/runtime/llm-status` reports whether `semantic_policy`,
`harden_draft`, and `trajectory_diagnostic` routes are configured, plus the full
list of known LLM route keys. It intentionally does not expose secrets or
provider environment values.

## Ownership

- Wire types — `crates/tl-core` (`HardenRequest`, `HardenResponse`,
  `HardenCandidate`, `HardenEventCandidate`, `HardenLabelPolicyCandidate`,
  `HardenRejection`, `VerifyResult`, `EventVerifyResult`).
- Classification + synthesis — `crates/tl-policy` (`synthesis`). Synthesis is
  pure; `tl-server` loads any agent workflow requirements and passes them in as
  context.
- LLM prompt/schema and routing — `crates/tl-llm`
  (`prompts::harden_draft`, `prompts::trajectory_diagnostic`,
  `JudgeKind::HardenDraft`, `JudgeKind::TrajectoryDiagnostic`). Providers are
  swappable through `config/llm-routing.toml`.
- Endpoint, verify/replay loop, persistence, regression promotion —
  `crates/tl-server`
  (`redteam::harden`, `redteam::harden_draft`, `redteam::verify`). Approval and
  parameter-source event candidates persist through the same `ToolMetadataStore`
  used by `/v1/tool-metadata`; label-policy candidates persist through the same
  `LabelPolicyStore` used by `/v1/label-policies`; promoted regression cases
  list at `GET /v1/redteam/regressions`.
- Verification reuses the engine evaluator and the runtime semantic judge, so a
  candidate's verdict matches production. See [event-engine.md](event-engine.md).

Related: [redteam-dispatch.md](redteam-dispatch.md) (the jobs harden reads from).
