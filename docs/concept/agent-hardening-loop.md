# Agent-hardening loop

The hardening loop solves the firewall's cold-start: a user imports an agent,
the system generates **attacks tailored to that agent's own definition**, runs
them, synthesizes **verified guardrail policies** from what lands, and the user
iterates. The exploit proves the policy — there is no blank policy page.

This doc owns the loop and its one new piece, the **attack-vector planner**. The
run and synthesis steps it stitches together have their own homes:
[redteam-dispatch.md](redteam-dispatch.md) and [redteam-harden.md](redteam-harden.md).

## The loop

```text
 import agent (chat prompt OR workflow JSON)
        │
        ▼
 [redteam:plan]  agent definition ─▶ tailored attack vectors        ← NEW
        │            (+ workflow source→sink paths)
        ▼
 run the vectors as seeds  (POST /v1/redteam/dispatch)              ← reuse
        │            observed-behavior scoring (sink/guard)
        ▼
 [harden]  landed ─▶ verified policies, attached to the agent       ← reuse
        │            enabled = false
        ▼
 refine agent & re-run ─ before/after (previously-landed → blocked) ← loop
        └──────────────────── repeat ───────────────────┘
```

Two of the three steps already existed. The loop adds the **missing middle**
(agent definition → tailored vectors), generalizes import to any agent kind
(see [glossary: agent profile](glossary.md#agent-profile) — now optionally
carrying a `workflow_definition` and a `target_url` connection), and the loop UX
on the Attacks tab. The agent owns its connection: `target_url` is captured at
import, so the Attacks page is **agent-first** — pick an agent and its endpoint +
saved plans load, no re-typing.

## Attack-vector planner (`redteam:plan`)

`POST /v1/agents/{id}/redteam/plan` derives tailored [attack vectors](glossary.md#attack-vector)
from an agent's own definition. It is the attack-side twin of
`guardrails:generate` and shares its plumbing (the agent store + the
structured-output LLM client), so the two read identically:

1. **Fetch** the agent. Require something to plan from — a `system_prompt` and/or
   a `workflow_definition` — else `422`.
2. **Analyse** the workflow graph (when present). The static
   `workflow_analyzer` classifies n8n nodes into **untrusted sources**
   (webhook, form trigger, inbound email, uploaded document) and **dangerous
   sinks** (HTTP egress, outbound email, database, code execution), then walks
   the `connections` graph to find injectable
   [`source → sink` paths](glossary.md#sourcesink-path). Unrecognised node types
   are reported in `unmapped_node_types`, never silently dropped, and are treated
   as pass-through vertices so they can't hide a real path.
3. **Ground + generate.** The paths and prompt ground an LLM
   (`ATTACK_VECTOR_SET_SYSTEM_PROMPT`) that emits schema-constrained vectors —
   each a `goal`, `technique`, `target_operation`, `injection_payload`, and the
   `source_path` it exploits. No LLM configured ⇒ `503`, like `guardrails:generate`.

## Saved plans (per-agent library)

Each plan is **saved** (Rust-owned, in `redteam_plans`) under a name, so it can be
re-selected and re-run instead of regenerated (which re-pays the LLM). The
generate call persists and returns the saved plan (`id`, `name`, `generated_at`);
`GET /v1/agents/{id}/redteam/plans` lists an agent's plans newest-first, and
`DELETE /v1/redteam/plans/{id}` removes one. The plan body (vectors + paths) is a
JSONB blob; the dashboard selects a saved plan and carries its vectors into the
next dispatch as seeds.

## The workflow graph is the provenance graph

The analyzer's `source → sink` paths feed **both** ends of the loop from one
artifact:

- **Attack generator** — what to inject (at the source) and which operation to
  drive (the sink).
- **Policy synthesizer** — what flow to block. Each unguarded path is a
  preventive policy candidate on its own.

## Two honest policy sources

| Source | Endpoint | Proof | Marked |
|---|---|---|---|
| **Dynamic** | run → [harden](redteam-harden.md) | an attack **landed** (observed sink/guard), then the candidate is **verified** | `harden` candidate (`create` or `tighten`) |
| **Static** | `POST /v1/agents/{id}/redteam/static-policies` | an unguarded `source → sink` path exists (discovery without execution) | policy id `static-…` |

Dynamic is proof; static is coverage. Static policies exist for agents with no
runnable target: one preventive [semantic](glossary.md#matcher) policy per
distinct `source_category → sink_category` class, generalized to the *class* of
exposure (never a literal payload). No injectable path ⇒ an empty set, never a
fabricated policy. Dynamic harden can also reject an attempted candidate with a
reason and route the operator to manual policy authoring. New generated policies
attach to the agent `enabled = false`; tightened dynamic policies keep their
previous enabled state. The operator opts in via `PATCH /v1/policies/{id}/enabled`,
exactly like the other generators.

## Seeds reach the attacker, not generic templates

A planned vector travels `RedteamDispatchRequest.attack_vectors` → the
orchestrator forwards it as `RunnerDispatch.attack_vectors` → the
HackAgentOrchestration runner feeds each vector's goal + seed payload into
HackAgent's case strengthening. So the attack is gray-box — it knows the agent's
real exposure — instead of running a generic pack. The runner contract is owned
by `tl-core` and vendored into HackAgentOrchestration; **HackAgent itself is
never modified** (the vectors are just stronger seeds). See the workspace
`CLAUDE.md` for the cross-repo contract rule.

## Ownership

- Wire types — `crates/tl-core` (`AttackVector`, `WorkflowPath`,
  `RedteamPlanResponse`; `WorkflowDefinition` on `AgentProfile`;
  `RunnerAttackVector` + `attack_vectors` on the dispatch contracts).
- Planner endpoint, workflow analyzer, static-policy synthesis — `crates/tl-server`
  (`redteam::plan`, `redteam::workflow_analyzer`).
- Vector forwarding — `crates/tl-server` (`redteam::orchestrator`).
- Runner consumption — `HackAgentOrchestration` (`engines/hackagent_adapter.py`).
- Loop UX — `apps/web/app/attacks` (the `PlanCard`, reusing the existing harden
  card and report comparison for before/after).

Related: [redteam-dispatch.md](redteam-dispatch.md) (the run),
[redteam-harden.md](redteam-harden.md) (the synthesis the loop reuses).
