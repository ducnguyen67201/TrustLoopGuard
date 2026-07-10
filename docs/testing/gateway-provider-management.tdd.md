# Gateway Provider Management TDD Evidence

Source plan: journeys were derived from the provider edit and hard-delete request.

## User journeys

| Journey | Implemented guarantee |
|---|---|
| As a workspace operator, I can edit a provider without exposing or re-entering its stored credential. | The dashboard patches editable metadata and only sends `provider_api_key` when a replacement is entered. |
| As a workspace operator, I can permanently delete an unused provider. | Rust deletes the provider row and encrypted credential; the same stable id can be created again afterward. |
| As a workspace operator, I cannot silently break a live route by deleting its provider. | Rust returns HTTP 409 while any Gateway route references the provider. |
| As a runtime-key caller, I cannot manage provider configuration. | Runtime keys receive HTTP 403 for provider creation and deletion. |

## RED/GREEN evidence

| Slice | RED evidence | GREEN evidence |
|---|---|---|
| Rust hard-delete endpoint | `cargo test -p tl-server --test gateway gateway_` ran the new tests and returned HTTP 405 instead of 204/409. | `cargo test -p tl-server --test gateway` passed all 21 Gateway tests. |
| Same-origin DELETE proxy | The focused Vitest run failed with `deleteRustResource is not a function`. | The focused Vitest run passed all 6 proxy-helper tests. |
| Provider edit/delete UI | The focused component tests could not find accessible Edit or Delete controls. | The focused Vitest run passed all 4 Gateway component tests, including PATCH without an existing secret and confirmed DELETE. |

## Test specification

| # | What is guaranteed | Test file or command | Test type | Result |
|---|---|---|---|---|
| 1 | An unreferenced provider is physically removed and its id can be reused. | `crates/tl-server/tests/gateway/route_validation.rs` | Router integration | PASS |
| 2 | A provider referenced by a route returns HTTP 409 and remains present. | `crates/tl-server/tests/gateway/route_validation.rs` | Router integration | PASS |
| 3 | Runtime API keys cannot delete provider configuration. | `crates/tl-server/tests/gateway/input_enforcement.rs` | Authorization integration | PASS |
| 4 | The Next.js proxy sends a workspace-authorized DELETE and returns 204. | `apps/web/lib/server/proxy-helpers.test.ts` | Web integration | PASS |
| 5 | Editing sends mutable metadata by PATCH without resending the stored key. | `apps/web/components/workspace/GatewayPageContent.test.tsx` | Component | PASS |
| 6 | Deletion requires an explicit permanent-delete confirmation before the request is sent. | `apps/web/components/workspace/GatewayPageContent.test.tsx` | Component | PASS |

## Validation

| Command | Result |
|---|---|
| `cargo test -p tl-server --test gateway` | PASS — 21 tests |
| `pnpm --filter web exec vitest run components/workspace/GatewayPageContent.test.tsx lib/server/proxy-helpers.test.ts` | PASS — 10 tests |
| `pnpm --filter web typecheck` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo run -p tl-codegen -- --check` | PASS — OpenAPI and generated artifacts are in sync |
| `pnpm lint:boundaries` | PASS |

Coverage was not measured for this focused slice. The changed behaviors are covered at the Rust router, Next proxy, and user-visible component boundaries; Postgres-backed deletion relies on the existing foreign-key constraint plus the repository conflict mapping.
