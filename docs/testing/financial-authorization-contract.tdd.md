# Financial Authorization Contract TDD Evidence

Source plan: `.claude/PRPs/plans/agentic-financial-authorization.plan.md`

This report covers the first implementation slices of the PRP: shared financial wire contracts, the `family: financial` policy shape, the pure engine evaluator for typed financial actions, and durable financial storage.

## User Journeys

| Journey | Implemented guarantee |
|---|---|
| As an agent platform, I can submit a typed financial action instead of a generic guard event. | `tl-core` exposes `FinancialAction`, `MoneyAmount`, counterparty/mandate/evidence refs, decisions, receipts, and outcome/recovery types. |
| As a policy author, I can express financial controls separately from legacy event-path payment caps. | `tl-policy` parses and validates `family: financial` without breaking `family: payment`. |
| As the runtime engine, I can evaluate action-local financial controls without storage or provider dependencies. | `tl-engine` exposes `evaluate_financial_policies` for selectors, per-action caps, hold thresholds, mandate presence, and counterparty rules. |
| As the authorization service, I can persist financial actions and calculate spend from a ledger instead of traces. | `tl-storage` exposes `FinancialRepo` with idempotent action creation, append-only events, status transitions, and net spend-window queries. |

## RED/GREEN Evidence

| Slice | RED evidence | GREEN evidence |
|---|---|---|
| `tl-core` financial contract | `cargo test -p tl-core financial_wire` failed with unresolved `tl_core::financial` and missing root `FinancialActionKind`. | `cargo test -p tl-core --test financial_wire` passed 5 tests. `cargo test -p tl-core` passed 32 unit tests, 5 financial wire tests, 3 harden wire tests, and 1 module export test. |
| `tl-policy` financial family | `cargo test -p tl-policy financial_policy` failed with missing `FamilyPolicy::Financial`. | `cargo test -p tl-policy financial_policy` passed 3 financial parser tests. `cargo test -p tl-policy` passed 57 tests. |
| `tl-engine` financial evaluator | `cargo test -p tl-engine --test financial_policy` failed with missing `tl_engine::evaluate_financial_policies`. | `cargo test -p tl-engine --test financial_policy` passed 5 tests. `cargo test -p tl-engine` passed 168 unit tests and 5 financial integration tests. |
| `tl-storage` financial repository | `cargo test -p tl-storage --features postgres-it --test financial_repo` failed with unresolved `tl_storage::FinancialRepo` and `FinancialLedgerEntryKind`. | `cargo test -p tl-storage --features postgres-it --test financial_repo` passed 3 Postgres-backed tests. |

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
| `cargo test -p tl-storage --features postgres-it --test financial_repo` | PASS | Uses testcontainers Postgres; covers financial action idempotency, tenant isolation, status events, and ledger-derived spend. |

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
| 10 | Financial action creation is idempotent per workspace and tenant-scoped. | `crates/tl-storage/tests/financial_repo.rs` | Postgres integration | PASS |
| 11 | Financial status transitions append events and reject terminal/regressive changes. | `crates/tl-storage/tests/financial_repo.rs` | Postgres integration | PASS |
| 12 | Spend windows use net reserved/executed ledger entries and exclude released holds. | `crates/tl-storage/tests/financial_repo.rs` | Postgres integration | PASS |

## Known Gaps

The PRP is not complete. Remaining slices include the server service/API, OpenAPI registration/codegen, SDK helpers, dashboard pages, demo, receipts/outcomes service behavior, and broader docs for the full financial authorization runtime.
