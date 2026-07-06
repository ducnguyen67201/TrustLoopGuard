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
- `financial_receipts` stores tenant-scoped proof records for executed actions. The first receipt slice creates a deterministic receipt id matching the action id and records generic execution proof; provider-rich proof is added when provider execution is wired into the financial service.
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

These endpoints route through `FinancialAuthorizationService`, the Rust service layer that owns create/list/read/hold/approve/deny/execute/outcome intent before storage is called. Today the service performs request validation, status orchestration, durable approval request creation for held actions, mandate create/list/revoke operations, generic receipt creation after execution, and append-only outcome recording/listing. It does not yet enforce mandate lookup during action authorization, policy window evaluation, provider execution, provider-rich receipt generation, or policy-driven approval recovery; those responsibilities belong in this same service layer as the subsystem matures.

## Policy Family

`family: financial` policies apply to typed financial actions only. They do not run on generic `/v1/events` guard events and do not replace the legacy `family: payment` event-path caps.

Selectors live under `when`:

- `agents`
- `action_kinds`
- `operations`
- `currencies`
- `rails`

Controls include per-action caps, hold thresholds, approval thresholds, mandate requirements, counterparty allow/deny lists, new-counterparty holds, refund-original-method-only rules, and required eligibility preconditions.

The pure evaluator in `tl-engine` may check only fields present on the `FinancialAction` and policy. Stateful checks such as ledger windows, mandate lookup, approval recovery, eligibility evidence, and provider execution belong in the Rust server financial authorization service. Ledger windows are backed by `tl-storage` financial ledger entries.

## Evidence And Eligibility

Financial policy caps answer "is the amount within the configured limit?" Eligibility answers "is this action legitimate?" For a refund, eligibility may require evidence that the order exists, payment was captured, the refund window is open, the amount is within refundable balance, and the destination is the original payment method.

AI output can draft a candidate action, but it is not trusted evidence. TrustLoopGuard should verify eligibility from trusted customer backends, provider data, stored facts, explicit evidence refs, or later connector integrations.

## Outcome Data

The financial action lifecycle records more than allow/block:

```text
proposed -> held/authorized -> executed/failed -> reversed/recovered/disputed/loss_recorded
```

That outcome history is the data foundation for future agent underwriting, risk scoring, guarantees, or premiums. Those products are not part of the financial authorization contract itself; the contract only preserves clean, structured, tenant-scoped action and outcome facts.

## Reversal Semantics

"Revert" is provider-aware. Some actions can be canceled before capture or settlement. Some pending refunds can be canceled. Many completed refunds cannot be literally undone and require compensating recovery such as recharge, invoice, internal balance adjustment, or manual recovery. `ReversalCapability` and `RecoveryStatus` represent those realities without promising a universal undo button.
