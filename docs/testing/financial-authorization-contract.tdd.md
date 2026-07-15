# Financial authorization contract tests

Financial authorization uses the common kernel and keeps execution/ledger proof in the financial domain.

## Contract matrix

| Behavior | Required coverage |
|---|---|
| Typed financial action | Wire tests cover action, evidence, common claim, authorization fields, and separate execution status. |
| Policy evaluation | Parser/engine tests cover approval threshold, grant requirement, counterparty rules, hard caps, missing evidence deferral, and failed-precondition denial. |
| Common authority | Coordinator tests prove requirement ID, capability, principal, domain, typed scope, expiry, revocation, and current-policy matching. |
| Approval signing | HTTP/storage tests prove authenticated role checks, envelope-hash conflicts, expiry, and transactional grant creation. |
| Saved grant | Service tests prove matching reuse removes only human review and never bypasses hard policy, eligibility, or live budget. |
| Lease | Storage tests prove exact-once concurrency, stable same-attempt retry, fingerprint/grant mismatch conflict, and consume/cancel behavior. |
| Execution | Service tests prove budget reservation before provider call, separate execution transitions, failure release, and linked receipts. |
| Tenant isolation | Repository and HTTP tests cover workspace, environment, and runtime-key principal boundaries. |
| UI | Component/proxy tests cover one approvals queue, hash-bound decisions, typed grant creation/revocation, and a read-only financial ledger. |
| SDK/demo | SDK tests prove callback-at-most-once and the refund demo proves saved-grant, approval-required, idempotency, and linked receipt flows. |

## Required gates

Run the focused Rust crates, Postgres authorization repository tests, all three SDK suites, MCP proxy tests, web tests, code generation check, demo checks, and the repository verification commands listed in the implementation plan.
