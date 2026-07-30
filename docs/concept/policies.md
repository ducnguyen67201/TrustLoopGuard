# Policies

TrustLoopGuard has one Rust-owned policy registry. Every policy definition is stored in `crates/tl-storage` in the `policies` table and exposed through Rust `/v1/policies` APIs. The web dashboard is only a UI/proxy layer.

## Registry

A policy has a `family`:

- `content` — legacy protection rules for generic guard events.
- `tool` — deterministic executable-tool controls, including shell command facts and exact-action approval.
- `financial` — spending, approval requirements, counterparty, grant, and refund eligibility controls for typed financial actions.
- `flow`, `parameter_source`, `approval`, `memory` — typed event-engine policy families.
- `source_label` — per-origin label overrides used by event source-label resolution.

Documents without a top-level `family:` tag are `content` policies. Family policies keep typed evaluators; the registry unifies lifecycle, not business logic.

```text
Policy document
      |
      v
tl-policy parses content or family
      |
      v
policies table
      |
      +--> entity_versions
      |
      +--> policy_environment_deployments
              |
              v
      runtime loads enabled policy family for workspace/environment
```

## Environment Enablement

Policy definitions are workspace-level. Runtime enablement is environment-level through `policy_environment_deployments`. A financial policy can be enabled in `production` and disabled in `test`, just like a content policy.

`policies.enabled` is authoring state and migration input. Runtime paths should use deployment state for the resolved environment.

## API

Policy reads and validation retain their existing authenticated access. Policy mutations require
an authenticated workspace Owner or Admin (or a platform administrator acting through the trusted
user/internal-service lane). Workspace runtime keys are rejected before parsing or storage access,
because a governed runtime principal must not change the policies that constrain it.

- `POST /v1/policies` creates or updates content and family policies from YAML or JSON.
- `GET /v1/policies` lists policies visible in the selected environment.
- `GET /v1/policies?family=financial` filters the registry by family.
- `GET /v1/policies/{id}` returns one policy document, including `family` and source YAML.
- `PATCH /v1/policies/{id}/enabled` changes environment enablement for any family.
- `DELETE /v1/policies/{id}` soft-deletes any family.

Domain endpoints can stay as ergonomic wrappers. For example, `POST /v1/financial/policies` accepts typed JSON for a spending control, converts it to `family: financial`, and stores it in the same registry. `POST /v1/label-policies` accepts the legacy source-label body and stores `family: source_label` policies with stable ids such as `source-label-web`.

## Runtime Boundaries

The generic guard pipeline evaluates `family: content` policies for content observations. Executable tool subjects pass enabled `family: tool` policies through the Tool authorization adapter. Shell commands can use deterministic analyzer facts or explicit JSON-parameter matching; [command-safety.md](command-safety.md) is the canonical contract.

The financial adapter loads `family: financial` policies for `FinancialAction` subjects, emits typed findings and authority requirements, and passes them to the common authorization coordinator. Ledger windows, eligibility evidence, and current policy ceilings are still financial-domain inputs; grants, approvals, leases, and authorization receipts are common kernel concepts. Provider execution and the financial execution receipt remain separate downstream concerns.

Source-label resolution reads `family: source_label` policies through the label-policy provider. The `/v1/label-policies` route remains for compatibility, but it is a wrapper over the same registry rather than a separate policy store.
