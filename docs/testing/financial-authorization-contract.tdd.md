# Financial Authorization Contract TDD Evidence

Source plan: `.claude/PRPs/plans/agentic-financial-authorization.plan.md`

This report covers the first implementation slices of the PRP: shared financial wire contracts, the `family: financial` policy shape, the pure engine evaluator for typed financial actions, durable financial storage, the first Rust HTTP action endpoints, initial TypeScript/Python/Rust SDK helpers, and the first `FinancialAuthorizationService` orchestration seam.

## User Journeys

| Journey | Implemented guarantee |
|---|---|
| As an agent platform, I can submit a typed financial action instead of a generic guard event. | `tl-core` exposes `FinancialAction`, `MoneyAmount`, counterparty/mandate/evidence refs, decisions, receipts, and outcome/recovery types. |
| As a policy author, I can express financial controls separately from legacy event-path payment caps. | `tl-policy` parses and validates `family: financial` without breaking `family: payment`. |
| As the runtime engine, I can evaluate action-local financial controls without storage or provider dependencies. | `tl-engine` exposes `evaluate_financial_policies` for selectors, per-action caps, hold thresholds, mandate presence, and counterparty rules. |
| As the authorization service, I can persist financial actions and calculate spend from a ledger instead of traces. | `tl-storage` exposes `FinancialRepo` with idempotent action creation, append-only events, status transitions, and net spend-window queries. |
| As an SDK or platform caller, I can create, list, and advance financial actions through the Rust HTTP API. | `tl-server` exposes `POST /v1/financial/actions`, `GET /v1/financial/actions`, `GET /v1/financial/actions/{id}`, and approve/deny/execute transition endpoints. |
| As a TypeScript, Python, or Rust integrator, I can call financial action APIs without hand-building paths. | SDK clients expose financial verify/guard-payment helpers, get/approve/deny/execute helpers, and typed financial action responses. |
| As the Rust server, I have one service seam for financial action orchestration. | `FinancialAuthorizationService` owns validation and create/list/get/approve/deny/execute intent before delegating to `FinancialStore`. |
| As an approver workflow, I have a durable queue for held financial actions. | `FinancialAuthorizationService::hold_action` creates pending `FinancialApprovalRequest` rows and Rust exposes `GET /v1/financial/approval-requests`. |
| As an authorization owner, I can create, list, and revoke financial mandates as durable scopes. | `tl-core`, `tl-storage`, `tl-server`, and SDKs expose typed mandate create/list/revoke behavior through `FinancialAuthorizationService`. |

## RED/GREEN Evidence

| Slice | RED evidence | GREEN evidence |
|---|---|---|
| `tl-core` financial contract | `cargo test -p tl-core financial_wire` failed with unresolved `tl_core::financial` and missing root `FinancialActionKind`. | `cargo test -p tl-core --test financial_wire` passed 5 tests. `cargo test -p tl-core` passed 32 unit tests, 5 financial wire tests, 3 harden wire tests, and 1 module export test. |
| `tl-policy` financial family | `cargo test -p tl-policy financial_policy` failed with missing `FamilyPolicy::Financial`. | `cargo test -p tl-policy financial_policy` passed 3 financial parser tests. `cargo test -p tl-policy` passed 57 tests. |
| `tl-engine` financial evaluator | `cargo test -p tl-engine --test financial_policy` failed with missing `tl_engine::evaluate_financial_policies`. | `cargo test -p tl-engine --test financial_policy` passed 5 tests. `cargo test -p tl-engine` passed 168 unit tests and 5 financial integration tests. |
| `tl-storage` financial repository | `cargo test -p tl-storage --features postgres-it --test financial_repo` failed with unresolved `tl_storage::FinancialRepo` and `FinancialLedgerEntryKind`. | `cargo test -p tl-storage --features postgres-it --test financial_repo` passed 3 Postgres-backed tests. |
| `tl-server` financial endpoints | `cargo test -p tl-server --test financial_actions` failed with 404 responses for `/v1/financial/actions`. | `cargo test -p tl-server --test financial_actions` passed 2 endpoint tests. |
| TypeScript/Python SDK financial helpers | `pnpm --dir sdks/typescript test -- financial-actions.test.ts` failed with missing `Client.verifyAction`; Python focused tests failed with missing financial exports. | TypeScript focused tests passed 3 financial tests; Python focused tests passed 2 financial tests. |
| Rust SDK financial helpers | `cargo test -p tl-sdk-rust --test financial_actions_integration` failed with missing financial root exports and missing `Client::verify_action`/`guard_payment`/transition methods. | Focused Rust SDK financial tests passed 3 tests, and the full `cargo test -p tl-sdk-rust` suite passed. |
| `FinancialAuthorizationService` orchestration seam | `cargo test -p tl-server --test financial_authorization_service` failed with missing `tl_server::FinancialAuthorizationService`. | Focused service tests passed 3 tests, and existing financial endpoint tests passed through the service path. |
| Financial action listing | Focused list tests failed with missing `FinancialAuthorizationService::list_actions` and `405 Method Not Allowed` for `GET /v1/financial/actions`. | Storage, service, and router list tests pass with tenant-scoped newest-first ordering. |
| SDK financial action listing | Focused SDK tests failed with missing `listFinancialActions`, missing Python `FinancialActionListResponse` export, and missing Rust `Client::list_financial_actions`. | TypeScript, Python, and Rust SDK suites pass with list helpers. |
| Durable financial approval requests | Focused tests failed with missing approval request wire types, missing `FinancialRepo::create_approval_request`, missing service hold/list methods, and missing `GET /v1/financial/approval-requests`. | Core, Postgres repo, service, and router tests pass for pending approval request creation/listing. |
| Financial approval request resolution | `cargo test -p tl-server --test financial_authorization_service` failed because held-action approve/deny left approval requests in `Pending`. | Service tests pass with approve/deny resolving pending queue items; Postgres repo tests pass for tenant/action-scoped approval request resolution. |
| Durable financial mandates | Focused tests failed with missing mandate wire types, missing `FinancialRepo` mandate methods, missing service/router endpoints, and missing SDK helpers. | Core, Postgres repo, service, router, and SDK tests pass for mandate create/list/revoke behavior. |

## Validation Commands

| Command | Result | Notes |
|---|---|---|
| `cargo fmt --check` | PASS | Formatting clean after `cargo fmt`. |
| `cargo test -p tl-core` | PASS | Includes new financial wire test target. |
| `cargo test -p tl-policy` | PASS | Confirms existing content/payment families still parse. |
| `cargo test -p tl-engine` | PASS | Confirms new engine export does not break existing engine behavior. |
| `cargo check -p tl-core --features codegen` | PASS | Emits an existing ts-rs warning about a serde `transparent` attribute outside this change. |
| `cargo check -p tl-policy --features schema` | PASS | Schema feature compiles with `FinancialPolicy`. |
| `cargo check -p tl-engine` | PASS | Engine crate compiles with the new evaluator export. |
| `cargo test -p tl-storage --features postgres-it --test financial_repo` | PASS | Uses testcontainers Postgres; covers financial action idempotency, tenant isolation, newest-first listing, approval request listing, mandate lifecycle, status events, and ledger-derived spend. |
| `cargo test -p tl-server --test financial_actions` | PASS | Covers create/list/get/idempotency/approve/execute, approval queue listing, mandate create/list/revoke, and invalid amount handling via the router. |
| `cargo test -p tl-server --test financial_authorization_service` | PASS | Covers service-level create/idempotency, list, get, hold approval request creation, approve, deny, execute, mandate create/list/revoke, and validation behavior. |
| `pnpm --dir sdks/typescript typecheck` | PASS | TypeScript SDK compiles with financial helpers and generated types. |
| `pnpm --dir sdks/typescript test` | PASS | 67 tests passed, including financial action helpers. |
| `sdks/python/.venv/bin/pytest sdks/python/tests` | PASS | 64 tests passed, including financial action helpers. |
| `cargo test -p tl-sdk-rust` | PASS | 43 Rust SDK tests passed, including 5 financial action helper integration tests. |
| `cargo run -p tl-codegen -- --check` | PASS | OpenAPI and generated SDK bindings are up to date with mandate types and endpoints. |
| `cargo check -p tl-server` | PASS | Server crate compiles after mandate route/service/store changes. |
| `cargo check -p tl-storage --features postgres` | PASS | Storage crate compiles with mandate repository methods under the Postgres feature. |

## Test Specification

| # | What is guaranteed | Test file or command | Test type | Result |
|---|---|---|---|---|
| 1 | Financial wire enums serialize with snake_case values. | `crates/tl-core/tests/financial_wire.rs` | Unit/integration | PASS |
| 2 | Financial actions use integer minor-unit money and optional typed refs. | `crates/tl-core/tests/financial_wire.rs` | Unit/integration | PASS |
| 3 | Financial decisions carry action status, verdict, approval, and receipt refs. | `crates/tl-core/tests/financial_wire.rs` | Unit/integration | PASS |
| 4 | Financial outcomes record recovery/loss state without float accounting fields. | `crates/tl-core/tests/financial_wire.rs` | Unit/integration | PASS |
| 5 | `family: financial` parses selectors, caps, counterparty, mandate, and eligibility controls. | `crates/tl-policy/src/family_parse.rs` | Unit | PASS |
| 6 | Invalid financial policies fail when selectors/controls are missing or amounts/actions are invalid. | `crates/tl-policy/src/family_parse.rs` | Unit | PASS |
| 7 | Pure financial evaluator blocks non-positive amounts and per-action cap breaches. | `crates/tl-engine/tests/financial_policy.rs` | Integration | PASS |
| 8 | Pure financial evaluator escalates hold thresholds and missing mandate requirements. | `crates/tl-engine/tests/financial_policy.rs` | Integration | PASS |
| 9 | Pure financial evaluator blocks denied counterparties and ignores non-matching actions. | `crates/tl-engine/tests/financial_policy.rs` | Integration | PASS |
| 10 | Financial action creation and listing are idempotent, tenant-scoped, and newest-first per workspace. | `crates/tl-storage/tests/financial_repo.rs` | Postgres integration | PASS |
| 11 | Financial status transitions append events and reject terminal/regressive changes. | `crates/tl-storage/tests/financial_repo.rs` | Postgres integration | PASS |
| 12 | Spend windows use net reserved/executed ledger entries and exclude released holds. | `crates/tl-storage/tests/financial_repo.rs` | Postgres integration | PASS |
| 13 | HTTP callers can create, list, idempotently replay, read, approve, and execute financial actions. | `crates/tl-server/tests/financial_actions.rs` | Router integration | PASS |
| 14 | HTTP action creation rejects missing/non-positive amounts before storage. | `crates/tl-server/tests/financial_actions.rs` | Router integration | PASS |
| 15 | TypeScript SDK posts, lists, and transitions financial actions through typed methods. | `sdks/typescript/test/financial-actions.test.ts` | SDK unit | PASS |
| 16 | Python SDK posts, lists, and transitions financial actions through sync client methods. | `sdks/python/tests/test_financial_actions.py` | SDK unit | PASS |
| 17 | Rust SDK posts, lists, fetches, and transitions financial actions through typed methods. | `crates/tl-sdk-rust/tests/financial_actions_integration.rs` | SDK integration | PASS |
| 18 | Financial action HTTP handlers delegate lifecycle intent through `FinancialAuthorizationService`. | `crates/tl-server/tests/financial_authorization_service.rs` and `crates/tl-server/tests/financial_actions.rs` | Server integration | PASS |
| 19 | Held actions can create durable pending approval requests and list the approval queue. | `crates/tl-core/tests/financial_wire.rs`, `crates/tl-storage/tests/financial_repo.rs`, `crates/tl-server/tests/financial_authorization_service.rs`, `crates/tl-server/tests/financial_actions.rs` | Contract/storage/server integration | PASS |
| 20 | Approving or denying a held action resolves its pending approval request without touching other workspace/action queue items. | `crates/tl-server/tests/financial_authorization_service.rs`, `crates/tl-storage/tests/financial_repo.rs` | Service/Postgres integration | PASS |
| 21 | Financial mandates can be created, listed newest-first, revoked, and isolated by workspace. | `crates/tl-core/tests/financial_wire.rs`, `crates/tl-storage/tests/financial_repo.rs`, `crates/tl-server/tests/financial_authorization_service.rs`, `crates/tl-server/tests/financial_actions.rs`, SDK financial action tests | Contract/storage/server/SDK integration | PASS |

## Known Gaps

The PRP is not complete. Remaining slices include policy/mandate enforcement during action authorization, provider execution, receipt/outcome behavior, dashboard pages, demo, and broader docs for the full financial authorization runtime.
