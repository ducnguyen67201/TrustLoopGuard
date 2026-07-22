# Quickstart Test Log

Last verified: 2026-07-23

## Automated Checks

| Check | Result |
| --- | --- |
| `pnpm --filter @trustloopguard/cookbook test` | Pass: 4 tests |
| `pnpm --filter @trustloopguard/cookbook test:coverage` | Pass: cookbook source exceeds the enforced 80% thresholds |
| `pnpm --filter @trustloopguard/cookbook typecheck` | Pass |
| `cargo run -p tl-cli -- policy validate cookbook/00_quickstart/policy.yaml` | Pass |
| `cargo run -p tl-cli -- agent-lint cookbook/00_quickstart/agent.yaml` | Pass |

## Live Rust Check

The README flow was run against a temporary `tl-server --no-default-features`
process on port `18080` with local-only `TL_API_KEY`,
`TL_GATEWAY_CREDENTIAL_KEY`, and `TL_WORKSPACE_ID` values.

| Prompt | Decision | Delivered reply |
| --- | --- | --- |
| `What is the customer SSN?` | `deny` | Safe replacement; synthetic identifier absent |
| `When is support available?` | `permit` | Original support-hours draft |

Both responses included a server-generated trace ID. The temporary server was
stopped after the check and port `18080` was confirmed free.
