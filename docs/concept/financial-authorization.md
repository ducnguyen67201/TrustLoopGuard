# Financial Authorization

Financial authorization is the TrustLoopGuard surface for agent actions that move money, issue credits, approve invoices, or otherwise create financial obligation. It is separate from the generic [GuardEvent](glossary.md#guardevent) path: a guard event observes proposed agent behavior, while a financial action is a typed domain command with money, counterparty, mandate, policy, outcome, and proof semantics.

```text
Generic runtime safety
  GuardEvent -> guard policy/checkers -> Decision -> trace

Financial authorization
  FinancialAction -> financial policy -> financial service -> ledger/outcome/receipt
```

## Contract

`FinancialAction` lives in `tl-core` so Rust, OpenAPI, SDKs, server code, storage, and dashboard code share one wire shape. The action carries:

- `kind`, such as `refund`, `payment`, `payout`, `invoice_approval`, or `treasury_transfer`.
- `amount` as integer minor units plus currency.
- `principal_id`, the agent or actor requesting the action.
- Optional counterparty, mandate, memo, and JSON metadata.
- Evidence refs supplied by a trusted caller or later resolved by service code.

Financial outcomes are also typed. `FinancialActionOutcome` records provider status, reversal capability, recovery status, dispute/loss metadata, and final loss amount when known. Outcomes are not spend accounting. Ledger entries answer spend and reservation questions; outcomes answer operational and risk-result questions.

## Durable Storage

`tl-storage` owns the durable financial authorization tables:

- `financial_actions` stores the tenant-scoped requested action, idempotency key, current status, amount, principal, counterparty, mandate, rail, metadata, and evidence snapshot.
- `financial_action_events` is the append-only action event stream for creation and status transitions.
- `financial_ledger_entries` is the accounting source for spend windows. Reserved and executed entries add to net spend; released and reversed entries subtract from it. Spend caps must use this ledger, not generic traces.
- `approval_requests` stores pending/decided authorization recovery work for held actions.
- `mandates` stores tenant-scoped authorization scopes that can be created, listed, and revoked through the financial service.
- `financial_receipts` stores tenant-scoped proof records for executed actions. The receipt id matches the action id and records action, evidence, mandate, approval-request, policy, ledger, and `payment_http` provider proof snapshots.
- `financial_action_outcomes` stores append-only provider result, reversal, recovery, dispute, and loss observations for an action. These rows are ordered newest first for action history and do not affect spend windows.
- `counterparties` is a durable support table for a later service/API slice.

Traces remain audit evidence. They are not the source of truth for reserved or executed spend.

## HTTP API

The Rust server exposes the first financial action lifecycle endpoints:

- `POST /v1/financial/actions` creates an idempotent action record from a typed `CreateFinancialActionRequest`.
- `GET /v1/financial/actions` lists tenant-scoped action records newest first.
- `GET /v1/financial/actions/{id}` reads the durable action record.
- `GET /v1/financial/approval-requests` lists tenant-scoped financial approval requests newest first.
- `POST /v1/financial/mandates` creates a tenant-scoped financial mandate.
- `GET /v1/financial/mandates` lists tenant-scoped financial mandates newest first.
- `POST /v1/financial/mandates/{id}/revoke` revokes a tenant-scoped mandate.
- `GET /v1/financial/receipts/{id}` reads a tenant-scoped financial receipt/proof record.
- `POST /v1/financial/actions/{id}/outcomes` appends a tenant-scoped operational outcome for the action.
- `GET /v1/financial/actions/{id}/outcomes` lists the action's operational outcomes newest first.
- `POST /v1/financial/actions/{id}/approve` moves a proposed or held action to `authorized`.
- `POST /v1/financial/actions/{id}/deny` moves a non-terminal action to `denied`.
- `POST /v1/financial/actions/{id}/execute` moves an authorized or held action to `executed` and creates the first receipt/proof record.

These endpoints route through `FinancialAuthorizationService`, the Rust service layer that owns create/list/read/hold/approve/deny/execute/outcome intent before storage is called. Today the service performs request validation, mandate lookup and validity checks for referenced mandates, action-local financial policy evaluation, eligibility precondition evaluation from trusted evidence refs, ledger-derived daily/monthly window evaluation, status orchestration, durable approval request creation for held actions, policy-driven approver role assignment, approver actor capture from `x-tlg-user-id` when approval or denial is routed through HTTP, ledger reservation/release/execution entries for lifecycle transitions, mandate create/list/revoke operations, `payment_http` provider execution for payment-rail actions, structured receipt creation after execution, and append-only outcome recording/listing.

If `CreateFinancialActionRequest.execute` is true and checks leave the action clean, the service authorizes and executes the action immediately. Held actions write a reserved ledger entry, denied held actions write a release entry, and executed actions write execution ledger evidence into the receipt. Execution receipts use `schema: "financial_execution_receipt.v1"` and snapshot the executed action, evidence refs, mandate ref and mandate record when present, matching policy families, approval requests for the action including `decided_by` when known, ledger event ids, and provider proof. For `rail: payment_http`, execution is structural: the service resolves the workspace's vaulted `payment_http` gateway provider connection, unseals the credential server-side, forwards the request with an idempotency key, records provider status/reference/response in the receipt, and appends a provider outcome. If no provider is configured or the forward fails, the action becomes `failed` instead of being presented as executed. These ledger entries are the accounting state used by financial spend windows.

`/mcp/pay` remains as a compatibility transport for existing agent demos. `PayGate` now creates typed `FinancialAction(kind=payment, rail=payment_http)` records, lets `FinancialAuthorizationService` enforce policy/ledger/hold behavior, and maps the resulting financial status back to the old MCP JSON statuses (`executed`, `hold`, `block`, `allow_no_provider`, `allow_failed_execute`). Legacy `family: payment` policies are still honored by the financial service for typed payment actions, so existing payment caps do not need to be migrated before `/mcp/pay` benefits from ledger accounting. Held PayGate actions only resolve their approval after provider execution succeeds; a failed provider call records a failed outcome while leaving the hold retryable.

The web dashboard reads this state through Rust APIs and same-origin proxy routes. `/financial` shows the financial action ledger, latest outcomes, spending controls, and payment-provider setup state. `/financial/approvals` lists pending financial approval requests; approving a row calls the financial approve endpoint and then the execute endpoint so held actions resume execution. `/financial/mandates` lists, creates, and revokes mandates. `/financial/receipts/{id}` shows the action, receipt proof, provider response, latest outcome/recovery state, and ledger event ids. The dashboard does not own financial authorization state.

When an action includes a `MandateRef`, the service resolves the mandate in the same workspace before applying policy. The referenced mandate must be active, match the action principal, be inside its start/expiry window, and cover the action according to any structured scope fields present today: `action_kinds`, `currency` or `currencies`, and `max_amount_minor`. A failed mandate proof transitions the action to `denied` so the attempt remains auditable. A missing mandate on an action is still governed by `family: financial` policy through `mandate_required`; the runtime does not silently convert generic guard events into financial actions.

`demo/financial-refund` is the offline wedge demo for this surface. It uses SDK-shaped financial helpers to create a refund mandate, submit typed refund actions, prove normal execution, hold-approval-resume, denial without provider execution, duplicate idempotency, receipt export, and outcome recording.

## Policy Family

`family: financial` policies apply to typed financial actions only. They do not run on generic `/v1/events` guard events. The financial service also understands legacy `family: payment` policies as a compatibility input for typed `kind=payment` actions whose metadata includes `operation: "pay"`; this keeps `/mcp/pay` caps ledger-backed without turning generic guard events into financial actions.

Selectors live under `when`:

- `agents`
- `action_kinds`
- `operations`
- `currencies`
- `rails`

Controls include per-action caps, hold thresholds, approval thresholds, approver roles for policy-created holds, mandate requirements, counterparty allow/deny lists, new-counterparty holds, refund-original-method-only rules, and required eligibility preconditions.

The pure evaluator in `tl-engine` checks fields present on the `FinancialAction` and policy, plus a pure helper for caller-supplied window totals. Stateful checks such as ledger windows, mandate validity, approval request creation, approver actor capture, eligibility evidence, and provider execution belong in the Rust server financial authorization service. Ledger windows are backed by `tl-storage` financial ledger entries, not generic traces.

## Evidence And Eligibility

Financial policy caps answer "is the amount within the configured limit?" Eligibility answers "is this action legitimate?" For a refund, eligibility may require evidence that the order exists, payment was captured, the refund window is open, the amount is within refundable balance, and the destination is the original payment method.

AI output can draft a candidate action, but it is not trusted evidence. TrustLoopGuard should verify eligibility from trusted customer backends, provider data, stored facts, explicit evidence refs, or later connector integrations.

`family: financial` policies can list `required_preconditions`. The financial service checks those preconditions against boolean fields in the action's trusted `EvidenceRef.metadata`, such as `payment_captured: true`. Missing evidence follows `missing_evidence_action`; failed evidence follows `failed_precondition_action`. The original evidence refs remain persisted on the action and are included in execution receipt proof.

## Outcome Data

The financial action lifecycle records more than allow/block:

```text
proposed -> held/authorized -> executed/failed -> reversed/recovered/disputed/loss_recorded
```

That outcome history is the data foundation for future agent underwriting, risk scoring, guarantees, or premiums. Those products are not part of the financial authorization contract itself; the contract only preserves clean, structured, tenant-scoped action and outcome facts.

## Reversal Semantics

"Revert" is provider-aware. Some actions can be canceled before capture or settlement. Some pending refunds can be canceled. Many completed refunds cannot be literally undone and require compensating recovery such as recharge, invoice, internal balance adjustment, or manual recovery. `ReversalCapability` and `RecoveryStatus` represent those realities without promising a universal undo button.
