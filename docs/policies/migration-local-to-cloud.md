# Local To Cloud Policy Migration

Local mode and cloud mode use the same policy YAML contract.

## Local Mode

Local policies live in files such as:

```text
policies/refund-promise.yaml
```

`tl-server` loads local policies from `TL_POLICY_DIR` at boot.

## Cloud Mode

Cloud policies will be saved through the policy API and stored in Postgres.
The DB should store both:

- `source_yaml`: original authoring source
- `parsed_policy`: normalized JSON representation

The runtime should parse and validate through `tl-policy` before saving or
evaluating a policy.

## Hybrid Mode

Hybrid mode should layer policies deterministically:

```text
universal built-ins
local baseline policies
cloud tenant policies
agent-scoped policies
request-selected policies
```

Local files are useful for version-controlled baseline rules. Cloud rules are
useful for dashboard-managed tenant and agent policies.

## Migration Checklist

1. Validate the local YAML file.
2. Upload the YAML to the cloud policy API.
3. Confirm the saved policy has the same `id`, `match`, `action`, and `rewrite`.
4. Run the playground against the policy before enabling it broadly.
5. Keep local YAML in Git if it remains a baseline policy.

