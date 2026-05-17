# SDK-Driven Development at TrustLoopGuard

TrustLoopGuard is open-source. Adoption happens when a stranger drops one of
our SDKs into their agent runtime and ships. That makes the SDK surface — not
the engine internals — the contract with users.

This doc captures the rules we hold ourselves to so that contract stays honest.

## The four rules

### 1. Engine-only PRs aren't done

If a feature changes user-visible behavior, it lands across the full surface in
the same PR:

- `tl-core` types updated (these are the source of truth for the wire format)
- `cargo run -p tl-codegen` regenerated `docs/openapi.yaml`,
  `policies/*.schema.json`, and `sdks/typescript/src/generated/*`
- `crates/tl-sdk-rust` exposes the new method or field
- `sdks/python/src/trustloopguard/client.py` exposes the new method or field
- `sdks/typescript/src/client.ts` exposes the new method or field
- The example apps under `apps/example-*` exercise the new surface

A PR that adds an engine capability without exposing it through every SDK is
half-shipped. Half-shipped features don't merge.

### 2. No internal imports in `apps/` or `demo/`

Example apps use *only* what a stranger could install from crates.io / PyPI /
npm. That means they import from `tl-sdk-rust`, `trustloopguard` (Python), and
`@trustloopguard/sdk` (TS) — not from `tl-core`, `tl-engine`, `tl-policy`,
`tl-server`, or any other internal crate.

If an example needs an internal type, the SDK is missing something. Add it to
the SDK first; then update the example.

This rule is enforced by lint (see PR 11).

### 3. The README quickstart works on a clean machine

The top-level README contains a copy-paste quickstart per language. CI runs it
literally — fresh container, no caches, no insider knowledge — and asserts a
`Decision` is returned.

If the quickstart breaks, that's a release blocker, not a docs ticket. The
README is executable specification, not marketing.

### 4. Cross-cutting concerns live in the SDK, once

Retries, auth, error mapping, tracing, timeouts, rate-limit handling — solved
in `tl-sdk-rust` / `trustloopguard` / `@trustloopguard/sdk`, never re-solved
per example or per caller.

If you catch yourself writing the same try/except, the same backoff loop, the
same auth header in two demos: it belongs in the SDK.

## Why this is harder than it sounds

Each rule is mechanical. The hard part is *refusing* to merge work that
violates them — especially under deadline pressure when "we'll add the Python
binding next sprint" feels reasonable. It isn't. Next sprint never comes, and
the SDK falls behind the engine until adoption stalls.

The discipline is the refusal.

## How features are built (the loop)

For every user-visible change:

1. **Write the call site first.** In a scratch file, write the line of code
   you wish a stranger could write to use this feature. If it feels awkward,
   the API is wrong — fix it on paper before writing real code.
2. **Update the Rust types in `tl-core`.** These flow downstream into OpenAPI,
   JSON Schemas, TS types, and (via `datamodel-code-generator`) Pydantic
   types.
3. **Run `cargo run -p tl-codegen`.** The scratch snippet from step 1 should
   compile-check against the regenerated SDK types. If it doesn't, the wire
   contract is wrong; back to step 1.
4. **Update the example apps before the implementation.** They won't pass
   yet — that's the point. The example is the executable form of "what does
   success look like for a stranger."
5. **Implement engine-side.** Make the example pass. Engine internals can be
   ugly; strangers don't see them.
6. **Wire the SDK surface.** Thin pass-through in each language. Cross-cutting
   helpers should already exist; you're only exposing the new method.
7. **Mirror to Python and TypeScript.** Mostly mechanical because of codegen.
   Hand-write only the ergonomic wrapper.
8. **Run the example apps in all three languages.** Same input → same
   decision. If they diverge, the SDK is leaking implementation details.
9. **Run `make verify-contract` locally** after changing Rust wire types,
   server route annotations, or generated SDK models.
10. **Run `make quickstart` locally** before pushing.
11. **Tick the PR template checklist.** CI runs `codegen-check`, `quickstart`,
    and the internal-import lint as required gates.

## Reviewer checklist

When reviewing, the question to ask is **not** "is this engine code clean?".
It's:

> Could a stranger use this feature from the SDK docs alone?

If you have to read `tl-engine` to understand how to call the new method, the
docs are wrong. If the example app imports an internal crate to make the new
feature work, the SDK is wrong. If `make quickstart` is green but the new
feature isn't exercised, the example is wrong.

## What this kills

- Half-shipped features (engine without bindings)
- Doc rot (README is executed in CI)
- API archaeology (new users read the example, not the engine)
- Bikeshedding internal abstractions (engine internals stop being a review
  battleground; the SDK is the contract)

## What this costs

- Each PR is ~30% bigger (engine + 3 SDKs + example updates).
- The first feature after adopting the discipline is slow because helpers
  (errors, retry, auth) didn't exist before. Subsequent features are fast.

## Required CI gates

These workflows run on every PR. They are **required status checks** on
`main` — see [`MERGE_GATES.md`](MERGE_GATES.md) for the
branch-protection settings.

| Gate                     | Workflow                                       | What it checks |
|--------------------------|------------------------------------------------|----------------|
| `codegen drift`          | `.github/workflows/codegen-check.yml`          | `tl-core` source-of-truth matches the generated OpenAPI / JSON Schemas / TS types / Pydantic models on disk |
| `sdk build`              | `.github/workflows/sdk-build.yml`              | All three SDKs compile and pass tests |
| `quickstart`             | `.github/workflows/quickstart.yml`             | The README copy-paste flow works on a clean Ubuntu runner |
| `boundary lint`          | `.github/workflows/lint-sdk-boundary.yml`      | `apps/example-*` and `demo/` only import the published SDK surface; public API DTOs live in `tl-core` |

Local equivalents:

| Gate                     | Local command                          |
|--------------------------|----------------------------------------|
| `codegen drift`          | `make verify-contract`                 |
| `sdk build`              | `make sdk-all`                         |
| `quickstart`             | `make quickstart`                      |
| `boundary lint`          | `make ci-lint`                         |
| All four                 | `make ci`                              |

## Out of scope

This doc is about discipline at the SDK boundary. It does **not** govern:

- Engine internal architecture (`tl-engine`, `tl-fuzzy`, `tl-storage`, …)
- Server transport details (`tl-server`)
- Internal CLI ergonomics (`tl-cli` is for operators, not third-party
  integrators; it has its own UX bar)

## Run grouping helper

SDKs expose runs as the grouping layer above individual checks:

```ts
const run = await client.startRun({
  agent_id: "support-agent",
  kind: "chat_session",
});
await client.check({
  run_id: run.id,
  run_event: {
    kind: "assistant_turn",
    label: "Turn 1",
    input_summary: "Customer asks about a refund",
    output_summary: "Agent drafts refund answer",
  },
  agent_id: "support-agent",
  channel: "chat",
  input,
  proposed_output,
});

await client.finishRun(run.id);
```

The same shape exists in Python as `start_run`, `check`, and `finish_run`. `createRunEvent` / `create_run_event` remain available for timeline moments that do not need an immediate guardrail check. `run_id`, `run_event_id`, and `run_event` are optional on `CheckRequest` so old clients continue to work.

For the engine roadmap, see the 21-PR plan in the repo issues.
