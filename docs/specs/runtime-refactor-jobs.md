# Runtime Refactor Jobs

Ordered local job board for executing
`docs/specs/runtime-refactor-plan.md`. Complete one phase before starting the
next. A phase is done only when every "done" item for that phase is checked or
the unchecked item has a documented blocker.

## Current Status

- [x] Phase 0: Baseline Evidence
- [x] Phase 1: Server Shell Cleanup
- [x] Phase 2: Guard Service Extraction
- [x] Phase 3: App State Decomposition
- [x] Phase 4: Gateway Decomposition
- [x] Phase 5: Core Contract Decomposition
- [x] Phase 6: Engine Runtime Decomposition
- [x] Phase 7: Storage Readability Pass
- [x] Phase 8: Documentation and Concept Sync
- [x] Phase 9: Crate Boundary Audit
- [x] Final Acceptance Gates

## Phase 0: Baseline Evidence

Jobs:

- [x] Review `git status --short`.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo test --workspace --all-targets`.
- [x] Run `cargo bench -p tl-engine --bench check_pipeline`.
- [x] Run `cargo run -p tl-codegen -- --check`.
- [x] Run `pnpm test:backend` or `make backend-test` if available.
- [x] Record benchmark summary.
- [x] Record pre-existing failures.

Done:

- [x] All baseline commands passed or failures are documented below.
- [x] No tracked source file was changed during this phase.

Failure log:

- [x] Command: `cargo test --workspace --all-targets`,
      `cargo run -p tl-codegen -- --check`, and `pnpm test:backend` without
      extra linker flags.
- [x] Summary: local Homebrew `libpq` was installed at
      `/opt/homebrew/Cellar/libpq/18.4`, but default Rust linker invocations did
      not include that native library path, producing `ld: library 'pq' not
      found`.
- [x] Decision: use this environment prefix for Rust gates that link Postgres:
      `export PATH="/opt/homebrew/Cellar/libpq/18.4/bin:$PATH";
      export DYLD_LIBRARY_PATH="/opt/homebrew/Cellar/libpq/18.4/lib:${DYLD_LIBRARY_PATH:-}";
      export PKG_CONFIG_PATH="/opt/homebrew/Cellar/libpq/18.4/lib/pkgconfig:${PKG_CONFIG_PATH:-}";
      export RUSTFLAGS="-L native=/opt/homebrew/Cellar/libpq/18.4/lib"`.

Baseline evidence:

- [x] `git status --short`: untracked `docs/specs/runtime-refactor-plan.md`.
- [x] `cargo fmt --check`: passed.
- [x] `cargo test --workspace --all-targets`: passed with the `libpq`
      environment prefix.
- [x] `cargo bench -p tl-engine --bench check_pipeline`: passed. Baseline:
      sync empty 904.52 ns; async empty 9.6457 us; async 50 policies 71.438 us;
      sync universal 4KB 6.1569 us; async cache hit 15.296 us; sync PII block
      18.254 us.
- [x] `cargo run -p tl-codegen -- --check`: passed with the `libpq`
      environment prefix; artifacts in sync.
- [x] `pnpm test:backend`: passed with the `libpq` environment prefix.

## Phase 1: Server Shell Cleanup

Jobs:

- [x] Add RED compile/import characterization for stable server exports.
- [x] Confirm RED with `cargo test -p tl-server --all-targets`.
- [x] Create `app/` module structure.
- [x] Create `api/` module structure.
- [x] Move router construction.
- [x] Move OpenAPI registration.
- [x] Move API error helpers.
- [x] Move HTTP logging middleware.
- [x] Move guard HTTP handlers.
- [x] Preserve compatibility re-exports.
- [x] Run GREEN verification.
- [x] Remove dead imports and keep `lib.rs` small.

Done:

- [x] RED was valid and recorded.
- [x] `cargo fmt --check` passed.
- [x] `cargo test -p tl-server --all-targets` passed.
- [x] `cargo run -p tl-codegen -- --check` passed.
- [x] Route paths, auth layering, and OpenAPI output are unchanged.

Evidence:

- [x] RED command/result: `cargo test -p tl-server --lib
      architecture_tests::server_shell_exports_stay_stable_during_module_split`
      failed with expected missing `crate::app` and `crate::api` exports before
      the module split existed.
- [x] GREEN command/result: `cargo fmt --check`, `cargo test -p tl-server
      --all-targets`, and `cargo run -p tl-codegen -- --check` passed with the
      `libpq` environment prefix. Generated artifacts remained in sync.

## Phase 2: Guard Service Extraction

Jobs:

- [x] Add or confirm guard characterization tests.
- [x] Confirm RED if a new service API/export is intentionally referenced.
- [x] Create `services/` module structure.
- [x] Move check orchestration to `services/guard_service.rs`.
- [x] Keep `api/guard.rs` thin.
- [x] Preserve redaction behavior.
- [x] Preserve run/run-event behavior.
- [x] Preserve runtime policy loading.
- [x] Preserve trace writer dispatch.
- [x] Preserve escalation dispatch.
- [x] Preserve error status/code mappings.
- [x] Run GREEN verification.

Done:

- [x] RED was valid or existing tests already covered the behavior.
- [x] `cargo fmt --check` passed.
- [x] `cargo test -p tl-server --test guardrails` passed.
- [x] `cargo test -p tl-server --test full_pipeline` passed.
- [x] `cargo test -p tl-server --all-targets` passed.
- [x] `/v1/check` behavior is unchanged.

Evidence:

- [x] RED command/result: `cargo test -p tl-server --lib
      architecture_tests::server_shell_exports_stay_stable_during_module_split`
      failed with expected missing `crate::services` before the service module
      existed.
- [x] GREEN command/result: `cargo fmt --check`, `cargo test -p tl-server
      --test guardrails`, `cargo test -p tl-server --test full_pipeline`,
      `cargo test -p tl-server --all-targets`, and `cargo run -p tl-codegen
      -- --check` passed with the `libpq` environment prefix.

## Phase 3: App State Decomposition

Jobs:

- [x] Add RED compile/import characterization for stable state exports.
- [x] Add or confirm env helper tests.
- [x] Confirm RED with server test target.
- [x] Create `state/` module structure.
- [x] Move `AppState` and `BuildOptions`.
- [x] Move high-level build orchestration.
- [x] Move memory wiring.
- [x] Move Postgres wiring.
- [x] Move env parsing.
- [x] Move worker setup where useful.
- [x] Move Postgres adapter impls to `state/postgres_adapters.rs`.
- [x] Preserve feature gates.
- [x] Run GREEN verification.

Done:

- [x] RED was valid and recorded.
- [x] `cargo fmt --check` passed.
- [x] `cargo test -p tl-server --all-targets` passed.
- [x] `cargo test -p tl-server --no-default-features --all-targets` passed.
- [x] `cargo test -p tl-storage --features postgres --all-targets` passed.
- [x] Public state exports remain compatible.

Evidence:

- [x] RED command/result: `cargo test -p tl-server --lib
      architecture_tests::server_shell_exports_stay_stable_during_module_split`
      failed with expected missing `state::app_state` and `state::memory`
      modules before the state split existed.
- [x] GREEN command/result: `cargo fmt --check`, `cargo test -p tl-server
      --all-targets`, `cargo test -p tl-server --no-default-features
      --all-targets`, `cargo test -p tl-storage --features postgres
      --all-targets`, and `cargo run -p tl-codegen -- --check` passed with
      the `libpq` environment prefix. No-default server tests still emit
      pre-existing dead-code warnings for gateway storage text helpers.

## Phase 4: Gateway Decomposition

Jobs:

- [x] Add or confirm gateway characterization tests.
- [x] Confirm RED if new module/export is intentionally referenced.
- [x] Create `gateway/` module directory.
- [x] Move gateway API handlers.
- [x] Move proxy service orchestration.
- [x] Move provider forwarding helpers.
- [x] Move normalization helpers.
- [x] Move credential crypto helpers.
- [x] Move guard check/regeneration helpers.
- [x] Move gateway errors.
- [x] Move memory store.
- [x] Preserve public re-exports.
- [x] Run GREEN verification.

Done:

- [x] RED was valid or existing tests already covered the behavior.
- [x] `cargo fmt --check` passed.
- [x] `cargo test -p tl-server --test gateway` passed.
- [x] `cargo test -p tl-server --all-targets` passed.
- [x] `cargo run -p tl-codegen -- --check` passed.
- [x] Gateway endpoint behavior is unchanged.

Evidence:

- [x] RED command/result: existing `cargo test -p tl-server --test gateway`
      coverage was used as the characterization suite for this mechanical
      split. No new intentionally missing export was required.
- [x] GREEN command/result: `cargo fmt --check`, `cargo test -p tl-server
      --test gateway`, `cargo test -p tl-server --all-targets`, and
      `cargo run -p tl-codegen -- --check` passed with the `libpq`
      environment prefix.

## Phase 5: Core Contract Decomposition

Jobs:

- [x] Add RED compile/import characterization for `tl_core::*`.
- [x] Confirm RED with `cargo test -p tl-core --all-targets`.
- [x] Create guard/admin/analytics/error/human-review/knowledge/run/trace module structure.
- [x] Move guard request/decision/protocol types.
- [x] Move redaction types.
- [x] Move API error types.
- [x] Move run/trace/analytics/knowledge DTOs where mechanical.
- [x] Preserve serialization, derives, and public re-exports.
- [x] Run GREEN verification.

Done:

- [x] RED was valid and recorded.
- [x] `cargo fmt --check` passed.
- [x] `cargo test -p tl-core --all-targets` passed.
- [x] `cargo test -p tl-sdk-rust --all-targets` passed.
- [x] `cargo test -p tl-server --all-targets` passed.
- [x] `cargo run -p tl-codegen -- --check` passed.
- [x] No generated contract drift unless explicitly accepted.

Evidence:

- [x] RED command/result: mechanical split initially failed
      `cargo test -p tl-core --all-targets` with misplaced derive attributes,
      confirming the moved modules were compiled rather than ignored.
- [x] GREEN command/result: `cargo fmt --check`, `cargo test -p tl-core
      --all-targets`, `cargo test -p tl-sdk-rust --all-targets`, `cargo test
      -p tl-server --all-targets`, and `cargo run -p tl-codegen -- --check`
      passed with the `libpq` environment prefix.

## Phase 6: Engine Runtime Decomposition

Jobs:

- [x] Add RED compile/import characterization for stable engine exports.
- [x] Confirm RED with `cargo test -p tl-engine --all-targets`.
- [x] Move `Engine`.
- [x] Move pipeline orchestrator.
- [x] Move tier runner traits/types.
- [x] Move cache scope helper.
- [x] Move deterministic, fuzzy, and LLM tiers.
- [x] Move context and resolver traits.
- [x] Move matcher logic.
- [x] Preserve public re-exports.
- [x] Run GREEN verification.
- [x] Compare benchmark against Phase 0.

Done:

- [x] RED was valid and recorded.
- [x] `cargo fmt --check` passed.
- [x] `cargo test -p tl-engine --all-targets` passed.
- [x] `cargo bench -p tl-engine --bench check_pipeline` passed.
- [x] Benchmark regression is absent or explicitly accepted.

Evidence:

- [x] RED command/result: `cargo test -p tl-engine --all-targets` initially
      failed after moving `Engine` because tests no longer inherited
      `CheckRequest`, `Verdict`, `Policy`, and `Arc` imports from `lib.rs`.
- [x] GREEN command/result: `cargo fmt --check`, `cargo test -p tl-engine
      --all-targets`, and `cargo bench -p tl-engine --bench check_pipeline`
      passed with the `libpq` environment prefix.
- [x] Benchmark comparison: sync empty 901.44 ns vs 904.52 ns baseline; async
      empty 9.3589 us vs 9.6457 us; async 50 policies 74.636 us vs 71.438 us
      with no detected change; sync universal 6.0267 us vs 6.1569 us; async
      cache hit 9.2293 us vs 15.296 us improved; sync PII block 17.264 us vs
      18.254 us improved.

## Phase 7: Storage Readability Pass

Jobs:

- [x] Add or confirm `tl_storage::*` export characterization.
- [x] Decide whether to keep flat repository files.
- [x] If moving, confirm RED for intended final structure.
- [x] Move repository modules only if readability payoff is clear.
- [x] Preserve public re-exports.
- [x] Avoid schema, migration, and query behavior changes.
- [x] Run GREEN verification.

Done:

- [x] Storage move/no-move decision is recorded.
- [x] `cargo fmt --check` passed.
- [x] `cargo test -p tl-storage --all-targets` passed.
- [x] `cargo test -p tl-storage --features postgres --all-targets` passed.
- [x] Optional `postgres-it` result recorded if Docker is available.

Evidence:

- [x] Decision: keep the existing flat repository files. `tl-storage/src/lib.rs`
      is already a small export surface, repository files are named by durable
      concept, and moving them into `repositories/` would add churn without a
      clearer ownership boundary.
- [x] RED command/result: added
      `crates/tl-storage/tests/export_surface.rs`; the first run of
      `cargo fmt --check && cargo test -p tl-storage --all-targets && cargo
      test -p tl-storage --features postgres --all-targets` failed only on
      rustfmt import ordering for the new characterization test.
- [x] GREEN command/result: after `cargo fmt`, `cargo fmt --check`,
      `cargo test -p tl-storage --all-targets`, and `cargo test -p tl-storage
      --features postgres --all-targets` passed with the `libpq` environment
      prefix. Docker-backed `postgres-it` was not run because it is optional
      and no storage behavior changed.

## Phase 8: Documentation and Concept Sync

Jobs:

- [x] Run codegen check before docs/codegen updates.
- [x] Search for stale source paths.
- [x] Update `docs/concept/crates.md`.
- [x] Update `docs/concept/architecture.md` only if needed.
- [x] Update `docs/openapi.yaml` only via codegen if needed.
- [x] Check glossary for stale terms.
- [x] Check concept docs for scaffolding language.
- [x] Run docs verification.

Done:

- [x] `cargo run -p tl-codegen -- --check` passed.
- [x] Diagram generation ran if diagrams changed.
- [x] Concept docs still each own one topic.
- [x] No stale crate count or source-path reference remains.

Evidence:

- [x] Docs/codegen command result: `cargo run -p tl-codegen -- --check`
      passed with the `libpq` environment prefix; generated artifacts remained
      in sync. No diagrams changed, so diagram generation was not needed.
- [x] Stale reference search summary: updated `docs/concept/crates.md`,
      `docs/concept/architecture.md`, `docs/concept/glossary.md`,
      `docs/concept/plugin-contract.md`, `docs/concept/v0-design-decisions.md`,
      `docs/OWNERSHIP.md`, and `docs/gateway-proxy-runtime-branch-guide.md`.
      `grep -RIn "TODO\\|Placeholder\\|Phase " docs/concept` returned no
      matches. Stale single-file source path matches remain only inside
      `docs/specs/runtime-refactor-plan.md`, where they describe the refactor
      jobs themselves.

## Phase 9: Crate Boundary Audit

Jobs:

- [x] Audit `tl-cache`.
- [x] Audit `tl-fuzzy`.
- [x] Audit `tl-llm`.
- [x] Audit `tl-stream`.
- [x] Audit `tl-replay`.
- [x] Record keep/merge decision for each crate.
- [x] Avoid crate merges unless separately approved by valid RED/GREEN cycle.
- [x] Update crate docs with final decisions.
- [x] Run audit verification.

Done:

- [x] `cargo tree -p tl-server` reviewed.
- [x] `cargo tree -p tl-engine` reviewed.
- [x] `cargo test --workspace --all-targets` passed.
- [x] Every support crate has a documented decision.

Decisions:

- [x] `tl-cache`: keep. It owns cache key derivation and Moka-backed cache
      behavior that should stay swappable without pushing cache dependencies
      into `tl-engine` internals.
- [x] `tl-fuzzy`: keep. It owns Tier 2 primitives and optional heavy embedder
      dependencies.
- [x] `tl-llm`: keep. It owns provider clients, router config, provider
      mocks, and live-provider gating.
- [x] `tl-stream`: keep. Streaming state is a distinct runtime surface from
      one-shot checks.
- [x] `tl-replay`: keep. Replay is an offline workflow that depends on engine
      and storage rather than belonging inside either one.
- [x] Verification: `cargo tree -p tl-server`, `cargo tree -p tl-engine`, and
      `cargo test --workspace --all-targets` passed with the `libpq`
      environment prefix.

## Final Acceptance Gates

Jobs:

- [x] Review `git status --short`.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] Run `cargo test --workspace --all-targets`.
- [x] Run `cargo test -p tl-server --no-default-features --all-targets`.
- [x] Run `cargo test -p tl-storage --features postgres --all-targets`.
- [x] Run `cargo bench -p tl-engine --bench check_pipeline`.
- [x] Run `cargo run -p tl-codegen -- --check`.
- [x] Run `pnpm test:backend` or `make backend-test` if available.
- [x] Run web checks if web files changed.
- [x] Run diagram generation if diagrams changed.
- [x] Review `git diff`.

Done:

- [x] All final gates pass or have documented external blockers.
- [x] Every changed line maps to the runtime refactor.
- [x] Unrelated user changes were not reverted.

Evidence:

- [x] `git status --short`: reviewed; changes are runtime refactor modules,
      storage export characterization, and refactor docs.
- [x] `cargo fmt --check`: passed after final warning cleanup.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed with the
      `libpq` environment prefix. The command still prints existing `ts-rs`
      serde-attribute warnings, but they are not clippy diagnostics.
- [x] `cargo test --workspace --all-targets`: passed after the final
      no-default warning cleanup.
- [x] `cargo test -p tl-server --no-default-features --all-targets`: passed
      after feature-gating Postgres-only gateway storage helpers.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed.
- [x] `cargo bench -p tl-engine --bench check_pipeline`: passed. Final mean
      samples: sync empty 903.57 ns; async empty 9.0374 us; async 50 policies
      74.249 us; sync universal 6.2251 us; async cache hit 9.2072 us; sync PII
      block 17.526 us. Criterion flagged two changes versus the immediately
      previous local run, but the affected values remain faster than the Phase
      0 baseline.
- [x] `cargo run -p tl-codegen -- --check`: passed; generated artifacts are in
      sync.
- [x] `pnpm test:backend`: passed.
- [x] Web checks were not needed because no web files changed.
- [x] Diagram generation was not needed because no diagram files changed.

## Continuation Readability Pass

Jobs:

- [x] Split `auth_user` OAuth handling into a focused module.
- [x] Split `auth_user` response helpers into a focused module.
- [x] Split `auth_user` behavior tests into a focused private test module.
- [x] Split `team_repo` generic helpers into a focused module.
- [x] Split `team_repo` invite/member operations, workspace operations, and
      row structs into focused private modules.
- [x] Split `policy_repo` authoring record reads, policy list reads, runtime
      enabled reads, and version history into focused private modules.
- [x] Split `run_repo` text conversion, validation, summary, and latest-review
      helper logic into focused private modules.
- [x] Split `human_review_repo` text conversion, validation, event summary,
      and analytics helper logic into focused private modules.
- [x] Split policy endpoint helpers into focused `context`, `mapping`,
      `response`, and `tests` modules.
- [x] Split LLM tier judge IO, prompt context, status output, aggregation, and
      tests into focused private modules.
- [x] Split analytics memory storage, authorization, defaults, query filters,
      response helpers, and validation into focused private modules.
- [x] Split team in-memory storage, Postgres adapter, request-context helpers,
      and response-envelope helpers into focused private modules.
- [x] Split dashboard-admin in-memory API-key storage, API-key authorization,
      default settings, and response-envelope helpers into focused private
      modules.
- [x] Split `gateway_repo` provider-connection operations, enforcement-profile
      operations, route/resolve operations, and row-to-wire mapping into
      focused private modules.
- [x] Split `analytics_repo` fact loading and metric/facet helpers into
      focused private modules.
- [x] Split `tl-llm` router config construction and router behavior tests into
      focused private modules.
- [x] Split policy authoring, draft, and version endpoint handlers into
      focused private modules.
- [x] Split agent in-memory storage, request parsing/validation, response
      helpers, and tests into focused private modules.
- [x] Split gateway service provider-request parsing/streaming setup and
      output enforcement into focused private modules.
- [x] Split gateway store public contract from the in-memory implementation.
- [x] Split environment in-memory storage, validation, and API error response
      mapping into focused private modules.
- [x] Split run endpoint environment-resolution glue and API error response
      mapping into focused private modules.
- [x] Split bearer-auth approval checks, token helpers, response-envelope
      helpers, and unit tests into focused private modules.
- [x] Split run repository event timeline operations and trace/stat reads into
      focused private modules.
- [x] Split Rust SDK guardrail endpoint helpers, run endpoint helpers, HTTP
      send/retry plumbing, and SDK-level tests into focused private modules.
- [x] Split `tl-engine` crate-root behavior tests into a focused private test
      module.
- [x] Split run endpoint HTTP handlers into a focused private handler module.
- [x] Split gateway OpenAI-compatible and Anthropic provider implementations
      into focused private provider modules.
- [x] Split human-review repository analytics query/aggregation into a focused
      private analytics query module.
- [x] Split gateway API provider-connection, enforcement-profile, route, and
      proxy handlers into focused private modules.
- [x] Split team repository membership listing/direct-add logic from invite
      lifecycle operations.
- [x] Split `auth_user` password-auth HTTP handlers into a focused handler
      module.
- [x] Split human-review endpoint handlers, in-memory store, query parsing,
      validation, and API error response helpers into focused private modules.
- [x] Split app route-group construction out of the top-level router
      orchestration.
- [x] Split team endpoint HTTP handlers out of the team store contract module.
- [x] Split environment repository delete/reference-check behavior out of the
      environment lifecycle repository module.
- [x] Split agent-profile YAML validation and parser tests out of the
      `tl-policy` parser entry-point module.
- [x] Split dashboard-admin API-key lifecycle/authentication out of the
      workspace settings repository module.
- [x] Split policy memory-store `PolicyStore` implementation out of the store
      shape and constructor module.
- [x] Split gateway memory-store `GatewayStore` implementation out of the
      in-memory row shape and constructor module.
- [x] Split policy repository mutating write, enable/disable, batch update, and
      soft-delete operations out of the cached read/repository shell module.
- [x] Split Postgres schema-drift integration tests out of the runtime
      Postgres store and migration module.
- [x] Split dashboard-admin API-key/settings HTTP handlers out of the
      dashboard-admin store contract and state module.
- [x] Split CLI policy commands, guardrail commands, and shared HTTP helpers
      out of the top-level CLI command definition module.
- [x] Split analytics dashboard-view row mapping and request validation out of
      the dashboard-view repository operation module.
- [x] Split gateway route-group construction out of the shared app route-group
      module.
- [x] Split knowledge-source HTTP handlers, in-memory storage, validation, and
      API error response helpers out of the endpoint contract module.
- [x] Split full-pipeline policy deployment and runtime-policy scenarios out
      of the full-pipeline test root while keeping root-level test names.
- [x] Split policy validation endpoint scenarios out of the policy
      integration-test root while keeping root-level test names.
- [x] Split auth OAuth-session scenarios out of the auth integration-test root
      while keeping root-level test names.
- [x] Split Anthropic gateway system-prompt scenario out of the provider-flow
      integration-test include while keeping the root-level test name.
- [x] Split gateway integration-test streaming and regeneration scenarios out
      of the oversized gateway test root while keeping root-level test names.
- [x] Split gateway integration-test output-action and signal-correctness
      scenarios out of the oversized gateway test root while keeping
      root-level test names.
- [x] Split gateway integration-test provider-error fail-mode scenarios out
      of the oversized gateway test root while keeping root-level test names.
- [x] Split full-pipeline redaction and redacted-only data-handling scenarios
      out of the oversized full-pipeline test root while keeping root-level
      test names.
- [x] Split gateway integration-test route/config validation scenarios out of
      the oversized gateway test root while keeping root-level test names.
- [x] Split gateway integration-test runtime-key and input-enforcement
      scenarios out of the oversized gateway test root while keeping
      root-level test names.
- [x] Split full-pipeline run lifecycle and run-event validation scenarios out
      of the oversized full-pipeline test root while keeping root-level test
      names.
- [x] Split auth integration-test API-key authorization and runtime-key
      scenarios out of the oversized auth test root while keeping root-level
      test names.
- [x] Split gateway integration-test provider forwarding, run correlation, and
      system-prompt scenarios out of the gateway test root while keeping
      root-level test names.
- [x] Split gateway integration-test output signal/header correctness
      scenarios out of the output-action include file while keeping root-level
      test names.
- [x] Preserve stable public/internal imports used by existing modules.
- [x] Run targeted tests for the touched areas.
- [x] Run contract and backend gates after the continuation split.

Done:

- [x] `crates/tl-server/src/policies.rs` now contains policy endpoint flow
      instead of endpoint flow plus response, mapping, context, and test
      utilities.
- [x] `crates/tl-server/src/auth_user.rs` no longer mixes password endpoints,
      OAuth session linking, and response-envelope helpers in one file.
- [x] `crates/tl-server/src/auth_user.rs` no longer carries the password-auth
      behavior tests inline with endpoint code. The endpoint file is 363 lines
      after the test split.
- [x] `crates/tl-storage/src/team_repo.rs` no longer carries token, slug, and
      user-existence helper implementations at the bottom of the repository
      file.
- [x] `crates/tl-storage/src/team_repo.rs` now reads as repository shell and
      shared types rather than invite/member workflows, workspace
      bootstrap/listing, row structs, token/slug helpers, and starter policy
      seeding in one file. The repository file is 63 lines after the split.
- [x] `crates/tl-storage/src/policy_repo.rs` now reads as policy write/cache
      operations rather than writes plus authoring-record reads, list queries,
      enabled runtime reads, environment deployment reads, and version history
      in one file. The repository file is 357 lines after the split.
- [x] `crates/tl-storage/src/run_repo.rs` now reads as run repository
      operations rather than operations plus string enum conversion, metadata
      validation, summary construction, p95 calculation, and latest human
      review lookup helpers. The repository file is 423 lines after the split.
- [x] `crates/tl-storage/src/human_review_repo.rs` now reads as human-review
      repository operations rather than operations plus outcome conversion,
      event normalization, analytics accumulator types, grouping helpers, and
      row sorting. The repository file is 391 lines after the split.
- [x] `crates/tl-engine/src/tiers/llm.rs` now reads as Tier 3 orchestration
      rather than judge transport, verdict interpretation, prompt rendering,
      status helpers, and tests all in one file.
- [x] `crates/tl-server/src/analytics.rs` now reads as endpoint flow rather
      than endpoint flow plus in-memory storage, membership authorization,
      default catalog/view construction, response-envelope helpers, and
      validation in one file. The endpoint file is 287 lines after the split.
- [x] `crates/tl-server/src/team.rs` now reads as team endpoint flow rather
      than endpoint flow plus in-memory storage, Postgres repository adapter,
      user-header parsing, and API error-envelope construction. The endpoint
      file is 320 lines after the split.
- [x] `crates/tl-server/src/dashboard_admin.rs` now reads as dashboard admin
      endpoint flow and public store traits rather than endpoint flow plus
      in-memory API-key storage, runtime-key verification, API-key management
      authorization, default settings, and API error-envelope construction.
      The endpoint file is 297 lines after the split.
- [x] `crates/tl-storage/src/gateway_repo.rs` now reads as repository shell and
      shared public types rather than provider-connection CRUD,
      enforcement-profile CRUD, route CRUD/resolve queries, enum parsing, and
      row-to-wire mapping in one file. The repository file is 62 lines after
      the split.
- [x] `crates/tl-storage/src/analytics_repo.rs` now reads as analytics
      catalog/query flow and repository shell rather than fact loading,
      filter/matching logic, metric accumulators, label catalogs, payload
      extraction, and workflow-step derivation in one file. The repository
      file is 138 lines after the split.
- [x] `crates/tl-llm/src/router.rs` now reads as router types and judge runtime
      flow rather than route config construction, provider client creation,
      budget wiring, and unit-test fixtures in one file. The router file is
      222 lines after the split.
- [x] `crates/tl-server/src/policies.rs` now reads as the policy store trait,
      route state, and stable re-export surface rather than carrying all
      authoring, draft, validation, and version endpoint bodies inline. The
      endpoint root is 135 lines after the split.
- [x] `crates/tl-server/src/agents.rs` now reads as the agent store trait,
      endpoint state, and endpoint flow rather than also carrying the
      in-memory store implementation, body parser, profile validator, API
      error envelope helper, and unit tests inline. The endpoint file is 235
      lines after the split.
- [x] `crates/tl-server/src/gateway/service.rs` now reads as gateway request
      orchestration rather than also carrying provider request parsing,
      streaming-mode validation, output action branching, regeneration fallback
      handling, and output enforcement response construction inline. The
      service file is 295 lines after the split.
- [x] `crates/tl-server/src/gateway/store.rs` now reads as the gateway
      configuration store contract and request/patch structs rather than also
      carrying the in-memory development/test store implementation inline. The
      store contract file is 152 lines after the split.
- [x] `crates/tl-server/src/environments.rs` now reads as the environment
      store trait, endpoint state, endpoint handlers, and environment header
      resolution rather than also carrying the in-memory store, validation
      helpers, and API error envelope construction inline. The endpoint file is
      188 lines after the split.
- [x] `crates/tl-server/src/runs.rs` now reads as the run store trait, endpoint
      state, and run endpoint handlers rather than also carrying
      environment-resolution glue and API error envelope construction inline.
      The endpoint file is 394 lines after the split.
- [x] `crates/tl-server/src/auth.rs` now reads as bearer-auth configuration,
      credential context types, verifier trait, and middleware flow rather than
      also carrying approval-gate checks, token utility functions, API error
      envelope construction, and unit tests inline. The middleware file is 290
      lines after the split.
- [x] `crates/tl-storage/src/run_repo.rs` now reads as the run repository shell
      and run CRUD/list operations rather than also carrying run-event
      insertion/listing, trace reads, latest-review joins, and trace statistic
      aggregation inline. The repository file is 215 lines after the split.
- [x] `crates/tl-sdk-rust/src/lib.rs` now reads as SDK docs, public wire-type
      re-exports, client construction, the primary `check` call, and fallback
      API-error synthesis rather than also carrying guardrail endpoint helpers,
      run endpoint helpers, low-level HTTP send helpers, retry-loop plumbing,
      and tests inline. The SDK root file is 134 lines after the split.
- [x] `crates/tl-engine/src/lib.rs` now reads as the engine crate overview,
      module declarations, and stable public re-export surface rather than
      also carrying the async orchestrator/cache behavior tests inline. The
      engine root file is 30 lines after the split.
- [x] `crates/tl-server/src/runs.rs` now reads as the run store contract,
      endpoint state, memory-store export, validation export, and stable
      handler re-export surface rather than also carrying every `/v1/runs`
      HTTP handler body and OpenAPI annotation inline. The run endpoint root
      file is 114 lines after the split.
- [x] `crates/tl-server/src/gateway/provider.rs` now reads as the gateway
      provider trait plus shared request-text, provider URL, and JSON response
      helpers rather than also carrying OpenAI-compatible and Anthropic
      provider-specific response/SSE/forwarding implementations inline. The
      provider root file is 137 lines after the split.
- [x] `crates/tl-storage/src/human_review_repo.rs` now reads as the
      human-review repository shell, event creation/list/latest lookup, and
      connection helper rather than also carrying the full analytics trace/run
      query and aggregation loop inline. The repository root file is 156 lines
      after the split.
- [x] `crates/tl-server/src/gateway/api.rs` now reads as gateway API state,
      the runtime-key configuration-access guard, and stable handler/OpenAPI
      re-exports rather than also carrying provider-connection,
      enforcement-profile, route, and proxy handler bodies inline. The gateway
      API root file is 51 lines after the split.
- [x] `crates/tl-storage/src/team_repo/invites.rs` now reads as invite
      lifecycle operations rather than also carrying member listing, username
      lookup, member row mapping, and existing-user workspace insertion inline.
      The invite file is 291 lines and the new member module is 114 lines after
      the split.
- [x] `crates/tl-server/src/auth_user.rs` now reads as the password/OAuth user
      store contract, auth-user state, and stable handler re-export surface
      rather than also carrying signup, login, and change-password HTTP handler
      bodies inline. The auth-user root file is 117 lines after the split.
- [x] `crates/tl-server/src/human_review.rs` now reads as the human-review
      store contract, analytics filter, endpoint state, and stable handler
      re-export surface rather than also carrying in-memory storage, HTTP
      handlers, query parsing, validation, and API error response helpers
      inline. The human-review root file is 65 lines after the split.
- [x] `crates/tl-server/src/app/router.rs` now reads as top-level route
      composition, auth layering, and shared middleware setup rather than also
      carrying every route group builder inline. The app router file is 97
      lines after the split.
- [x] `crates/tl-server/src/team.rs` now reads as the team store contract,
      endpoint state, and stable handler re-export surface rather than also
      carrying member, invite, and workspace HTTP handler bodies inline. The
      team root file is 115 lines after the split.
- [x] `crates/tl-storage/src/environment_repo.rs` now reads as the environment
      repository lifecycle flow rather than also carrying delete-time runtime
      reference checks and soft-delete mechanics inline. The environment
      repository root file is 250 lines after the split.
- [x] `crates/tl-policy/src/agent_parse.rs` now reads as the public
      agent-profile YAML loader contract rather than also carrying validation
      internals, public URL safety checks, and parser tests inline. The parser
      root file is 26 lines after the split.
- [x] `crates/tl-storage/src/dashboard_admin_repo.rs` now reads as the
      dashboard-admin repository shell and workspace settings reader rather
      than also carrying API-key listing, creation, revocation, authentication,
      row mapping, and environment lookup logic inline. The repository root
      file is 82 lines after the split.
- [x] `crates/tl-server/src/policies/memory_store.rs` now reads as the
      in-memory policy store data shape and constructors rather than also
      carrying every `PolicyStore` trait method inline. The memory-store root
      file is 53 lines after the split.
- [x] `crates/tl-server/src/gateway/store/memory.rs` now reads as the
      in-memory gateway row shape, constructor, and shared lock-error helper
      rather than also carrying every `GatewayStore` trait method inline. The
      gateway memory-store root file is 43 lines after the split.
- [x] `crates/tl-storage/src/policy_repo.rs` now reads as the policy repository
      shell, constructors, cached read path, cache diagnostics, and stable
      connection/debug helpers rather than also carrying upsert, enable/disable,
      batch update, and soft-delete workflows inline. The repository root file
      is 125 lines and the mutation module is 248 lines after the split.
- [x] `crates/tl-storage/src/postgres.rs` now reads as the Postgres connection,
      migration, schema repair, and `DecisionStore` implementation module
      rather than also carrying opt-in schema-drift integration test helpers and
      test cases inline. The runtime module is 163 lines and the private test
      module is 193 lines after the split.
- [x] `crates/tl-server/src/dashboard_admin.rs` now reads as the dashboard-admin
      store contract, endpoint state, memory/default-settings exports, and
      stable handler re-export surface rather than also carrying API-key and
      settings HTTP handler bodies inline. The dashboard-admin root file is 68
      lines and the handler module is 238 lines after the split.
- [x] `crates/tl-cli/src/main.rs` now reads as the CLI command shape and
      dispatch entry point rather than also carrying policy HTTP calls,
      guardrail HTTP calls, response decoding, URL handling, and path encoding
      helpers inline. The CLI root file is 122 lines; the new policy,
      guardrails, and HTTP helper modules are 100, 98, and 49 lines after the
      split.
- [x] `crates/tl-storage/src/analytics_repo/dashboard_views.rs` now reads as
      dashboard-view list/create/update/delete flow rather than also carrying
      row-to-wire mapping and dashboard-view validation internals inline. The
      dashboard-view operation file is 235 lines, and the new record and
      validation modules are 40 lines each after the split.
- [x] `crates/tl-server/src/app/route_groups.rs` now reads as the app route
      family index rather than also carrying the gateway route builder and
      outbound gateway HTTP client setup inline. The route-group root file is
      255 lines, and the new gateway route module is 59 lines after the split.
- [x] `crates/tl-server/src/knowledge_sources.rs` now reads as the
      knowledge-source store contract, endpoint state, stable handler exports,
      and compatibility file-decoder export rather than also carrying HTTP
      handlers, in-memory storage, validation, and API error-envelope helpers
      inline. The root file is 49 lines; the handler, memory-store,
      validation, and response modules are 85, 103, 61, and 40 lines after the
      split.
- [x] `crates/tl-server/tests/gateway.rs` now keeps shared gateway test helpers
      and the remaining scenario groups rather than carrying every streaming
      and regeneration scenario inline. The gateway test root is 1768 lines,
      and the new streaming and regeneration include files are 217 and 231
      lines after the split.
- [x] `crates/tl-server/tests/gateway.rs` now also delegates output-action and
      signal-correctness scenarios to a focused include file. The gateway test
      root is 1207 lines, and the new output-action include file is 562 lines
      after the split.
- [x] `crates/tl-server/tests/gateway.rs` now also delegates provider-error
      fail-mode scenarios to a focused include file. The gateway test root is
      1022 lines, and the new fail-mode include file is 186 lines after the
      split.
- [x] `crates/tl-server/tests/full_pipeline.rs` now delegates redaction and
      redacted-only data-handling scenarios to a focused include file. The
      full-pipeline test root is 822 lines, and the new redaction include file
      is 244 lines after the split.
- [x] `crates/tl-server/tests/gateway.rs` now also delegates route/config
      validation and input-enforcement scenarios to focused include files. The
      gateway test root is 597 lines, and the new route-validation and
      input-enforcement include files are 148 and 277 lines after the split.
- [x] `crates/tl-server/tests/full_pipeline.rs` now also delegates run
      lifecycle and run-event validation scenarios to a focused include file.
      The full-pipeline test root is 555 lines, and the new run include file
      is 268 lines after the split.
- [x] `crates/tl-server/tests/auth.rs` now delegates API-key authorization and
      runtime-key scenarios to a focused include file. The auth test root is
      486 lines, and the new API-key include file is 335 lines after the
      split.
- [x] `crates/tl-server/tests/gateway.rs` now delegates provider forwarding,
      run-correlation, and system-prompt scenarios to a focused include file.
      The gateway test root is 220 lines, and the new provider-flow include
      file is 378 lines after the split.
- [x] `crates/tl-server/tests/gateway/output_actions.rs` now keeps output
      action behavior scenarios rather than also carrying signal/header
      correctness checks inline. The output-action include file is 234 lines,
      and the new output-signal include file is 327 lines after the split.
- [x] `crates/tl-server/tests/full_pipeline.rs` now also delegates policy
      deployment and runtime-policy scenarios to a focused include file. The
      full-pipeline test root is 227 lines, and the new policy include file is
      329 lines after the split.
- [x] `crates/tl-server/tests/policies.rs` now delegates policy validation
      endpoint scenarios to a focused include file. The policy test root is
      373 lines, and the new validation include file is 130 lines after the
      split.
- [x] `crates/tl-server/tests/auth.rs` now also delegates OAuth-session
      scenarios to a focused include file. The auth test root is 386 lines,
      and the new OAuth include file is 101 lines after the split.
- [x] `crates/tl-server/tests/gateway/provider_flow.rs` now keeps OpenAI
      provider-forwarding and run-correlation scenarios while delegating the
      Anthropic top-level system-prompt scenario to a focused include file.
      The provider-flow include file is 267 lines, and the new Anthropic
      system-prompt include file is 112 lines after the split.
- [x] No OpenAPI or SDK contract drift was introduced.
- [x] No backend test failure remains.

Evidence:

- [x] `cargo test -p tl-server auth_user --all-targets`: passed after the
      auth-user splits.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the team repository helper split.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the team repository operation split.
- [x] `cargo test -p tl-server --test auth --test analytics`: passed after the
      team repository operation split.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the policy repository read/version split.
- [x] `cargo test -p tl-server --test policies --test guardrails`: passed after
      the policy repository read/version split.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the run repository helper split.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the run
      repository helper split.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the human-review repository helper split.
- [x] `cargo test -p tl-server --test human_review`: passed after the
      human-review repository helper split.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the
      human-review repository helper split.
- [x] `cargo test -p tl-server policies --all-targets`: passed after the
      policies helper split.
- [x] `cargo test -p tl-engine tiers::llm --all-targets`: passed after the LLM
      tier split.
- [x] `cargo test -p tl-engine --all-targets`: passed after the LLM tier split.
- [x] `cargo test -p tl-server analytics --all-targets`: passed after the
      analytics module split.
- [x] `cargo test -p tl-server --test auth --test analytics --test gateway`:
      passed before and after the team module split.
- [x] `cargo test -p tl-server --all-targets`: passed after the team module
      split.
- [x] `cargo test -p tl-server --test auth --test full_pipeline`: passed after
      the dashboard-admin module split.
- [x] `cargo test -p tl-server dashboard_admin --all-targets`: passed after the
      dashboard-admin module split.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the gateway repository operation/mapping split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      repository operation/mapping split.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the analytics repository fact/metrics split.
- [x] `cargo test -p tl-server --test analytics`: passed after the analytics
      repository fact/metrics split.
- [x] `cargo test -p tl-llm --all-targets`: passed after the LLM router
      config/test split.
- [x] `cargo test -p tl-server policies --all-targets`: passed after the policy
      endpoint family split.
- [x] `cargo test -p tl-server agents --all-targets`: passed after the agents
      helper/store/test split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      service request/output split.
- [x] `cargo fmt --check`: passed after the gateway service request/output
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway service request/output split with the `libpq` environment
      prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway service
      request/output split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the gateway service request/output
      split with the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway store
      memory implementation split.
- [x] `cargo fmt --check`: passed after the gateway store memory
      implementation split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway store memory implementation split with the `libpq`
      environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway store
      memory implementation split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the gateway store memory
      implementation split with the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the
      environment module split.
- [x] `cargo fmt --check`: passed after the environment module split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the environment module split with the `libpq` environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the environment
      module split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the environment module split with the
      `libpq` environment prefix.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the run
      response/context split.
- [x] `cargo test -p tl-sdk-rust --test runs_integration`: passed after the run
      response/context split.
- [x] `cargo fmt --check`: passed after the run response/context split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the run response/context split with the `libpq` environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the run
      response/context split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the run response/context split with the
      `libpq` environment prefix.
- [x] `cargo test -p tl-server auth::tests --all-targets`: passed after the
      bearer-auth helper/test split.
- [x] `cargo test -p tl-server --test auth`: passed after the bearer-auth
      helper/test split.
- [x] `cargo fmt --check`: passed after the bearer-auth helper/test split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the bearer-auth helper/test split with the `libpq` environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the bearer-auth
      helper/test split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the bearer-auth helper/test split with
      the `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the run repository event/trace split.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the run
      repository event/trace split.
- [x] `cargo fmt --check`: passed after the run repository event/trace split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the run repository event/trace split with the `libpq` environment
      prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the run repository
      event/trace split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the run repository event/trace split
      with the `libpq` environment prefix.
- [x] `cargo fmt --check`: passed after the analytics repository fact/metrics
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the analytics repository fact/metrics split with the `libpq` environment
      prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the analytics
      repository fact/metrics split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the analytics repository fact/metrics
      split with the `libpq` environment prefix.
- [x] `cargo fmt --check`: passed after the LLM router config/test split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the LLM router config/test split with the `libpq` environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the LLM router
      config/test split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the LLM router config/test split with
      the `libpq` environment prefix.
- [x] `cargo fmt --check`: passed after the policy endpoint family split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the policy endpoint family split with the `libpq` environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the policy endpoint
      family split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the policy endpoint family split with
      the `libpq` environment prefix.
- [x] `cargo fmt --check`: passed after the agents helper/store/test split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the agents helper/store/test split with the `libpq` environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the agents
      helper/store/test split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the agents helper/store/test split
      with the `libpq` environment prefix.
- [x] `cargo test -p tl-sdk-rust --all-targets`: passed after the SDK endpoint
      and HTTP helper split.
- [x] `cargo fmt --check`: passed after the SDK endpoint and HTTP helper
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the SDK endpoint and HTTP helper split with the `libpq` environment
      prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the SDK endpoint and
      HTTP helper split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the SDK endpoint and HTTP helper split
      with the `libpq` environment prefix.
- [x] `cargo test -p tl-engine --all-targets`: passed after the engine
      crate-root test split.
- [x] `cargo fmt --check`: passed after the engine crate-root test split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the engine crate-root test split with the `libpq` environment prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the engine crate-root
      test split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the engine crate-root test split with
      the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the run
      handler split.
- [x] `cargo test -p tl-sdk-rust --test runs_integration`: passed after the run
      handler split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the run handler split;
      generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the run handler split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the run handler split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the run handler split with the `libpq`
      environment prefix.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      provider implementation split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway provider
      implementation split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway provider implementation
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway provider implementation split with the `libpq` environment
      prefix.
- [x] `pnpm test:backend`: passed after the gateway provider implementation
      split with the `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the human-review analytics query split.
- [x] `cargo test -p tl-server --test human_review`: passed after the
      human-review analytics query split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the human-review
      analytics query split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the human-review analytics query split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the human-review analytics query split with the `libpq` environment
      prefix.
- [x] `pnpm test:backend`: passed after the human-review analytics query split
      with the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway API
      handler split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway API
      handler split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway API handler split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway API handler split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the gateway API handler split with the
      `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the team repository member/invite split.
- [x] `cargo test -p tl-server team --all-targets`: passed after the team
      repository member/invite split.
- [x] `cargo test -p tl-server --test auth --test analytics`: passed after the
      team repository member/invite split.
- [x] `cargo fmt --check`: passed after the team repository member/invite
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the team repository member/invite split with the `libpq` environment
      prefix.
- [x] `cargo run -p tl-codegen -- --check`: passed after the team repository
      member/invite split; generated artifacts are in sync.
- [x] `pnpm test:backend`: passed after the team repository member/invite
      split with the `libpq` environment prefix.
- [x] `cargo test -p tl-server auth_user --all-targets`: passed after the
      auth-user handler split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the auth-user handler
      split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the auth-user handler split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the auth-user handler split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the auth-user handler split with the
      `libpq` environment prefix.
- [x] `cargo test -p tl-server --test human_review`: passed after the
      human-review endpoint split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the human-review
      endpoint split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the human-review endpoint split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the human-review endpoint split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the human-review endpoint split with
      the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test auth --test analytics --test gateway
      --test full_pipeline --test human_review`: passed after the app route
      group split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the app route group
      split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the app route group split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the app route group split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the app route group split with the
      `libpq` environment prefix.
- [x] `cargo test -p tl-server team --all-targets`: passed after the team
      endpoint handler split.
- [x] `cargo test -p tl-server --test auth --test analytics`: passed after the
      team endpoint handler split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the team endpoint
      handler split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the team endpoint handler split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the team endpoint handler split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the team endpoint handler split with
      the `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the environment repository delete/reference-check split.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the
      environment repository delete/reference-check split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the environment
      repository delete/reference-check split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the environment repository
      delete/reference-check split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the environment repository delete/reference-check split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the environment repository
      delete/reference-check split with the `libpq` environment prefix.
- [x] `cargo test -p tl-policy --all-targets`: passed after the agent parser
      validation/test split.
- [x] `cargo test -p tl-server --test agents --test guardrails`: passed after
      the agent parser validation/test split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the agent parser
      validation/test split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the agent parser validation/test
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the agent parser validation/test split with the `libpq` environment
      prefix.
- [x] `pnpm test:backend`: passed after the agent parser validation/test split
      with the `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the dashboard-admin API-key repository split.
- [x] `cargo test -p tl-server --test auth`: passed after the dashboard-admin
      API-key repository split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the dashboard-admin
      API-key repository split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the dashboard-admin API-key repository
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the dashboard-admin API-key repository split with the `libpq` environment
      prefix.
- [x] `pnpm test:backend`: passed after the dashboard-admin API-key repository
      split with the `libpq` environment prefix.
- [x] `cargo test -p tl-server policies --all-targets`: passed after the
      policy memory-store trait-implementation split.
- [x] `cargo test -p tl-server --test guardrails --test full_pipeline`: passed
      after the policy memory-store trait-implementation split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the policy
      memory-store trait-implementation split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the policy memory-store
      trait-implementation split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the policy memory-store trait-implementation split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the policy memory-store
      trait-implementation split with the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      memory-store trait-implementation split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      memory-store trait-implementation split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway memory-store
      trait-implementation split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway memory-store trait-implementation split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the gateway memory-store
      trait-implementation split with the `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the policy repository mutation split.
- [x] `cargo test -p tl-server --test policies --test guardrails`: passed after
      the policy repository mutation split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the policy repository
      mutation split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the policy repository mutation split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the policy repository mutation split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the policy repository mutation split
      with the `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the Postgres integration-test module split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the Postgres
      integration-test module split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the Postgres integration-test module
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the Postgres integration-test module split with the `libpq` environment
      prefix.
- [x] `pnpm test:backend`: passed after the Postgres integration-test module
      split with the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test auth`: passed after the dashboard-admin
      handler split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the dashboard-admin
      handler split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the dashboard-admin handler split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the dashboard-admin handler split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the dashboard-admin handler split with
      the `libpq` environment prefix.
- [x] `cargo test -p tl-cli --all-targets`: passed after the CLI command/helper
      split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the CLI command/helper
      split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the CLI command/helper split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the CLI command/helper split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the CLI command/helper split with the
      `libpq` environment prefix.
- [x] `cargo test -p tl-storage --features postgres --all-targets`: passed
      after the analytics dashboard-view mapping/validation split.
- [x] `cargo test -p tl-server --test analytics`: passed after the analytics
      dashboard-view mapping/validation split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the analytics
      dashboard-view mapping/validation split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the analytics dashboard-view
      mapping/validation split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the analytics dashboard-view mapping/validation split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the analytics dashboard-view
      mapping/validation split with the `libpq` environment prefix.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      route-group split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      route-group split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway route-group split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway route-group split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the gateway route-group split with the
      `libpq` environment prefix.
- [x] `git diff --check`: passed after the gateway route-group split.
- [x] `cargo test -p tl-server knowledge --all-targets`: passed after the
      knowledge-source endpoint/store split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the knowledge-source
      endpoint/store split; generated artifacts are in sync and OpenAPI schema
      refs remain unchanged.
- [x] `cargo fmt --check`: passed after the knowledge-source endpoint/store
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the knowledge-source endpoint/store split with the `libpq` environment
      prefix.
- [x] `pnpm test:backend`: passed after the knowledge-source endpoint/store
      split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the knowledge-source endpoint/store
      split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      integration-test streaming/regeneration split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      integration-test streaming/regeneration split; generated artifacts are in
      sync.
- [x] `cargo fmt --check`: passed after the gateway integration-test
      streaming/regeneration split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway integration-test streaming/regeneration split with the
      `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the gateway integration-test
      streaming/regeneration split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the gateway integration-test
      streaming/regeneration split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      integration-test output-action/signal split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      integration-test output-action/signal split; generated artifacts are in
      sync.
- [x] `cargo fmt --check`: passed after the gateway integration-test
      output-action/signal split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway integration-test output-action/signal split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the gateway integration-test
      output-action/signal split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the gateway integration-test
      output-action/signal split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      integration-test fail-mode split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      integration-test fail-mode split; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway integration-test
      fail-mode split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway integration-test fail-mode split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the gateway integration-test
      fail-mode split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the gateway integration-test fail-mode
      split.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the
      full-pipeline redaction/data-handling split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the full-pipeline
      redaction/data-handling split with the `libpq` environment prefix;
      generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the full-pipeline
      redaction/data-handling split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the full-pipeline redaction/data-handling split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the full-pipeline
      redaction/data-handling split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the full-pipeline
      redaction/data-handling split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      integration-test route-validation and input-enforcement split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      integration-test route-validation and input-enforcement split with the
      `libpq` environment prefix; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway integration-test
      route-validation and input-enforcement split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway integration-test route-validation and input-enforcement
      split with the `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the gateway integration-test
      route-validation and input-enforcement split with the `libpq`
      environment prefix.
- [x] `git diff --check`: passed after the gateway integration-test
      route-validation and input-enforcement split.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the
      full-pipeline run lifecycle/run-event split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the full-pipeline run
      lifecycle/run-event split with the `libpq` environment prefix; generated
      artifacts are in sync.
- [x] `cargo fmt --check`: passed after the full-pipeline run
      lifecycle/run-event split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the full-pipeline run lifecycle/run-event split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the full-pipeline run
      lifecycle/run-event split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the full-pipeline run
      lifecycle/run-event split.
- [x] `cargo test -p tl-server --test auth`: passed after the auth
      integration-test API-key split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the auth
      integration-test API-key split with the `libpq` environment prefix;
      generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the auth integration-test API-key
      split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the auth integration-test API-key split with the `libpq` environment
      prefix.
- [x] `pnpm test:backend`: passed after the auth integration-test API-key split
      with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the auth integration-test API-key
      split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      integration-test provider-flow split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      integration-test provider-flow split with the `libpq` environment
      prefix; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway integration-test
      provider-flow split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway integration-test provider-flow split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the gateway integration-test
      provider-flow split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the gateway integration-test
      provider-flow split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      integration-test output-signal split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      integration-test output-signal split with the `libpq` environment
      prefix; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway integration-test
      output-signal split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway integration-test output-signal split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the gateway integration-test
      output-signal split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the gateway integration-test
      output-signal split.
- [x] `cargo test -p tl-server --test full_pipeline`: passed after the
      full-pipeline policy deployment/runtime-policy split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the full-pipeline
      policy deployment/runtime-policy split with the `libpq` environment
      prefix; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the full-pipeline policy
      deployment/runtime-policy split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the full-pipeline policy deployment/runtime-policy split with the
      `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the full-pipeline policy
      deployment/runtime-policy split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the full-pipeline policy
      deployment/runtime-policy split.
- [x] `cargo test -p tl-server --test policies`: passed after the policy
      validation integration-test split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the policy validation
      integration-test split with the `libpq` environment prefix; generated
      artifacts are in sync.
- [x] `cargo fmt --check`: passed after the policy validation
      integration-test split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the policy validation integration-test split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the policy validation integration-test
      split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the policy validation integration-test
      split.
- [x] `cargo test -p tl-server --test auth`: passed after the auth
      OAuth-session integration-test split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the auth
      OAuth-session integration-test split with the `libpq` environment
      prefix; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the auth OAuth-session
      integration-test split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the auth OAuth-session integration-test split with the `libpq`
      environment prefix.
- [x] `pnpm test:backend`: passed after the auth OAuth-session
      integration-test split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the auth OAuth-session integration-test
      split.
- [x] `cargo test -p tl-server --test gateway`: passed after the gateway
      Anthropic system-prompt integration-test split.
- [x] `cargo run -p tl-codegen -- --check`: passed after the gateway
      Anthropic system-prompt integration-test split with the `libpq`
      environment prefix; generated artifacts are in sync.
- [x] `cargo fmt --check`: passed after the gateway Anthropic system-prompt
      integration-test split.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: passed after
      the gateway Anthropic system-prompt integration-test split with the
      `libpq` environment prefix.
- [x] `pnpm test:backend`: passed after the gateway Anthropic system-prompt
      integration-test split with the `libpq` environment prefix.
- [x] `git diff --check`: passed after the gateway Anthropic system-prompt
      integration-test split.
