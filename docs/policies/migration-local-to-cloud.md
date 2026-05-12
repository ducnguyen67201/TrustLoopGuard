# Local To Cloud Policy Migration

Local mode and cloud mode use the same policy YAML contract.

## Local Mode

Local policies live in files such as:

```text
policies/refund-promise.yaml
```

`tl-server` loads local policies from `TL_POLICY_DIR` at boot.

## Cloud Mode

Cloud policies are saved through the policy API and stored in Postgres.
The DB stores both:

- `source_yaml`: original authoring source
- `parsed_policy`: normalized JSON representation

The runtime parses and validates through `tl-policy` before saving or
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

## CLI Workflow

```bash
cargo run -p tl-cli -- policy validate policies/refund-promise.yaml
cargo run -p tl-cli -- policy push policies/refund-promise.yaml --url http://localhost:8080
cargo run -p tl-cli -- policy pull refund-promise --output policies/refund-promise.yaml --url http://localhost:8080
```

`policy push` sends the raw YAML to `POST /v1/policies`. `policy pull` reads
`source_yaml` from `GET /v1/policies/{id}` and writes it to disk.

For protected servers, either pass `--api-key` or export `TL_API_KEY`.
