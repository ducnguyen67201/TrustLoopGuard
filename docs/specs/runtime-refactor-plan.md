# Runtime Refactor Implementation Plan

This plan describes how to refactor the Rust runtime code into a cleaner,
human-readable structure without changing product behavior or public API
contracts. It is an implementation spec, not the canonical architecture source;
`docs/concept/` remains the source of truth for product architecture and must be
updated only when code ownership, endpoint semantics, or contract behavior
changes.

The ordered execution checklist and completion evidence live in
[`runtime-refactor-jobs.md`](runtime-refactor-jobs.md).

## Goal

TrustLoopGuard should read like the runtime product it is:

- `tl-core`: protocol and public wire vocabulary.
- `tl-policy`: policy DSL parsing, validation, and compilation surface.
- `tl-engine`: real-time guardrail runtime and evaluation pipeline.
- `tl-server`: HTTP, auth, routing, request orchestration, and worker dispatch.
- `tl-storage`: durable persistence and Postgres repositories.

The refactor must preserve existing behavior, generated contracts, OpenAPI
output, SDK compatibility, and benchmark posture.

## TDD Rules

Every implementation phase follows the same loop:

1. **RED**: add or adjust a test, compile assertion, or characterization check
   that describes the phase target. For pure refactors, compile-time RED is
   acceptable when the failure is the expected missing module/export.
2. **GREEN**: move the minimum code necessary to make the same target pass.
3. **REFACTOR**: remove dead imports, reduce glue, and keep compatibility
   re-exports stable.
4. **VERIFY**: run the phase-specific gates before starting the next phase.

Do not treat syntax errors, missing dependencies, or unrelated failures as a
valid RED state. If a baseline command already fails before the refactor, record
the exact command and failure summary before changing code.

## Phase 0: Baseline Evidence

Purpose: know the starting point before moving files.

Execution result: complete. Phase 0 had no product test failure. The only
recorded failure was local environment setup: Postgres-linked Rust commands
could not find Homebrew `libpq` until the linker/library environment prefix was
set. With that prefix, `cargo test --workspace --all-targets`,
`cargo bench -p tl-engine --bench check_pipeline`,
`cargo run -p tl-codegen -- --check`, and `pnpm test:backend` all passed. See
[`runtime-refactor-jobs.md`](runtime-refactor-jobs.md#phase-0-baseline-evidence)
for the checked evidence.

Implementation jobs:

- [x] Review `git status --short` and note unrelated/untracked files.
- [x] Run all baseline verification commands.
- [x] Record any pre-existing failures in the active job log.
- [x] Do not edit tracked source files in this phase.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test --workspace --all-targets`
- [x] `cargo bench -p tl-engine --bench check_pipeline`
- [x] `cargo run -p tl-codegen -- --check`
- [x] `pnpm test:backend` or `make backend-test`, if available

Phase done when:

- [x] Every baseline command passed, or each failure is documented as
      pre-existing with exact command and summary.
- [x] The engine benchmark output is saved or summarized for later comparison.

## Phase 1: Server Shell Cleanup

Purpose: make `tl-server/src/lib.rs` a small module/export surface.

TDD RED:

- [x] Add or update a compile test/import path proving these stable exports
      remain available: `tl_server::router`, `tl_server::ApiDoc`,
      `tl_server::health`.
- [x] Add an internal compile reference for the intended app/API split if
      practical.
- [x] Run `cargo test -p tl-server --all-targets` and confirm the failure is
      the expected missing module/export.

Implementation jobs:

- [x] Create `crates/tl-server/src/app/mod.rs`.
- [x] Move router construction to `app/router.rs`.
- [x] Move OpenAPI registration to `app/openapi.rs`.
- [x] Move API error helpers to `app/error.rs`.
- [x] Move HTTP logging middleware to `app/middleware.rs`.
- [x] Create `crates/tl-server/src/api/mod.rs`.
- [x] Move `/health` and `/v1/check` HTTP handlers to `api/guard.rs`.
- [x] Keep route paths, auth layering, middleware order, state usage, and
      OpenAPI schemas unchanged.
- [x] Re-export compatibility symbols from `lib.rs`.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test -p tl-server --all-targets`
- [x] `cargo run -p tl-codegen -- --check`

Phase done when:

- [x] `tl-server/src/lib.rs` is reduced to module declarations, public exports,
      and minimal crate docs.
- [x] OpenAPI check passes without generated artifact drift.
- [x] No route behavior, status code, or auth behavior changed.

## Phase 2: Guard Service Extraction

Purpose: make `/v1/check` read as `api -> service -> engine/storage/workers`.

TDD RED:

- [x] Add or confirm focused tests for redaction-required workspace rejection.
- [x] Add or confirm focused tests for invalid run/run-event combinations.
- [x] Add or confirm focused tests for inline run event creation before check.
- [x] Add or confirm focused tests for enabled runtime policy loading.
- [x] Add or confirm focused tests for escalation dispatch on `Escalate`.
- [x] Run `cargo test -p tl-server --test guardrails` and confirm RED only if
      a new service API/export is intentionally missing.

Implementation jobs:

- [x] Create `crates/tl-server/src/services/mod.rs`.
- [x] Create `crates/tl-server/src/services/guard_service.rs`.
- [x] Move check orchestration out of the HTTP handler.
- [x] Keep `api/guard.rs` responsible for request extraction, redaction info
      validation, workspace header/body selection, and JSON/error response.
- [x] Keep workspace settings resolution, redaction enforcement, run event
      creation, policy loading, engine call, trace dispatch, and escalation
      dispatch in the service.
- [x] Preserve full-handler latency semantics for `Decision.latency_ms`.
- [x] Preserve every existing HTTP status and `ApiErrorCode` mapping.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test -p tl-server --test guardrails`
- [x] `cargo test -p tl-server --test full_pipeline`
- [x] `cargo test -p tl-server --all-targets`

Phase done when:

- [x] The route handler is thin.
- [x] The service owns the runtime check workflow.
- [x] Existing guardrail and full pipeline tests pass.

## Phase 3: App State Decomposition

Purpose: split boot wiring, memory wiring, Postgres wiring, env parsing, and
adapters into readable modules.

TDD RED:

- [x] Add or update compile/import checks for `tl_server::AppState`,
      `tl_server::BuildOptions`, `tl_server::build_app_state`, and
      `tl_server::memory_app_state`.
- [x] Add or preserve focused tests for environment-derived auth settings.
- [x] Run `cargo test -p tl-server --all-targets` and confirm expected RED if
      target modules/exports are not yet present.

Implementation jobs:

- [x] Create `crates/tl-server/src/state/`.
- [x] Move `AppState` and `BuildOptions` to `state/app_state.rs`.
- [x] Move high-level `build_app_state` to `state/build.rs`.
- [x] Move `memory_app_state` and memory store construction to
      `state/memory.rs`.
- [x] Move Postgres construction and feature-gated boot wiring to
      `state/postgres.rs`.
- [x] Move env parsing helpers to `state/env.rs`.
- [x] Move escalation/trace worker setup to `state/workers.rs` if needed.
- [x] Move Postgres adapter impls to `state/postgres_adapters.rs`.
- [x] Preserve all feature gates and memory-only behavior.
- [x] Re-export stable state APIs from `state/mod.rs` and `lib.rs`.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test -p tl-server --all-targets`
- [x] `cargo test -p tl-server --no-default-features --all-targets`
- [x] `cargo test -p tl-storage --features postgres --all-targets`

Phase done when:

- [x] No single state module carries unrelated boot, env, memory, Postgres, and
      adapter responsibilities.
- [x] Default-feature and no-default-feature server tests pass.

## Phase 4: Gateway Decomposition

Purpose: split gateway API, service orchestration, provider forwarding,
normalization, crypto, checks, errors, and memory store.

TDD RED:

- [x] Add or confirm tests for gateway route/profile/provider normalization.
- [x] Add or confirm tests for seal key and credential crypto behavior.
- [x] Add or confirm tests for proxy guard check behavior.
- [x] Add or confirm tests for provider error mapping.
- [x] Run `cargo test -p tl-server --test gateway` and confirm expected RED if
      new module/export names are not yet present.

Implementation jobs:

- [x] Create `crates/tl-server/src/gateway/`.
- [x] Move route handlers to `gateway/api.rs`.
- [x] Move proxy orchestration to `gateway/service.rs`.
- [x] Move provider trait and forwarding helpers to `gateway/provider.rs`.
- [x] Move normalization helpers to `gateway/normalization.rs`.
- [x] Move credential sealing helpers to `gateway/crypto.rs`.
- [x] Move guard check/regeneration helpers to `gateway/checks.rs`.
- [x] Move gateway-specific errors to `gateway/errors.rs`.
- [x] Move memory store implementation to `gateway/memory_store.rs`.
- [x] Preserve public names through `gateway/mod.rs` re-exports.
- [x] Keep OpenAPI path registration pointing to the moved handlers.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test -p tl-server --test gateway`
- [x] `cargo test -p tl-server --all-targets`
- [x] `cargo run -p tl-codegen -- --check`

Phase done when:

- [x] Gateway API handlers are thin.
- [x] Provider and guard enforcement flow are readable separately.
- [x] Gateway tests and codegen check pass.

## Phase 5: Core Contract Decomposition

Purpose: make `tl-core` readable without changing generated contracts.

TDD RED:

- [x] Add or update compile/import checks proving downstream crates can still
      import existing `tl_core::*` public names.
- [x] Run `cargo test -p tl-core --all-targets`; expected RED is missing
      module/export during the move, not serialization behavior.

Implementation jobs:

- [x] Move guard protocol types to `guard/`.
- [x] Move redaction types to `guard/redaction.rs`.
- [x] Move API error envelope/code to `error.rs`.
- [x] Move run, trace, analytics, and knowledge DTOs out of `lib.rs` where the
      move is mechanical.
- [x] Preserve all `serde` names, optional codegen derives, and public type
      names.
- [x] Preserve `tl_core::*` re-exports for server, storage, SDKs, and tests.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test -p tl-core --all-targets`
- [x] `cargo test -p tl-sdk-rust --all-targets`
- [x] `cargo test -p tl-server --all-targets`
- [x] `cargo run -p tl-codegen -- --check`

Phase done when:

- [x] `tl-core/src/lib.rs` is primarily crate docs, module declarations, and
      public re-exports.
- [x] Codegen check passes with no unexpected OpenAPI/schema/SDK drift.

## Phase 6: Engine Runtime Decomposition

Purpose: make `tl-engine` read as engine, pipeline, tiers, context, and
matchers.

TDD RED:

- [x] Add or update compile/import checks for `tl_engine::Engine`,
      `tl_engine::HandlerCtx`, `tl_engine::TierRunner`, and
      `tl_engine::DefaultTierRunner`.
- [x] Preserve behavior tests for empty engine allow, tier cancellation, tier 3
      timeout escalation, and default runner tier statuses.
- [x] Run `cargo test -p tl-engine --all-targets`; expected RED is missing
      module/export during the move.

Implementation jobs:

- [x] Move `Engine` to `engine.rs`.
- [x] Move orchestration code to `pipeline/orchestrator.rs`.
- [x] Move tier runner traits/types to `pipeline/tier_runner.rs`.
- [x] Move policy cache scope helper to `pipeline/cache_scope.rs`.
- [x] Move `tier1.rs` to `tiers/deterministic.rs`.
- [x] Move `tier2.rs` to `tiers/fuzzy.rs`.
- [x] Move `tier3.rs` to `tiers/llm.rs`.
- [x] Move handler context and resolver traits to `context/`.
- [x] Move matcher logic to `matchers/policy_match.rs`.
- [x] Preserve stable public re-exports from `lib.rs`.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test -p tl-engine --all-targets`
- [x] `cargo bench -p tl-engine --bench check_pipeline`

Phase done when:

- [x] Public engine APIs remain stable.
- [x] Engine tests pass.
- [x] Benchmark output shows no meaningful hot-path regression, or the
      regression is explicitly documented and accepted.

## Phase 7: Storage Readability Pass

Purpose: decide whether storage needs file moves after server cleanup.

TDD RED:

- [x] Add or update compile/import checks proving existing `tl_storage::*`
      repository exports remain stable.
- [x] If moving files under a `repositories/` folder, run
      `cargo test -p tl-storage --all-targets` and confirm expected RED from
      missing modules/exports.

Implementation jobs:

- [x] Prefer no move if current flat repository files are readable enough after
      server cleanup.
- [x] If moving, create `repositories/` and preserve every public export.
- [x] Do not change Diesel schema, migrations, or query semantics.
- [x] Keep `writer.rs` focused on async trace writes.

Testing and verification:

- [x] `cargo fmt --check`
- [x] `cargo test -p tl-storage --all-targets`
- [x] `cargo test -p tl-storage --features postgres --all-targets`
- [x] Optional with Docker: `cargo test -p tl-storage --features postgres-it`

Phase done when:

- [x] Storage exports remain compatible.
- [x] Repository behavior tests pass.
- [x] No schema or migration drift was introduced.

## Phase 8: Documentation and Concept Sync

Purpose: sync docs with the final code organization and fix known drift.

TDD RED:

- [x] Run `cargo run -p tl-codegen -- --check` before docs/codegen updates.
- [x] Search docs for stale moved source paths and outdated crate count.

Implementation jobs:

- [x] Update `docs/concept/crates.md` crate count and dependency graph.
- [x] Update `docs/concept/architecture.md` only if request flow or source
      references changed.
- [x] Update `docs/openapi.yaml` only through `tl-codegen` if output changed.
- [x] Check `docs/concept/glossary.md` for stale terms.
- [x] Avoid duplicating concept explanations across docs.
- [x] Avoid scaffolding language in `docs/concept/`.

Testing and verification:

- [x] `cargo run -p tl-codegen -- --check`
- [x] `pnpm docs:diagrams` or `make diagrams` only if diagrams changed
- [x] `grep -RIn "TODO\\|Placeholder\\|Phase " docs/concept`

Phase done when:

- [x] Docs match the refactored code.
- [x] Concept docs still each own one topic.
- [x] No generated docs/artifacts are stale.

## Phase 9: Crate Boundary Audit

Purpose: decide whether support crates still deserve independent crate
boundaries after readability cleanup.

TDD RED:

- [x] No production-code RED is required for audit-only work.
- [x] If a crate merge is selected, add compile/import tests for the intended
      final public surface and confirm RED first.

Implementation jobs:

- [x] Audit `tl-cache`.
- [x] Audit `tl-fuzzy`.
- [x] Audit `tl-llm`.
- [x] Audit `tl-stream`.
- [x] Audit `tl-replay`.
- [x] Default to keeping crates unless there is a concrete simplification with
      low migration risk.
- [x] Update `docs/concept/crates.md` with final decisions.

Testing and verification:

- [x] `cargo tree -p tl-server`
- [x] `cargo tree -p tl-engine`
- [x] `cargo test --workspace --all-targets`

Phase done when:

- [x] Every support crate has a documented keep/merge decision.
- [x] No crate is merged without a separate RED/GREEN cycle.

## Final Acceptance Gates

The refactor is complete only when these pass or have a documented external
blocker:

- [x] `git status --short`
- [x] `cargo fmt --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace --all-targets`
- [x] `cargo test -p tl-server --no-default-features --all-targets`
- [x] `cargo test -p tl-storage --features postgres --all-targets`
- [x] `cargo bench -p tl-engine --bench check_pipeline`
- [x] `cargo run -p tl-codegen -- --check`
- [x] `pnpm test:backend` or `make backend-test`, if available
- [x] If web files changed: `pnpm --filter web typecheck` and relevant web tests
- [x] If diagrams changed: `pnpm docs:diagrams` or `make diagrams`
- [x] Review `git diff` and confirm every changed line belongs to the refactor
