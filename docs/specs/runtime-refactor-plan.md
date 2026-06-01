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

- [ ] Review `git status --short` and note unrelated/untracked files.
- [ ] Run all baseline verification commands.
- [ ] Record any pre-existing failures in the active job log.
- [ ] Do not edit tracked source files in this phase.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo bench -p tl-engine --bench check_pipeline`
- [ ] `cargo run -p tl-codegen -- --check`
- [ ] `pnpm test:backend` or `make backend-test`, if available

Phase done when:

- [ ] Every baseline command passed, or each failure is documented as
      pre-existing with exact command and summary.
- [ ] The engine benchmark output is saved or summarized for later comparison.

## Phase 1: Server Shell Cleanup

Purpose: make `tl-server/src/lib.rs` a small module/export surface.

TDD RED:

- [ ] Add or update a compile test/import path proving these stable exports
      remain available: `tl_server::router`, `tl_server::ApiDoc`,
      `tl_server::health`.
- [ ] Add an internal compile reference for the intended app/API split if
      practical.
- [ ] Run `cargo test -p tl-server --all-targets` and confirm the failure is
      the expected missing module/export.

Implementation jobs:

- [ ] Create `crates/tl-server/src/app/mod.rs`.
- [ ] Move router construction to `app/router.rs`.
- [ ] Move OpenAPI registration to `app/openapi.rs`.
- [ ] Move API error helpers to `app/error.rs`.
- [ ] Move HTTP logging middleware to `app/middleware.rs`.
- [ ] Create `crates/tl-server/src/api/mod.rs`.
- [ ] Move `/health` and `/v1/check` HTTP handlers to `api/guard.rs`.
- [ ] Keep route paths, auth layering, middleware order, state usage, and
      OpenAPI schemas unchanged.
- [ ] Re-export compatibility symbols from `lib.rs`.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test -p tl-server --all-targets`
- [ ] `cargo run -p tl-codegen -- --check`

Phase done when:

- [ ] `tl-server/src/lib.rs` is reduced to module declarations, public exports,
      and minimal crate docs.
- [ ] OpenAPI check passes without generated artifact drift.
- [ ] No route behavior, status code, or auth behavior changed.

## Phase 2: Guard Service Extraction

Purpose: make `/v1/check` read as `api -> service -> engine/storage/workers`.

TDD RED:

- [ ] Add or confirm focused tests for redaction-required workspace rejection.
- [ ] Add or confirm focused tests for invalid run/run-event combinations.
- [ ] Add or confirm focused tests for inline run event creation before check.
- [ ] Add or confirm focused tests for enabled runtime policy loading.
- [ ] Add or confirm focused tests for escalation dispatch on `Escalate`.
- [ ] Run `cargo test -p tl-server --test guardrails` and confirm RED only if
      a new service API/export is intentionally missing.

Implementation jobs:

- [ ] Create `crates/tl-server/src/services/mod.rs`.
- [ ] Create `crates/tl-server/src/services/guard_service.rs`.
- [ ] Move check orchestration out of the HTTP handler.
- [ ] Keep `api/guard.rs` responsible for request extraction, redaction info
      validation, workspace header/body selection, and JSON/error response.
- [ ] Keep workspace settings resolution, redaction enforcement, run event
      creation, policy loading, engine call, trace dispatch, and escalation
      dispatch in the service.
- [ ] Preserve full-handler latency semantics for `Decision.latency_ms`.
- [ ] Preserve every existing HTTP status and `ApiErrorCode` mapping.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test -p tl-server --test guardrails`
- [ ] `cargo test -p tl-server --test full_pipeline`
- [ ] `cargo test -p tl-server --all-targets`

Phase done when:

- [ ] The route handler is thin.
- [ ] The service owns the runtime check workflow.
- [ ] Existing guardrail and full pipeline tests pass.

## Phase 3: App State Decomposition

Purpose: split boot wiring, memory wiring, Postgres wiring, env parsing, and
adapters into readable modules.

TDD RED:

- [ ] Add or update compile/import checks for `tl_server::AppState`,
      `tl_server::BuildOptions`, `tl_server::build_app_state`, and
      `tl_server::memory_app_state`.
- [ ] Add or preserve focused tests for environment-derived auth settings.
- [ ] Run `cargo test -p tl-server --all-targets` and confirm expected RED if
      target modules/exports are not yet present.

Implementation jobs:

- [ ] Create `crates/tl-server/src/state/`.
- [ ] Move `AppState` and `BuildOptions` to `state/app_state.rs`.
- [ ] Move high-level `build_app_state` to `state/build.rs`.
- [ ] Move `memory_app_state` and memory store construction to
      `state/memory.rs`.
- [ ] Move Postgres construction and feature-gated boot wiring to
      `state/postgres.rs`.
- [ ] Move env parsing helpers to `state/env.rs`.
- [ ] Move escalation/trace worker setup to `state/workers.rs` if needed.
- [ ] Move Postgres adapter impls to `state/postgres_adapters.rs`.
- [ ] Preserve all feature gates and memory-only behavior.
- [ ] Re-export stable state APIs from `state/mod.rs` and `lib.rs`.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test -p tl-server --all-targets`
- [ ] `cargo test -p tl-server --no-default-features --all-targets`
- [ ] `cargo test -p tl-storage --features postgres --all-targets`

Phase done when:

- [ ] No single state module carries unrelated boot, env, memory, Postgres, and
      adapter responsibilities.
- [ ] Default-feature and no-default-feature server tests pass.

## Phase 4: Gateway Decomposition

Purpose: split gateway API, service orchestration, provider forwarding,
normalization, crypto, checks, errors, and memory store.

TDD RED:

- [ ] Add or confirm tests for gateway route/profile/provider normalization.
- [ ] Add or confirm tests for seal key and credential crypto behavior.
- [ ] Add or confirm tests for proxy guard check behavior.
- [ ] Add or confirm tests for provider error mapping.
- [ ] Run `cargo test -p tl-server --test gateway` and confirm expected RED if
      new module/export names are not yet present.

Implementation jobs:

- [ ] Create `crates/tl-server/src/gateway/`.
- [ ] Move route handlers to `gateway/api.rs`.
- [ ] Move proxy orchestration to `gateway/service.rs`.
- [ ] Move provider trait and forwarding helpers to `gateway/provider.rs`.
- [ ] Move normalization helpers to `gateway/normalization.rs`.
- [ ] Move credential sealing helpers to `gateway/crypto.rs`.
- [ ] Move guard check/regeneration helpers to `gateway/checks.rs`.
- [ ] Move gateway-specific errors to `gateway/errors.rs`.
- [ ] Move memory store implementation to `gateway/memory_store.rs`.
- [ ] Preserve public names through `gateway/mod.rs` re-exports.
- [ ] Keep OpenAPI path registration pointing to the moved handlers.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test -p tl-server --test gateway`
- [ ] `cargo test -p tl-server --all-targets`
- [ ] `cargo run -p tl-codegen -- --check`

Phase done when:

- [ ] Gateway API handlers are thin.
- [ ] Provider and guard enforcement flow are readable separately.
- [ ] Gateway tests and codegen check pass.

## Phase 5: Core Contract Decomposition

Purpose: make `tl-core` readable without changing generated contracts.

TDD RED:

- [ ] Add or update compile/import checks proving downstream crates can still
      import existing `tl_core::*` public names.
- [ ] Run `cargo test -p tl-core --all-targets`; expected RED is missing
      module/export during the move, not serialization behavior.

Implementation jobs:

- [ ] Move guard protocol types to `guard/`.
- [ ] Move redaction types to `guard/redaction.rs`.
- [ ] Move API error envelope/code to `error.rs`.
- [ ] Move run, trace, analytics, and knowledge DTOs out of `lib.rs` where the
      move is mechanical.
- [ ] Preserve all `serde` names, optional codegen derives, and public type
      names.
- [ ] Preserve `tl_core::*` re-exports for server, storage, SDKs, and tests.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test -p tl-core --all-targets`
- [ ] `cargo test -p tl-sdk-rust --all-targets`
- [ ] `cargo test -p tl-server --all-targets`
- [ ] `cargo run -p tl-codegen -- --check`

Phase done when:

- [ ] `tl-core/src/lib.rs` is primarily crate docs, module declarations, and
      public re-exports.
- [ ] Codegen check passes with no unexpected OpenAPI/schema/SDK drift.

## Phase 6: Engine Runtime Decomposition

Purpose: make `tl-engine` read as engine, pipeline, tiers, context, and
matchers.

TDD RED:

- [ ] Add or update compile/import checks for `tl_engine::Engine`,
      `tl_engine::HandlerCtx`, `tl_engine::TierRunner`, and
      `tl_engine::DefaultTierRunner`.
- [ ] Preserve behavior tests for empty engine allow, tier cancellation, tier 3
      timeout escalation, and default runner tier statuses.
- [ ] Run `cargo test -p tl-engine --all-targets`; expected RED is missing
      module/export during the move.

Implementation jobs:

- [ ] Move `Engine` to `engine.rs`.
- [ ] Move orchestration code to `pipeline/orchestrator.rs`.
- [ ] Move tier runner traits/types to `pipeline/tier_runner.rs`.
- [ ] Move policy cache scope helper to `pipeline/cache_scope.rs`.
- [ ] Move `tier1.rs` to `tiers/deterministic.rs`.
- [ ] Move `tier2.rs` to `tiers/fuzzy.rs`.
- [ ] Move `tier3.rs` to `tiers/llm.rs`.
- [ ] Move handler context and resolver traits to `context/`.
- [ ] Move matcher logic to `matchers/policy_match.rs`.
- [ ] Preserve stable public re-exports from `lib.rs`.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test -p tl-engine --all-targets`
- [ ] `cargo bench -p tl-engine --bench check_pipeline`

Phase done when:

- [ ] Public engine APIs remain stable.
- [ ] Engine tests pass.
- [ ] Benchmark output shows no meaningful hot-path regression, or the
      regression is explicitly documented and accepted.

## Phase 7: Storage Readability Pass

Purpose: decide whether storage needs file moves after server cleanup.

TDD RED:

- [ ] Add or update compile/import checks proving existing `tl_storage::*`
      repository exports remain stable.
- [ ] If moving files under a `repositories/` folder, run
      `cargo test -p tl-storage --all-targets` and confirm expected RED from
      missing modules/exports.

Implementation jobs:

- [ ] Prefer no move if current flat repository files are readable enough after
      server cleanup.
- [ ] If moving, create `repositories/` and preserve every public export.
- [ ] Do not change Diesel schema, migrations, or query semantics.
- [ ] Keep `writer.rs` focused on async trace writes.

Testing and verification:

- [ ] `cargo fmt --check`
- [ ] `cargo test -p tl-storage --all-targets`
- [ ] `cargo test -p tl-storage --features postgres --all-targets`
- [ ] Optional with Docker: `cargo test -p tl-storage --features postgres-it`

Phase done when:

- [ ] Storage exports remain compatible.
- [ ] Repository behavior tests pass.
- [ ] No schema or migration drift was introduced.

## Phase 8: Documentation and Concept Sync

Purpose: sync docs with the final code organization and fix known drift.

TDD RED:

- [ ] Run `cargo run -p tl-codegen -- --check` before docs/codegen updates.
- [ ] Search docs for stale moved source paths and outdated crate count.

Implementation jobs:

- [ ] Update `docs/concept/crates.md` crate count and dependency graph.
- [ ] Update `docs/concept/architecture.md` only if request flow or source
      references changed.
- [ ] Update `docs/openapi.yaml` only through `tl-codegen` if output changed.
- [ ] Check `docs/concept/glossary.md` for stale terms.
- [ ] Avoid duplicating concept explanations across docs.
- [ ] Avoid scaffolding language in `docs/concept/`.

Testing and verification:

- [ ] `cargo run -p tl-codegen -- --check`
- [ ] `pnpm docs:diagrams` or `make diagrams` only if diagrams changed
- [ ] `grep -RIn "TODO\\|Placeholder\\|Phase " docs/concept`

Phase done when:

- [ ] Docs match the refactored code.
- [ ] Concept docs still each own one topic.
- [ ] No generated docs/artifacts are stale.

## Phase 9: Crate Boundary Audit

Purpose: decide whether support crates still deserve independent crate
boundaries after readability cleanup.

TDD RED:

- [ ] No production-code RED is required for audit-only work.
- [ ] If a crate merge is selected, add compile/import tests for the intended
      final public surface and confirm RED first.

Implementation jobs:

- [ ] Audit `tl-cache`.
- [ ] Audit `tl-fuzzy`.
- [ ] Audit `tl-llm`.
- [ ] Audit `tl-stream`.
- [ ] Audit `tl-replay`.
- [ ] Default to keeping crates unless there is a concrete simplification with
      low migration risk.
- [ ] Update `docs/concept/crates.md` with final decisions.

Testing and verification:

- [ ] `cargo tree -p tl-server`
- [ ] `cargo tree -p tl-engine`
- [ ] `cargo test --workspace --all-targets`

Phase done when:

- [ ] Every support crate has a documented keep/merge decision.
- [ ] No crate is merged without a separate RED/GREEN cycle.

## Final Acceptance Gates

The refactor is complete only when these pass or have a documented external
blocker:

- [ ] `git status --short`
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo test -p tl-server --no-default-features --all-targets`
- [ ] `cargo test -p tl-storage --features postgres --all-targets`
- [ ] `cargo bench -p tl-engine --bench check_pipeline`
- [ ] `cargo run -p tl-codegen -- --check`
- [ ] `pnpm test:backend` or `make backend-test`, if available
- [ ] If web files changed: `pnpm --filter web typecheck` and relevant web tests
- [ ] If diagrams changed: `pnpm docs:diagrams` or `make diagrams`
- [ ] Review `git diff` and confirm every changed line belongs to the refactor
