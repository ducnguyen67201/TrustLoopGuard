# Financial authorization and execution

Financial actions are typed domain commands for payments, refunds, payouts, purchases, invoice approvals, treasury transfers, and x402 payments. They use the shared [authorization kernel](authorization-kernel.md) for authority and retain financial-only execution and ledger invariants.

## Ownership

- `crates/tl-core`: financial wire types and `FinancialExecutionStatus`.
- `crates/tl-engine`: pure `family: financial` matching and effect generation.
- `crates/tl-server`: the financial adapter, live eligibility/budget checks, provider execution, and HTTP handlers.
- `crates/tl-storage`: actions, ledger entries, execution receipts, outcomes, reservations, and the common authorization repositories.
- `apps/web`: thin proxies and ledger/history presentation. It has no approval mutation routes.

## Two independent state axes

Every `FinancialActionRecord` exposes:

- `authorization_effect`: `permit`, `deny`, `require_approval`, or `defer`; financial actions cannot use `transform`.
- `authorization_status`: the shared durable intent lifecycle.
- `execution_status`: `not_started`, `executing`, `succeeded`, `failed`, `canceled`, or `reversed`.

An approved action is not an executed action. Authorization answers whether execution may start now. Execution state records what the provider and ledger actually did.

## Request flow

1. The caller submits `POST /v1/financial/actions` with a typed action, evidence, idempotency key, and optional `AuthorizationClaim`.
2. The financial adapter derives a `financial:<operation>` capability and a typed `FinancialGrantScope`.
3. Current financial policies and trusted evidence emit hard findings plus stable authority requirements.
4. The common coordinator matches only the explicitly claimed grant. A grant must cover the principal, capability, requirement IDs, action kind, operation, rail, currency, amount, counterparty, and any x402 or precondition bounds.
5. If authority remains, the action appears in the one `/approvals` queue. Reviewer sign-off creates a common exact-once or scoped grant.
6. The caller retries with the grant and a stable attempt ID. Current policy, evidence, eligibility, and budget state are evaluated again.
7. Immediately before provider execution, the financial service atomically reserves live budget and claims a common execution lease.
8. Provider execution writes financial ledger entries and a `FinancialReceipt` linked to the common `AuthorizationReceipt`.
9. The lease is consumed on success or canceled on failure. Outcomes and reversals remain financial-domain records.

`POST /v1/financial/actions/{id}/execute` accepts an optional authorization claim and attempt ID. This lets an action be reviewed first and executed later without weakening the recheck boundary.

## Financial policy controls

Financial policies share the unified policy registry and use `family: financial`. Important controls include:

- `per_transaction_minor`, daily, weekly, and monthly hard caps;
- `approval_threshold_minor`;
- `grant_required`;
- `require_approval_for_new_counterparty`;
- counterparty allow/deny lists;
- trusted eligibility preconditions;
- `missing_evidence_effect`, which normally defaults to `defer`;
- `failed_precondition_effect` and `on_breach`, which normally use `deny`.

Policies create requirements; they do not create or discover grants. A saved grant can remove repeated human review only when it satisfies the current requirement. It never overrides a hard cap, failed eligibility check, missing evidence, revoked authority, or a live budget denial.

## x402

x402 authorization uses the same financial adapter and grant model, with scope fields for host, resource, network, asset, payee, amount, and required preconditions. The agentic-payment endpoints retain their reservation, commit, and rollback lifecycle because signing and settlement are execution concerns. A common authorization receipt explains why signing was permitted; the financial receipt proves what settled.

## UI

- `/approvals` is the only actionable decision queue for both financial and non-financial work.
- `/grants` creates, lists, and revokes saved authority.
- `/financial` is a ledger and execution-history surface. It displays authorization and execution separately and has no approve/deny controls.
