# Copyable Policy Examples

The YAML files in `docs/policies/examples/` are documentation fixtures. Tests
parse them through `tl-policy`, so they should stay valid as the DSL evolves.

If you are new, start with `refund-guarantee.yaml`. It shows the most common
pattern: catch risky wording and replace it with a safer answer.

## Refund Guarantee Rewrite

Use when an agent is allowed to discuss refunds but cannot promise an outcome.

See [refund-guarantee.yaml](examples/refund-guarantee.yaml).

## PII Block

Use when the proposed output contains sensitive information that should never
be sent.

See [pii-block.yaml](examples/pii-block.yaml).

## Legal Advice Escalation

Use when an agent starts giving legal recommendations that require human review.

See [legal-escalation.yaml](examples/legal-escalation.yaml).

## Voice-Only Disclosure

Use when a policy is specific to the voice channel and should not affect chat
or email output.

See [voice-disclosure.yaml](examples/voice-disclosure.yaml).
