# Policy YAML Reference

This page explains every field. For a quick start, read
[Write Your First Policy](README.md) first.

This is the canonical shape for a TrustLoopGuard policy file:

```yaml
id: refund-guarantee
description: Prevents agents from guaranteeing refunds.
severity: high

when:
  domains: [customer_support]
  channels: [chat, email]
  agents: [acme-support-v3]

match:
  any:
    - literal: "guaranteed refund"
    - regex: "(?i)money[- ]back guarantee"

action: transform
rewrite: "I can help check refund eligibility, but I can't guarantee the outcome."
```

## `id`

Required. Stable machine-readable identifier. Users see this in logs,
dashboard rows, and `triggered_policies`.

Rules:

- use lowercase letters, numbers, `-`, or `_`
- keep it stable once logs or dashboards reference it
- unique within the policy set that is active for a tenant

Good:

```yaml
id: refund-guarantee
```

Avoid:

```yaml
id: Refund Guarantee
```

## `description`

Optional but recommended. Use one plain sentence that explains why the rule
exists.

```yaml
description: Prevents agents from guaranteeing refunds.
```

## `severity`

Optional. Defaults to `medium`.

Allowed values:

- `low`
- `medium`
- `high`
- `critical`

Severity is reported in `triggered_policies`. It does not replace `action`.

## `when`

Optional scope filters. Empty or omitted filters mean "all".

```yaml
when:
  domains: [customer_support]
  channels: [chat, email]
  agents: [acme-support-v3]
```

### `when.domains`

Limits the policy to request domains. If the request has no domain, the engine
uses `customer_support`.

### `when.channels`

Limits the policy to `voice`, `chat`, or `email`.

Legacy `when.channel` is still accepted for existing local files, but new docs
and examples use `when.channels`.

### `when.agents`

Limits the policy to specific `agent_id` values.

## `match`

Required. Describes what triggers the policy. Matching is evaluated against the
agent's `proposed_output`.

Single matcher:

```yaml
match:
  literal: "guaranteed refund"
```

Any matcher:

```yaml
match:
  any:
    - literal: "guaranteed refund"
    - regex: "(?i)money[- ]back guarantee"
```

All matchers:

```yaml
match:
  all:
    - regex: "(?i)refund"
    - semantic: "the agent guarantees the outcome"
```

## Matchers

### `literal`

Fast exact substring match. Use this first when you know the phrase.

```yaml
match:
  literal: "guaranteed refund"
```

### `regex`

Rust regex pattern. Use this when users may write small variations.

```yaml
match:
  regex: "(?i)money[- ]back guarantee"
```

Notes:

- use `(?i)` for case-insensitive matching
- Rust regex does not support lookaround
- invalid regex patterns fail validation

### `semantic`

Meaning-based matcher. Use this for concepts that cannot be captured with a
short phrase.

```yaml
match:
  semantic: "the agent gives legal advice"
```

Semantic matching is opt-in and uses the configured `semantic_policy` LLM judge
route. Literal and regex matchers are evaluated deterministically first; semantic
matchers are skipped if the route is not configured. A high-confidence semantic
match applies the policy effect. Ambiguous or unavailable judge results produce
`defer` for high and critical policies, while lower-severity policies fail open.

## `action`

Required. What TrustLoopGuard should do if the policy triggers.

Allowed values:

- `permit`
- `deny`
- `transform`
- `require_approval`
- `defer`

## `rewrite`

Required when `action: transform`.

```yaml
action: transform
rewrite: "I can help review eligibility, but I can't guarantee the outcome."
```

Do not include `rewrite` unless the action is `transform`.
