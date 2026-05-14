# Policy Cookbook

Use these recipes as starting points. Copy the closest complete example from
`docs/policies/examples/`, then change the scope and matcher text.

## Prevent Refund Promises

Use `action: rewrite` when the user can still receive a safe response.

```yaml
match:
  any:
    - literal: "guaranteed refund"
    - regex: "(?i)money[- ]back guarantee"
action: rewrite
rewrite: "I can help check refund eligibility, but I can't guarantee the outcome."
```

## Block PII Leakage

Use `action: block` when the output should not be sent at all.

```yaml
match:
  regex: "(?i)(ssn|social security number)"
action: block
```

## Escalate Legal Advice

Use `action: escalate` when a human should review the conversation.

```yaml
match:
  any:
    - regex: "(?i)you should sue"
    - semantic: "the agent gives legal advice"
action: escalate
```

## Apply A Rule To One Agent

```yaml
when:
  agents: [acme-support-v3]
```

## Apply A Rule To Voice Only

```yaml
when:
  channels: [voice]
```

## Test Before Enabling Broadly

For local files, run the policy linter:

```bash
cargo run -p tl-cli -- policy-lint docs/policies/examples/refund-guarantee.yaml
```

For API-based validation, call:

```text
POST /v1/policies/validate
```

## Auto-Generate Guardrails From An Agent Prompt

When an agent has a `system_prompt` on file, the server can ask an LLM to
derive a tailored guardrail policy set: PII leakage, scope discipline, tone
discipline, hallucinated guarantees, and any role-specific risks the prompt
implies. Every generated policy lands with `enabled=false` and is scoped
to the originating agent (`when.agents=[id]`, `owner_agent_id=id`), so
`/v1/check` ignores them until you opt in.

The agent profile must include `system_prompt` (added by [PR A](../../crates/tl-core/src/agent.rs)):

```yaml
agent_id: baker-9000
display_name: Baker 9000
system_prompt: |
  You are a baking assistant for Sweet Loaf. Help customers with bread,
  pastries, and cake recipes. Stay strictly in scope.
scope:
  in_scope: [baking recipes, ingredient substitutions]
  out_of_scope: [medical advice]
authority: {}
tone: {target: warm-helpful}
```

### CLI

```bash
# Register the agent first.
tl agent-lint policies/agents/baker-9000.yaml
# (or push it via curl / your dashboard — there's no `tl agents push` yet.)

# Generate the guardrail set. Persists 3–8 disabled policies on the server.
tl agents guardrails generate baker-9000

# Review what was generated.
tl agents guardrails list baker-9000
```

The generator returns the IDs and short descriptions so you can pick which
ones to enable. Use the policies API to flip the gate:

```bash
curl -X PATCH \
  -H "Authorization: Bearer $TL_API_KEY" \
  -H 'content-type: application/json' \
  -d '{"enabled": true}' \
  $TL_SERVER_URL/v1/policies/no-pii-leak/enabled
```

### HTTP

```bash
curl -X POST \
  -H "Authorization: Bearer $TL_API_KEY" \
  $TL_SERVER_URL/v1/agents/baker-9000/guardrails/generate
```

```bash
curl -H "Authorization: Bearer $TL_API_KEY" \
  $TL_SERVER_URL/v1/agents/baker-9000/guardrails
```

### When this returns errors

| Status | Cause | Fix |
|---|---|---|
| `404` | Agent not registered | `POST /v1/agents` with the YAML above |
| `422` | Agent has no `system_prompt` | Add it to the profile YAML, re-upsert |
| `503` | Deployment has no LLM key | Set `OPENAI_API_KEY` in Doppler / env |
| `502` | LLM returned an invalid shape | Retry; if persistent, file a bug |

### Deletion

Deleting the agent cascades: every owned policy is soft-deleted in the same
operation. The runtime treats those policies as gone immediately.

```bash
curl -X DELETE \
  -H "Authorization: Bearer $TL_API_KEY" \
  $TL_SERVER_URL/v1/agents/baker-9000
```
