# Repository Agent Instructions

These instructions apply to all AI coding agents working in this repository.

## Architecture: Rust Backend Is the Source of Truth

The dashboard web app must not become a second backend. TrustLoopGuard has one durable/runtime backend: the Rust service in `crates/tl-server`, backed by `crates/tl-storage`. The Next.js app in `apps/web` is a UI and same-origin proxy layer only.

Authoritative backend ownership:
- Rust HTTP API: `crates/tl-server`
- Rust durable storage: `crates/tl-storage`
- Rust wire contracts: `crates/tl-core`
- Web dashboard/proxy: `apps/web`

Target architecture:

```text
                         source of truth
                               |
                               v
+---------+        +----------------------+        +----------------------+
| Browser | -----> | Next.js dashboard UI | -----> | Next API proxy      |
|         |        | apps/web pages       |        | apps/web/app/api/*  |
+---------+        +----------------------+        +----------+-----------+
                                                              |
                                                              | HTTP /v1/*
                                                              v
                                                   +----------+-----------+
                                                   | Rust API server     |
                                                   | crates/tl-server    |
                                                   +----------+-----------+
                                                              |
                                  +---------------------------+---------------------------+
                                  |                           |                           |
                                  v                           v                           v
                       +----------+-----------+    +----------+-----------+    +----------+-----------+
                       | Runtime guard logic  |    | Durable storage     |    | Wire contracts      |
                       | policy evaluation    |    | crates/tl-storage   |    | crates/tl-core      |
                       +----------------------+    +----------+-----------+    +----------------------+
                                                              |
                                                              v
                                                   +----------+-----------+
                                                   | Postgres / runtime  |
                                                   | persistence         |
                                                   +----------------------+

Forbidden source-of-truth path for guardrail/runtime data:

+----------------------+        +----------------------+
| Next API route       | -X---> | Web DB / Drizzle    |
| apps/web/app/api/*   |        | runtime ownership   |
+----------------------+        +----------------------+
```

SDK runtime integration path:

```text
Customer / integrator runtime

+----------------------+        +----------------------+
| Customer AI agent    | -----> | TrustLoopGuard SDK   |
| app code             |        | TS / Python / Rust   |
+----------------------+        +----------+-----------+
                                           |
                                           | POST /v1/check
                                           | Authorization: Bearer <api key>
                                           v
                                +----------+-----------+
                                | Rust API server     |
                                | crates/tl-server    |
                                +----------+-----------+
                                           |
                         +-----------------+-----------------+
                         |                 |                 |
                         v                 v                 v
              +----------+------+  +-------+----------+  +---+---------------+
              | Policy runtime  |  | Agent profiles   |  | Knowledge/runtime |
              | tl-engine       |  | tl-storage       |  | context           |
              +----------+------+  +------------------+  +-------------------+
                         |
                         v
              +----------+-----------+
              | Decision             |
              | allow/block/rewrite  |
              | escalate + trace_id  |
              +----------+-----------+
                         |
                         v
+----------------------+        +----------------------+
| TrustLoopGuard SDK   | -----> | Customer AI agent    |
| typed response       |        | applies decision     |
+----------------------+        +----------------------+

Trace persistence side effect:

+----------------------+        +----------------------+
| Rust API / engine    | -----> | Rust trace writer    |
| decision produced    |        | crates/tl-storage    |
+----------------------+        +----------+-----------+
                                           |
                                           v
                                +----------+-----------+
                                | Postgres traces     |
                                | dashboard reads via |
                                | Rust trace API      |
                                +----------------------+
```

SDK rules:
- Customer agents use the SDKs in `sdks/typescript`, `sdks/python`, or `crates/tl-sdk-rust`.
- SDKs call Rust API endpoints directly, especially `POST /v1/check` for runtime guard decisions.
- SDK runtime checks do not go through `apps/web`.
- The dashboard may display SDK-produced traces, but it must read them through Rust trace APIs.
- If an SDK needs a new capability, add the Rust endpoint and shared wire type first, then expose it in the SDK.

When adding or changing dashboard behavior:
- Browser/client components should call `apps/web/app/api/...` routes when they need same-origin HTTP.
- Next API routes may authenticate, validate UI-shaped input, translate camelCase/snake_case, attach auth/session context, and proxy Rust responses.
- Next API routes must not own guardrail business logic, runtime authorization, policy evaluation, trace storage, agent storage, API key enforcement, runtime settings, knowledge-source persistence, or durable dashboard data.
- If a page needs durable data or runtime behavior, add or extend a Rust `/v1/...` endpoint first, then make the web route proxy to it.
- Do not introduce new Drizzle/web-DB tables as the source of truth for guardrail runtime entities.
- Do not read/write existing web DB tables as the source of truth for decisions, policies, agents, API keys, runtime settings, or knowledge sources unless the task is explicitly a temporary migration/compatibility bridge.

Before implementing a page or API change, identify which layer owns the data. If ownership is guardrail/runtime/durable product data, Rust owns it and web proxies it.

## Page Integration Expectations

- `/` dashboard: recent decisions must come from Rust persisted traces, not `guardrail_decisions` or static fixture data.
- `/policies` and `/policies/new`: save/list through Rust policy APIs. Treat `workspace_policies` as legacy or migration input only.
- `/agents`: use Rust agent APIs/storage. Do not use `workspace_agents` as the source of truth.
- `/knowledge-sources`: add a Rust storage/API path before calling it backend-integrated.
- `/api-keys`: connect to Rust runtime auth/key enforcement before calling it backend-integrated.
- `/settings`: apply settings to Rust runtime configuration before calling it backend-integrated.
- `/team` and `/workspaces`: web ownership/membership pages; Rust guardrail storage integration is not expected unless explicitly requested.

## Implementation Checklist

Before changing a page or route:
1. Identify who owns the data.
2. If it is guardrail/runtime/durable product data, add or extend the Rust `/v1/...` API.
3. Add or update Rust storage in `crates/tl-storage` when persistence is required.
4. Update shared Rust wire types/codegen when the contract changes.
5. Make the Next API route a thin proxy to Rust.
6. Keep browser components calling the Next route, not Rust directly, unless the existing pattern for that feature says otherwise.
7. Remove or bypass legacy web DB reads/writes for the migrated path.

## Docs Are the Single Source of Truth (`docs/concept`)

`docs/concept/` is the authoritative description of what the system is. Code changes that drift from these docs are incomplete. Treat docs as part of the change, not a follow-up.

What lives in `docs/concept/`:
- `architecture.md` — high-level system shape, request flow, latency budget, layer ownership.
- `crates.md` — every Rust crate, its responsibility, and its dependency position.
- `glossary.md` — canonical definition of every domain term (Channel, Verdict, Policy, Decision, hot path, etc.).
- `plugin-contract.md` — plugin/extension interface contract.
- `web-dashboard-authentication.md` — dashboard auth model.
- `v0-design-decisions.md` — why the current shape exists; append, do not rewrite history.
- One service-/subsystem-level doc per durable concept. If a service has no concept doc, add one before merging the change that introduces it.

Mandatory doc updates (the change is not done until these are reflected):
- New crate, renamed crate, or changed crate responsibility → update `crates.md`.
- New or changed Rust `/v1/...` endpoint, request/response shape, or error semantics → update `architecture.md` and `docs/openapi.yaml`; update `glossary.md` if a new term appears.
- New durable entity, table, or storage repository → update `architecture.md` (ownership) and add or extend the relevant concept doc.
- New service, daemon, worker, or runtime component → add a concept doc for it under `docs/concept/` describing purpose, inputs, outputs, ownership, and where it sits in the request flow.
- New SDK capability or breaking SDK change → update `docs/SDK_DRIVEN.md` and any concept doc the capability touches.
- Auth/identity/runtime-policy change → update `web-dashboard-authentication.md` and/or `architecture.md`.
- Renamed or retired concept → update `glossary.md` and grep for stale references across `docs/concept/`.

Single-topic, centralized, unique (one doc owns one thing):
- Each doc in `docs/concept/` owns exactly one topic. The pipeline doc talks about the pipeline only. The auth doc talks about auth only. The storage doc talks about storage only.
- A topic has exactly one home. Do not describe the pipeline in `architecture.md` and again in a separate pipeline doc — the pipeline doc is the canonical source; `architecture.md` links to it.
- If you find yourself writing the same explanation in two docs, stop. Pick the canonical home, delete the other copy, and replace it with a link.
- Cross-cutting concerns (e.g. "how the engine talks to storage") belong in the doc that owns the **calling** side, with a link to the **callee's** doc. Do not duplicate the description on both sides.
- File name = topic. `pipeline.md` is about the pipeline. `web-dashboard-authentication.md` is about web dashboard auth. Do not let a doc grow into a second topic — split it out.
- When adding a new doc, first check whether the topic already has a home. If yes, extend that doc; do not create a parallel one.

Cleanliness rules for `docs/concept/`:
- Keep each doc short and onboarding-focused. Split before a doc exceeds ~400 lines.
- One canonical definition per term. If `glossary.md` already defines it, link instead of redefining.
- No "TODO", "Placeholder", "PR N", or "Phase N" scaffolding language in concept docs. If a section is not ready, omit it.
- No duplicated diagrams across docs. The canonical architecture diagram lives in `architecture.md` and is referenced from elsewhere.
- Remove stale sections in the same PR that makes them stale. Do not leave outdated content "for context".
- When a concept doc and code disagree, the code wins for current behavior, but the docs must be updated in the same change to match.

PR checklist addition:
- [ ] Identified which `docs/concept/` files this change affects.
- [ ] Updated those docs, or explicitly noted "no concept impact" in the PR description with reasoning.
- [ ] Verified glossary terms used in code and docs are still accurate.
- [ ] No scaffolding language introduced into `docs/concept/`.
- [ ] Each affected doc still owns exactly one topic. No content duplicated across docs.

## Coding Conventions

Follow the existing repository patterns first. Do not introduce a new framework, state-management style, error envelope, database access pattern, or file organization unless the current code has no suitable precedent.

Rust conventions:
- Keep wire types in `crates/tl-core`; do not duplicate request/response structs in server, storage, SDK, or web code.
- Put HTTP handlers and routing in `crates/tl-server`.
- Put durable persistence, repositories, Diesel schema usage, and migrations in `crates/tl-storage`.
- Keep runtime policy evaluation in Rust engine/server code, not in web routes.
- Use typed errors and existing `ApiError`/`ApiErrorCode` response patterns for public endpoints.
- Prefer repository methods over raw SQL at call sites. Keep Diesel queries inside storage/repo modules.
- Run `cargo fmt` for Rust changes. Run targeted `cargo test -p <crate>` for changed crates when practical.

TypeScript and Next.js conventions:
- Keep browser components calling same-origin routes under `apps/web/app/api/...`.
- Keep Next API routes thin: parse, validate, translate, proxy, return.
- Use shared/generated SDK types when available instead of hand-written duplicate shapes.
- Use `zod` at browser/web boundaries where runtime validation is needed.
- Preserve existing component structure and design-system primitives in `apps/web/components/ui`.
- Do not put secrets, provider keys, or runtime enforcement logic in client components.
- Run the relevant `pnpm` type-check/lint/test command for touched packages when practical.

API and contract conventions:
- Rust wire contracts are the source of truth. When a public API shape changes, update `tl-core`, regenerate artifacts, and update SDKs/web consumers in the same change.
- Keep Rust wire format conventions at the boundary. Translate only at the web/UI edge when the UI needs a different shape.
- Keep endpoint semantics consistent: `GET` reads, `POST` creates/actions, `PATCH` partially updates, `DELETE` deletes or soft-deletes according to existing storage behavior.
- Prefer additive API changes. If a breaking change is unavoidable, update docs, SDKs, examples, and tests together.

Testing and verification conventions:
- Add or update tests when behavior changes, not for documentation-only edits.
- For storage changes, include repository-level or endpoint-level coverage when practical.
- For web proxy changes, verify both request translation and error handling.
- For SDK changes, test the typed client surface and retry/error behavior if touched.
- Before pushing, inspect `git diff` and make sure every changed line is tied to the request.

## Think Before Coding

Don't assume. Don't hide confusion. Surface tradeoffs.

Before implementing:
- State assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them instead of picking silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop, name what is confusing, and ask.

## Simplicity First

Minimum code that solves the problem. Nothing speculative.

- No features beyond what was asked.
- No abstractions for single-use code.
- No flexibility or configurability that was not requested.
- No error handling for impossible scenarios.
- If 200 lines could be 50, rewrite it.

Ask: would a senior engineer say this is overcomplicated? If yes, simplify.

## Surgical Changes

Touch only what you must. Clean up only your own changes.

When editing existing code:
- Do not improve adjacent code, comments, or formatting unless needed for the task.
- Do not refactor things that are not broken.
- Match existing style, even if you would do it differently.
- If you notice unrelated dead code, mention it instead of deleting it.

When your changes create orphans:
- Remove imports, variables, and functions that your changes made unused.
- Do not remove pre-existing dead code unless asked.

Every changed line should trace directly to the user's request.

## Goal-Driven Execution

Define success criteria and loop until verified.

Transform tasks into verifiable goals:
- "Add validation" means write tests for invalid inputs, then make them pass.
- "Fix the bug" means write a test that reproduces it, then make it pass.
- "Refactor X" means ensure tests pass before and after.

For multi-step tasks, state a brief plan:

```text
1. [Step] -> verify: [check]
2. [Step] -> verify: [check]
3. [Step] -> verify: [check]
```

Strong success criteria let agents loop independently. Weak criteria require clarification.

## General Coding Discipline

- Make surgical changes that directly serve the task.
- Match existing style and contracts.
- Prefer typed/shared schemas over ad hoc JSON where this repo already has generated contracts.
- Run the smallest meaningful verification for the touched layer.
