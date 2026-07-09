# Financial Authorization

Financial authorization is the TrustLoopGuard surface for agent actions that move money, issue credits, approve invoices, or otherwise create financial obligation. It is separate from the generic [GuardEvent](glossary.md#guardevent) path: a guard event observes proposed agent behavior, while a financial action is a typed domain command with money, counterparty, mandate, policy, outcome, and proof semantics.

```text
Generic runtime safety
  GuardEvent -> guard policy/checkers -> Decision -> trace

Financial authorization
  FinancialAction -> financial policy -> financial service -> ledger/outcome/receipt
```

## Policies, Mandates, And Runtime Payments

Agentic payments use three separate authorization primitives:

- A financial policy is a standing rule for an agent or class of actions. It answers "what is this principal generally allowed to do?" Examples: require a mandate for x402 payments, cap a payment at $5, hold over $50, block unknown counterparties, or limit daily spend.
- A mandate is task-specific authority. It answers "what did the user or customer app authorize for this purchase?" Examples: `spid:commerce-agent` may pay up to $5 in USD for `/premium/article/agentic-commerce` on `base-sepolia` to a specific `pay_to` address.
- A runtime payment is the merchant or provider request the agent is about to execute. For x402, this is the HTTP 402 payment requirement returned by the merchant.

TrustLoopGuard sits between the runtime payment request and wallet signing. The service normalizes the payment requirement, checks it against the active mandate boundary, then applies standing financial policy and budget reservation before returning `signable: true`.

Mandates can be stored by TrustLoopGuard or supplied by a customer's external mandate system. The current production path is TrustLoopGuard-managed mandates: `POST /v1/financial/mandates` accepts either legacy raw `scope` JSON or a typed `payment_scope` that the service normalizes into the durable scope. External signed mandates are represented in the product model, but cryptographic verifier configuration is a separate hardening slice; do not treat bearer API authentication as proof that an external mandate signature was verified.

## Contract

`FinancialAction` lives in `tl-core` so Rust, OpenAPI, SDKs, server code, storage, and dashboard code share one wire shape. The action carries:

- `kind`, such as `refund`, `payment`, `payout`, `invoice_approval`, or `treasury_transfer`.
- `operation`, the customer-defined business operation policy selectors match, such as `issue_refund` or `pay_invoice`.
- `amount` as integer minor units plus currency.
- `principal_id`, the agent or actor requesting the action.
- Optional counterparty, mandate, memo, and JSON metadata. Metadata can carry domain refs, but operation identity must not be hidden there.
- Evidence refs supplied by a trusted caller or later resolved by service code.

Financial outcomes are also typed. `FinancialActionOutcome` records provider status, reversal capability, recovery status, dispute/loss metadata, and final loss amount when known. Outcomes are not spend accounting. Ledger entries answer spend and reservation questions; outcomes answer operational and risk-result questions.

Decision receipts and execution receipts answer different audit questions. A `FinancialActionDecisionReceipt` is a per-action proof that can be read before execution: it explains the normalized decision, authorization scope check, trusted evidence checks, risk codes, approval requirement, policy references, and whether execution proof exists yet. A `FinancialReceipt` is execution proof created after execution: it snapshots provider response, ledger event ids, approval history, policies, evidence, and authorization scope at the moment money moved.

## Durable Storage

`tl-storage` owns the durable financial authorization tables:

- `financial_actions` stores the tenant-scoped requested action, idempotency key, current status, kind, operation, amount, principal, counterparty, mandate, rail, metadata, and evidence snapshot.
- `financial_action_evaluations` stores the immutable environment, effective financial mode, `allow`/`hold`/`block` or `would_*` result, risks, policy ids, reason, and money snapshot used for that action.
- `financial_execution_grants` stores the one-time action hash, execution binding, claim state, expiry, and commit identity for an authorized generic action.
- `financial_execution_connectors` stores externally executed rail scopes and encrypted HMAC credentials. Plaintext secrets are returned only when a connector is created.
- `financial_observation_reviews` is append-only human feedback for adverse observe-mode results; reporting uses the latest review while retaining history.
- `financial_action_events` is the append-only action event stream for creation and status transitions.
- `financial_ledger_entries` is the accounting source for spend windows. Reserved and executed entries add to net spend; released and reversed entries subtract from it. Spend caps must use this ledger, not generic traces.
- `financial_payment_sessions` stores x402 agentic payment budget sessions: principal, currency, maximum spend, reserved amount, committed amount, released amount, expiry, and metadata.
- `financial_payment_reservations` stores action-bound x402 reservations keyed by action, session, and normalized payment requirement hash. It is the concurrency boundary for pre-signing budget reservation, commit, and rollback.
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
- `GET /v1/financial/actions/{id}/decision-receipt` reads the per-action decision receipt for proposed, held, denied, authorized, failed, or executed actions.
- `POST /v1/financial/agentic-payments/authorize` creates the canonical `FinancialAction` for an x402 payment, normalizes the payment requirement, applies mandate and financial policy checks, and reserves session budget before the agent signs.
- `GET /v1/financial/agentic-payments/{id}` reads the action-bound x402 payment record, including normalized requirement and reservation state.
- `POST /v1/financial/agentic-payments/{id}/commit` verifies the settlement proof against the authorized requirement, commits the reservation, writes execution ledger entries, creates a receipt, and records a provider outcome.
- `POST /v1/financial/agentic-payments/{id}/rollback` releases an unsettled reservation and records a failed/rolled-back outcome. It does not reverse a settled payment.
- `GET /v1/financial/agentic-payments/{id}/receipt` reads the execution receipt for a committed x402 payment.
- `GET /v1/financial/approval-requests` lists tenant-scoped financial approval requests newest first.
- `POST /v1/financial/mandates` creates a tenant-scoped financial mandate.
- `GET /v1/financial/mandates` lists tenant-scoped financial mandates newest first.
- `POST /v1/financial/mandates/{id}/revoke` revokes a tenant-scoped mandate.
- `GET /v1/financial/receipts/{id}` reads a tenant-scoped financial receipt/proof record.
- `POST /v1/financial/actions/{id}/outcomes` appends a tenant-scoped operational outcome for the action.
- `GET /v1/financial/actions/{id}/outcomes` lists the action's operational outcomes newest first.
- `POST /v1/financial/actions/{id}/approve` moves a proposed or held action to `authorized`.
- `POST /v1/financial/actions/{id}/deny` moves a non-terminal action to `denied`.
- `POST /v1/financial/actions/{id}/execute` claims a managed execution grant and executes only a persisted `payment_http` payload. Other generic rails return `409` and cannot be marked executed through this endpoint.
- `POST /v1/financial/actions/{id}/commit` verifies an external connector attestation before atomically committing the grant, action, ledger, outcome, and receipt.
- `POST|GET /v1/financial/execution-connectors` and `POST /v1/financial/execution-connectors/{id}/revoke` manage admin-only external executor credentials and scopes.
- `GET /v1/financial/observations/summary` reports currency-separated counterfactual exposure, approval burden, reasons, and reviewed false-positive rates.
- `GET|POST /v1/financial/actions/{id}/observation-reviews` reads or appends human review feedback for adverse observations.
- `GET /v1/financial/policies` lists enabled financial spending controls for the workspace and selected environment.
- `POST /v1/financial/policies` creates or updates a `family: financial` spending control from a typed JSON request. It is an ergonomic wrapper over the unified policy registry.

These endpoints route through `FinancialAuthorizationService`, the Rust service layer that owns create/list/read/hold/approve/deny/execute/outcome/policy intent before storage is called. Today the service performs request validation, financial spending-control authoring, mandate lookup and validity checks for referenced mandates, action-local financial policy evaluation, eligibility precondition evaluation from trusted evidence refs, ledger-derived daily/monthly window evaluation, status orchestration, durable approval request creation for held actions, policy-driven approver role assignment, approver actor capture from `x-tlg-user-id` when approval or denial is routed through HTTP, ledger reservation/release/execution entries for lifecycle transitions, mandate create/list/revoke operations, `payment_http` provider execution for payment-rail actions, structured receipt creation after execution, and append-only outcome recording/listing. Action read/list responses include `status_reason` when the latest status transition recorded a reason, such as a failed eligibility precondition or spend-window breach.

## Agentic x402 Payments

x402 payments use the same financial authorization source of truth, but they add a pre-signing lifecycle because an agent can discover many payment requirements concurrently before anything settles.

```text
x402 payment requirement
    -> authorize FinancialAction
    -> mandate + financial policies
    -> reserve session budget
    -> agent signs/pays
    -> commit with settlement proof
       or rollback before settlement
```

`POST /v1/financial/agentic-payments/authorize` is the first call an agent runtime makes after receiving an x402 payment requirement. The service normalizes the requirement into a canonical hash over amount, payee, network, asset, scheme, resource, method, host, and facilitator. That hash is persisted on the action and reservation so commit can prove the settlement corresponds to the exact authorized requirement. The action uses `kind: payment`, `rail: x402`, the runtime principal, the normalized amount, and an x402 counterparty derived from `pay_to`.

Runtime workspace API keys bind the acting principal. If a key has `principal_id`, the request principal must match it; otherwise the API key id is the principal. Dashboard/internal calls may submit a principal explicitly. This prevents a runtime agent from authorizing payment under another agent's budget identity.

Payment sessions bound concurrent reservations to one time-boxed budget. The first authorization for a session creates `financial_payment_sessions` with the requested session limit; later authorizations for the same session lock that row and update reserved/committed/released counters atomically. A reservation is idempotent for `(workspace_id, session_id, payment_requirement_hash)`, conflicts if the same requirement is reused for a different action, and conflicts if projected reserved plus committed spend would exceed the session max. This row lock is the infrastructure-level guard against stale balance reads under parallel agent payments.

Commit only accepts an authorized action. Held actions must be approved first through the normal financial approval endpoint. `commit` verifies the supplied x402 settlement proof against the stored normalized requirement, commits the reservation, releases the pre-signing ledger reservation, writes an executed ledger entry, creates a `financial_execution_receipt.v1`, and appends a succeeded outcome. `rollback` only releases an unsettled internal reservation and fails the action; it is not a blockchain or provider reversal after settlement.

If `CreateFinancialActionRequest.execute` is true and checks leave the action clean, `payment_http` can execute immediately through its managed grant. Externally executed rails stop at `authorized` with an external-attestation grant; the trusted customer connector must later call `/commit`. Held actions write a reserved ledger entry, and finalization releases that reservation before recording executed spend. Decision receipts use `schema: "financial_action_decision_receipt.v1"` and product-facing `authorization_scope` fields. For `rail: payment_http`, the service resolves the workspace's vaulted provider connection, claims the grant before the provider call, reuses the action id as provider idempotency key, and atomically records the committed grant, executed action, ledger, outcome, and receipt after success. A concurrent caller cannot claim the same active grant.

## Bound Execution

Every newly authorized generic action receives a time-limited `FinancialExecutionGrant`. Its `action_hash` is SHA-256 over a recursively key-sorted `financial_action_authorization.v1` snapshot containing workspace, environment, action, evidence, and the persisted evaluation. Mutable provider results and lifecycle fields are excluded.

`payment_http` uses `managed_executor`: TrustLoopGuard executes the persisted action itself after atomically claiming the grant. `card`, `ach`, `wire`, `internal`, and `other` use `external_attestation`: a separately configured executor signs a length-prefixed message with HMAC-SHA256 over the action hash, grant, provider/reference/status, execution time, idempotency key, and provider-proof digest. The server recomputes every hash, checks connector rail/operation scope and expiry, and accepts exact replay only when commit identity matches.

An external connector attestation proves that the configured customer executor attested to execution. It is not provider-native or bank settlement proof. x402 remains stronger and separate: its commit verifies settlement proof against the normalized payment requirement hash.

## Financial Observe Mode

Financial mode resolves from `WorkspaceSettings.financial_action_mode` plus the selected environment override. The default is `enforce`; callers cannot select mode in an action body. In `observe`, the service runs the real mandate, evidence, policy, and ledger-window evaluation and persists `would_allow`, `would_hold`, or `would_block`, while leaving actual action status `proposed`.

Observe mode never authorizes, holds, denies, creates an approval request, reserves money, issues a grant, calls a provider, writes ledger/outcome/receipt rows, or exposes a signable x402 authorization. `execute: true` is deliberately accepted and suppressed so a pilot can run against the production-shaped integration without moving money.

Observation summaries never add unlike currencies. Each currency reports observed amounts and counts, adverse and would-hold rates, reasons, and reviewed false positives. The false-positive denominator is reviewed adverse observations only; unreviewed rows are not counted as correct.

The web dashboard reads this state through Rust APIs and same-origin proxy routes. `/financial` shows the financial action ledger, latest outcomes, inline approve/deny controls for pending approval requests, spending controls, and payment-provider setup state; its Spending controls card creates financial policies through `/api/financial/policies`, which proxies Rust `/v1/financial/policies`. Approving a held row calls the financial approve endpoint and then the execute endpoint so held actions resume execution. `/financial/mandates` lists, creates, and revokes mandates. `/financial/actions/{id}/decision` shows the action's decision receipt. `/financial/receipts/{id}` shows the execution receipt proof, provider response, latest outcome/recovery state, and ledger event ids. The dashboard does not own financial authorization state.

When an action includes a `MandateRef`, the service resolves the mandate in the same workspace before applying policy. The referenced mandate must be active, match the action principal, be inside its start/expiry window, and cover the action according to any structured scope fields present today: `action_kinds`, `operation`, `rail`, `currency` or `currencies`, `max_amount_minor`, `allowed_counterparty_ids`, and for x402 payments `allowed_hosts`, `allowed_resources`, `allowed_networks`, `allowed_assets`, and `allowed_pay_to`. A failed mandate proof transitions the action to `denied` so the attempt remains auditable. A missing mandate on an action is still governed by `family: financial` policy through `mandate_required`; the runtime does not silently convert generic guard events into financial actions.

Decision receipts include `authorization_scope.mandate_hash` and `authorization_scope.normalized_scope` when a mandate was checked. The hash lets a customer prove which mandate boundary was evaluated without relying on dashboard copy. If a matching financial policy requires a mandate and no scope exists, the receipt reports `authorization_scope.result: "missing"` and the x402 response is not signable.

`demo/financial-refund` is the offline wedge demo for this surface. It uses SDK-shaped financial helpers to create a refund mandate, submit typed refund actions, prove normal execution, hold-approval-resume, denial without provider execution, duplicate idempotency, receipt export, and outcome recording.

## Policy Family

`family: financial` policies apply to typed financial actions only. They do not run on generic `/v1/events` guard events, and generic guard events are not converted into financial actions. Payment controls are expressed as financial policies that select `action_kinds: [payment]`, `operations`, currencies, and rails. `kind` is the broad money-bearing action category; `operation` is the stable business operation an SDK wrapper sets on every submitted action.

Financial spending controls can be authored as YAML family policies for tests and fixtures through `POST /v1/policies`, or through the typed `POST /v1/financial/policies` JSON endpoint used by the dashboard. Both paths store the same Rust-owned `family: financial` policy record in the unified policy registry. Runtime loading is environment-aware through policy deployment state.

Selectors live under `when`:

- `agents`
- `action_kinds`
- `operations`
- `currencies`
- `rails`

Controls include per-action caps, hold thresholds, approval thresholds, approver roles for policy-created holds, mandate requirements, counterparty allow/deny lists, new-counterparty holds, refund-original-method-only rules, and required eligibility preconditions.

The pure evaluator in `tl-engine` checks fields present on the `FinancialAction` and policy, including first-class `operation`, plus a pure helper for caller-supplied window totals. Stateful checks such as ledger windows, mandate validity, approval request creation, approver actor capture, eligibility evidence, and provider execution belong in the Rust server financial authorization service. Ledger windows are backed by `tl-storage` financial ledger entries, not generic traces.

SDKs expose financial operation helpers so customer agents do not hand-build the hidden parts of the contract. A refund agent should define `issue_refund` once, then call the helper with order facts and trusted evidence. The helper still submits `CreateFinancialActionRequest`; it only centralizes operation, principal, rail, idempotency, evidence, and action construction.

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
