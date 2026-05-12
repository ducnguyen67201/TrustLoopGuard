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
