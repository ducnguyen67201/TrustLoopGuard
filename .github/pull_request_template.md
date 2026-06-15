<!--
TrustLoopGuard is SDK-driven. See docs/SDK_DRIVEN.md for the full philosophy.

The checklist below is mandatory for any PR that changes user-visible
behavior. If your PR is engine-internal only (refactor, perf work, internal
test) tick "engine-internal only" and skip the SDK-parity boxes.
-->

## 📝 Summary

<!-- One paragraph: what changes for the caller? Skip "what" if the diff is
self-explanatory; lead with "why". -->

## 🎨 UI Changes

<!--
Required for user-facing UI changes. Add before/after screenshots, screen
recordings, or mark "N/A — no UI changes".
-->

| Before | After |
|--------|-------|
|        |       |

## 🧭 Type of change

- [ ] User-visible feature or behavior change
- [ ] Engine-internal only (refactor, perf, internal test, internal docs)
- [ ] Build / CI / tooling
- [ ] Docs only

## 🧩 SDK-parity checklist

> ℹ️ Required for user-visible changes. Skip with "N/A — engine-internal" for
> engine-only PRs.

- [ ] `tl-core` types updated (the source of truth for wire formats)
- [ ] `cargo run -p tl-codegen` ran clean — `codegen-check` is green
- [ ] `crates/tl-sdk-rust` exposes the new surface
- [ ] `sdks/python` exposes the new surface
- [ ] `sdks/typescript` exposes the new surface
- [ ] No new imports of `tl-core`, `tl-engine`, `tl-policy`, `tl-server`,
      `tl-fuzzy`, `tl-storage`, or `tl-replay` in `demo/`

## 🔁 Cross-cutting concerns

> ⚠️ If your change touches errors, retries, auth, tracing, timeouts, or rate
> limits, the change must land in the SDK helpers, not in consumer code.

- [ ] N/A — this PR doesn't touch cross-cutting concerns
- [ ] Cross-cutting change landed in `tl-sdk-rust` and was mirrored to Python
      + TS

## ✅ Test plan

<!--
- [ ] Unit tests added for the SDK surface change
- [ ] Parity test added/updated (all three SDKs produce the same result for
      the same input)
-->

## 👀 Reviewer prompt

Read the SDK surface diff, not just the engine diff. Could a stranger reading
the SDK docs alone use this feature?
