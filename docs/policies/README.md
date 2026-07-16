# Write Your First Policy

A policy is a YAML rule that tells TrustLoopGuard what an agent is not allowed
to say or execute, and what to do when the rule matches. This guide starts with
the default content family; executable shell controls are covered in
[Shell command safety](../concept/command-safety.md).

![Policy lifecycle](../concept/assets/policy-lifecycle.svg)

You can write a useful policy in five minutes:

1. Copy the example below into `policies/refund-guarantee.yaml`.
2. Change `id` and `description` to match your use case.
3. Change the phrases under `match.any`.
4. Choose the `action`.
5. Validate the file.

## Copy This Rule

```yaml
id: refund-guarantee
description: Prevents agents from guaranteeing refunds.
severity: high

when:
  domains: [customer_support]
  channels: [chat, email]

match:
  any:
    - literal: "guaranteed refund"
    - regex: "(?i)money[- ]back guarantee"

action: transform
rewrite: "I can help check refund eligibility, but I can't guarantee the outcome."
```

## Validate It

From the repo root:

```bash
cargo run -p tl-cli -- policy validate policies/refund-guarantee.yaml
```

Expected output:

```text
ok: policy `refund-guarantee` valid
```

`policy-lint` remains as a legacy alias for local validation.

## What Each Part Means

| Field | Meaning |
| --- | --- |
| `id` | Short stable name. This appears in logs and `triggered_policies`. |
| `description` | Plain-English reason this rule exists. |
| `severity` | Risk level shown to operators: `low`, `medium`, `high`, `critical`. |
| `when` | Optional scope. Use it when a rule only applies to some domains, channels, or agents. |
| `match` | The text or meaning that triggers the rule. |
| `action` | The canonical finding effect: `permit`, `deny`, `transform`, `require_approval`, or `defer`. |
| `rewrite` | Safe replacement text. Required for `action: transform`. |

## Choosing An Action

| Action | Use When |
| --- | --- |
| `permit` | You want to record a match without stopping the output. |
| `deny` | The output should not be sent. |
| `transform` | The user can receive the provided safer replacement. |
| `require_approval` | A matching authenticated grant may satisfy the requirement. |
| `defer` | Evidence or system state is unresolved; approval cannot bypass it. |

For most first policies, use `transform` or `deny`.

## Choosing A Matcher

| Matcher | Use When |
| --- | --- |
| `literal` | You know the exact phrase to catch. Start here. |
| `regex` | You need case-insensitive text or small variations. |
| `semantic` | You need meaning-based matching and have a `semantic_policy` LLM route configured. |

Start with `literal`. Add `regex` only when exact text is not enough. Use
`semantic` for concepts that need model judgment; if no semantic judge route is
configured, semantic matchers are skipped while literal and regex matchers still
run.

## Local And Cloud Mode

The same YAML contract is used everywhere:

```text
local file -> tl-policy parser -> Policy
cloud row  -> tl-policy parser -> Policy
```

The runtime engine evaluates parsed `Policy` values, not raw YAML. That keeps
local and cloud behavior aligned as long as both paths use `tl-policy`
validation.

## Push And Pull

After validating locally, publish the same YAML to a running `tl-server`:

```bash
cargo run -p tl-cli -- policy push policies/refund-guarantee.yaml \
  --url http://localhost:8080
```

If the server requires auth, pass `--api-key` or set `TL_API_KEY`.

To pull the saved YAML back from the server:

```bash
cargo run -p tl-cli -- policy pull refund-guarantee \
  --output policies/refund-guarantee.yaml \
  --url http://localhost:8080
```

## Common Fixes

- `id` failed: use lowercase letters, numbers, `-`, or `_`.
- `rewrite` failed: add `rewrite` when `action` is `transform`.
- `regex` failed: simplify the pattern; Rust regex does not support lookaround.
- rule does not fire: check `when.channels`, `when.agents`, `when.domains`, and
  the request's `policies` list.

## Keep Reading

- [YAML reference](yaml-reference.md) for every field.
- [Examples](examples.md) for complete copyable policies.
- [Shell command safety](../concept/command-safety.md) for `family: tool` facts, bounds, and approval.
- [Validation errors](validation-errors.md)
- [Local to cloud migration](migration-local-to-cloud.md)
- [Cookbook](cookbook.md)
