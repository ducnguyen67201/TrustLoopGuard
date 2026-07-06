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
- `mandates`, `approval_requests`, `financial_receipts`, and `counterparties` are durable support tables for later service/API slices.

Traces remain audit evidence. They are not the source of truth for reserved or executed spend.

## HTTP API

The Rust server exposes the first financial action lifecycle endpoints:

- `POST /v1/financial/actions` creates an idempotent action record from a typed `CreateFinancialActionRequest`.
- `GET /v1/financial/actions/{id}` reads the durable action record.
- `POST /v1/financial/actions/{id}/approve` moves a proposed or held action to `authorized`.
- `POST /v1/financial/actions/{id}/deny` moves a non-terminal action to `denied`.
- `POST /v1/financial/actions/{id}/execute` moves an authorized or held action to `executed`.

These endpoints are the action-state API. They do not yet perform full mandate lookup, policy window evaluation, provider execution, receipt generation, or approval recovery; those remain service-layer responsibilities to build on the same storage and wire contract.

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
