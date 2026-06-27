# Agent profile — field reference

An **agent profile** is the YAML (or JSON) document you register once per agent. Runtime `GuardEvent`s reference it through `principal.agent_id`. The profile tells TrustLoopGuard what the agent **is**, what it **may claim**, and how it **should sound**.

This document is the field-by-field reference. If you just want to copy-paste a working file and ship, see [`demo/agents/acme-support-v3.yaml`](../demo/agents/acme-support-v3.yaml).

For the request flow, see [`INTEGRATION.md`](INTEGRATION.md). For the runtime behaviour, see [`concept/v0-design-decisions.md`](concept/v0-design-decisions.md).

---

## Minimum valid profile

The smallest profile the parser accepts:

```yaml
agent_id: my-bot
display_name: My Bot
scope:
  in_scope:
    - billing
authority: {}
tone:
  target: neutral
```

Three things are required: `agent_id`, `display_name`, and at least one entry in `scope.in_scope`. Everything else defaults to empty.

## Full schema

```yaml
agent_id: string                 # required, non-empty
display_name: string             # required, non-empty
scope:
  in_scope: [string, ...]        # required, ≥1 entry
  out_of_scope: [string, ...]    # optional
authority:
  can_promise: [string, ...]     # optional
  cannot_promise: [string, ...]  # optional
tone:
  target: string                 # required by Tier 3, free-form
  forbidden: [string, ...]       # optional
knowledge_sources:
  - kb_id: string                # required per source; stable source id
    kind: local | web            # optional, defaults to local
    url: string                  # required when kind: web
    description: string          # optional, shown to Tier 3 judges
escalation_triggers: [string, ...]  # optional; parsed but not yet enforced
workflow_requirements:
  - name: string                 # workflow label used by harden synthesis
    required_before: [string, ...]  # checks required before sensitive steps
    sensitive_steps: [string, ...]  # workflow steps harden should protect
```

The Rust source of truth is `crates/tl-core/src/agent.rs`. The validation rules live in `crates/tl-policy/src/agent_parse.rs`.

---

## Field-by-field

### `agent_id`

| | |
|---|---|
| Required | yes — non-empty |
| Used by | server routing |
| Effect | Runtime identity for policies, traces, and agent-owned generated guardrails. |
| Best practice | Include a version suffix (`acme-support-v3`) so you can ship a new profile alongside the old one and migrate traffic. |

### `display_name`

| | |
|---|---|
| Required | yes — non-empty |
| Used by | logs, traces, future dashboards |
| Effect | Human label. Never sent to the LLM judges. |

### `scope.in_scope`

| | |
|---|---|
| Required | ≥1 entry |
| Used by | future Tier 2 out-of-scope embeddings (v0 boots indexes but doesn't yet enforce a verdict from them) |
| Effect | Each entry is embedded once at boot and indexed by HNSW. At check time the draft is embedded and compared. |
| Best practice | Topic phrases, not full sentences. `"billing questions"` beats `"the user is asking about billing"`. 3–10 entries is a healthy size. |

### `scope.out_of_scope`

| | |
|---|---|
| Required | no |
| Used by | same as `in_scope`, the negative side |
| Effect | A draft semantically close to one of these is a stronger signal that it's off-topic. |
| Best practice | Use this for topics the agent should **redirect**, not for things it should refuse outright. Refusals belong in `authority.cannot_promise`. |

### `authority.can_promise`

| | |
|---|---|
| Required | no |
| Used by | Tier 3 **authority** judge |
| Effect | Substituted verbatim into `crates/tl-llm/src/prompts/authority.md` as `{{CAN_PROMISE}}`. The judge marks the draft `within_authority: false` if it makes any commitment **not** on this list. |
| Best practice | Specific verbs the agent owns: `"we'll respond within 24 hours"`, `"I'll send a transcript by email"`. Phrase as something the agent *says*, not a category. |

### `authority.cannot_promise`

| | |
|---|---|
| Required | no |
| Used by | Tier 3 **authority** judge |
| Effect | Substituted into `{{CANNOT_PROMISE}}`. A draft making any of these promises is blocked even if it's worded politely. |
| Best practice | Bright lines: `"refunds (any amount)"`, `"delivery dates beyond what shipping API confirms"`, `"feature timelines"`. Vague entries (`"things we can't deliver"`) are too soft for the judge to act on consistently. |

### `tone.target`

| | |
|---|---|
| Required | no (but Tier 3 tone judge produces a meaningful verdict only when set) |
| Used by | Tier 3 **tone** judge |
| Effect | Substituted into `{{TONE_TARGET}}`. The judge decides whether the draft "matches the target tone." |
| Best practice | A short phrase the LLM will understand: `"warm-professional"`, `"warm, concise, factual"`, `"clinical-neutral"`. Don't write a paragraph — the judge needs a *label*, not an essay. |

### `tone.forbidden`

| | |
|---|---|
| Required | no |
| Used by | Tier 3 **tone** judge |
| Effect | Substituted into `{{TONE_FORBIDDEN}}`. The judge marks the draft non-compliant if its detected tone matches anything here. |
| Best practice | **Adjectives, not vocabulary.** This is for tones (`"dismissive"`, `"sarcastic"`, `"defensive"`), not banned words. To ban specific words (`"guaranteed"`, `"definitely"`), write a Tier 1 policy file in `policies/*.yaml` with a `Literal` matcher — it's faster and more reliable than asking an LLM. |

### `knowledge_sources`

| | |
|---|---|
| Required | no |
| Used by | Tier 3 **hallucination** judge |
| Effect | Each source is substituted into `{{PROFILE}}` in `prompts/hallucination.md`. `kind: local` is the default. `kind: web` requires a public `http(s)` URL and rejects localhost/private loopback hosts. |
| Best practice | Treat `knowledge_sources` as the approved source catalog. The actual grounding excerpts still come per-request via `context.docs`; fetch or retrieve docs in your app, then pass the snippets on the `GuardEvent` submitted to `/v1/events`. |

Example:

```yaml
knowledge_sources:
  - kb_id: internal-kb
    kind: local
    description: Curated internal support knowledge base
  - kb_id: public-docs
    kind: web
    url: https://docs.acme.com/support
    description: Public support docs
```

### `escalation_triggers`

| | |
|---|---|
| Required | no |
| Used by | nothing yet — v0 parses and stores it but no tier consumes it |
| Effect | Stored on the profile. Reserved for a future policy-driven escalation rule (Phase 2 work). |
| Best practice | Document the intent now (`"threats of self-harm"`, `"mentions of legal action"`) — the day the feature ships you'll already have the policy. |

### `workflow_requirements`

| | |
|---|---|
| Required | no |
| Used by | red-team harden policy synthesis |
| Effect | When a red-team attack lands, harden uses these requirements to synthesize workflow-specific semantic policies instead of relying only on built-in harm heuristics. |
| Best practice | Name the workflow and list concrete checks/steps: `"Refund processing"` with `"identity verification"` and `"requesting payout destination"` is better than `"be careful with refunds"`. |

Example:

```yaml
workflow_requirements:
  - name: Refund processing
    required_before:
      - identity verification
      - transaction verification
    sensitive_steps:
      - promising a refund
      - issuing a refund
      - requesting payout destination
```

---

## Two common authoring mistakes

### 1. Putting banned vocabulary in `tone.forbidden`

```yaml
tone:
  forbidden:
    - "guaranteed"        # ❌ this is a word, not a tone
    - "definitely"        # ❌
    - "promise"           # ❌
```

The Tier 3 tone judge treats this as "the model shouldn't sound *guaranteed*." That's incoherent. The model will sometimes flag, sometimes not.

**Fix:** put banned vocabulary in a tenant policy file under `policies/*.yaml` with a `Literal` matcher. That fires in Tier 1, in microseconds, deterministically:

```yaml
# policies/no-guarantees.yaml
id: brand.no-guarantees
match:
  any:
    - literal: "guaranteed"
    - literal: "definitely will"
    - literal: "I promise"
action: rewrite
rewrite: "Strip the guarantee, hedge the promise."
severity: medium
```

Use `tone.forbidden` for *moods* (`dismissive`, `sarcastic`). Use Tier 1 policies for *words*.

### 2. Listing categories instead of commitments in `authority.cannot_promise`

```yaml
authority:
  cannot_promise:
    - "things outside our policy"   # ❌ too abstract
    - "anything the user might ask" # ❌ judge can't act on this
```

The judge needs a concrete commitment to compare against. Rewrite each entry as a verb the agent might say:

```yaml
authority:
  cannot_promise:
    - "refunds of any kind"
    - "lifetime discounts"
    - "delivery dates not confirmed by the shipping API"
    - "feature release timelines"
```

Rule of thumb: if you can't imagine the agent typing it verbatim, the judge can't catch it.

---

## Validation errors you'll see

The parser is strict on three things. The HTTP response on bad YAML is `400 Bad Request` with a JSON body like `{"error":"validation: <message>"}`.

| Error | Cause | Fix |
|---|---|---|
| `agent_id is required` | empty/missing/whitespace | provide a non-empty id |
| `display_name is required` | empty/missing/whitespace | provide a non-empty name |
| `scope.in_scope must contain at least one entry` | empty list or missing field | add ≥1 topic |
| YAML parse error (any line) | malformed YAML | check indentation; YAML lists need `- ` prefix |

Profiles are versioned **by `agent_id`**. To roll out a change safely, register the new version under a new id (`acme-support-v4`), shift traffic in your client code, then `DELETE /v1/agents/acme-support-v3` once you're confident.

---

## Registering, fetching, deleting

```bash
# Register (creates or replaces by agent_id)
curl -X POST http://localhost:8080/v1/agents \
  -H "Authorization: Bearer $TL_API_KEY" \
  -H "Content-Type: application/yaml" \
  --data-binary @policies/agents/acme-support-v3.yaml

# Fetch
curl http://localhost:8080/v1/agents/acme-support-v3 \
  -H "Authorization: Bearer $TL_API_KEY"

# Delete (soft delete — future runtime references should stop using this profile)
curl -X DELETE http://localhost:8080/v1/agents/acme-support-v3 \
  -H "Authorization: Bearer $TL_API_KEY"
```

You can also POST JSON instead of YAML if your client doesn't have a YAML serializer handy — same schema, set `Content-Type: application/json`.

---

## Round-trip checklist

Before pointing real traffic at a new profile:

- [ ] `cargo test -p tl-policy` passes locally with the new file added as a fixture (optional but cheap)
- [ ] `POST /v1/agents` returns 200 against a local `tl-server`
- [ ] `GET /v1/agents/<id>` returns what you registered
- [ ] One `POST /v1/events` returns a `Decision` for a `GuardEvent` using `principal.agent_id=<id>`
- [ ] If Tier 3 is configured: a draft engineered to break each judge (hallucination / tone / authority) returns the expected `Block`
- [ ] Soft-delete works (`DELETE /v1/agents/<id>` removes the profile from authoring APIs)
