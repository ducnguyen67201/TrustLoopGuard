# Plugin contract

The single signature every TrustLoopGuard host adapter mirrors, in every language. If you're writing a new SDK or a new host integration (OpenAI middleware, LiveKit, Pipecat, Vapi, custom), this is the shape.

## The contract

```
Guard.check(draft, ctx) -> Decision
```

That's it. One method. Same name in every language. Same return type.

## Pseudocode

```
fn check(draft: Draft, ctx: Context) -> Decision
```

### `Draft` — what the agent wants to do

```
Draft {
  agent_id: String           // which of the customer's agents
  channel: Channel           // voice | chat | email | other(string)
  input: String              // what the user said (matcher context)
  proposed_output: String    // what the agent wants to say/do (the thing under review)
  policies: List<String>     // optional: scope to specific policy ids
}
```

### `Context` — anything the customer wants logged but not evaluated

```
Context {
  trace_id: Option<String>   // caller-supplied for correlation; auto-generated if absent
  metadata: JSON             // free-form: user_id, session_id, tier, locale, etc.
}
```

`Context.metadata` is **not** evaluated by matchers. It's persisted on the `Decision` for audit and dashboards. Don't put anything that should affect the verdict here — that goes on `Draft`.

### `Decision` — what TrustLoopGuard returns

Same as `tl-core::Decision`. See [glossary.md](glossary.md#decision).

```
Decision {
  trace_id: String
  verdict: Allow | Block | Rewrite | Escalate
  reason: String
  triggered_policies: List<TriggeredPolicy>
  safe_output: Option<String>     // present when verdict = Rewrite
  checked_input_excerpt: Option<String>
  checked_output_excerpt: Option<String>
  latency_ms: u64
}
```

Versioning is enforced at the URL: `/v1/check` is v1; when wire shape breaks, `/v2/check` ships alongside it.

## Required behaviors per language binding

Every host SDK must:

1. **Expose exactly `Guard.check(draft, ctx)`** as the public entry point. Method name, argument order, return shape — same everywhere. Customers should be able to read one SDK's docs and use any of them.
2. **Default `trace_id` generation** if the caller doesn't supply one (UUIDv4). Surface it on the returned `Decision` so the customer can log it.
3. **Honor URL versioning**. Default to `/v1/check`. If the server returns 404 / 410 on the configured version, the SDK fails closed (`Block` or `Escalate`, configurable). SDKs do not silently fall back to a different major version.
4. **Respect `fail_open` vs `fail_closed`** on transport errors. Per-policy config in v2; per-client config in v1. Default = fail-closed. See [glossary.md](glossary.md#fail-open-vs-fail-closed).
5. **Be cancellable / deadline-aware.** Voice callers will pass a deadline (`tokio::time::timeout` in Rust, `AbortSignal` in TS, `asyncio.timeout` in Python). The SDK must cancel the in-flight HTTP request when the deadline fires, not just discard the result.
6. **Never log `proposed_output` by default.** It is potentially user-facing PII. Log `trace_id` and `verdict` only; let the customer opt in to body logging.

## Required behaviors per host adapter

Adapters wrap a third-party SDK (OpenAI, LiveKit, etc.) so the customer doesn't write `Guard.check` themselves. Each adapter must:

1. **Intercept the model's proposed output before it reaches the user.** For streaming providers, this means buffering enough tokens to make a defensible decision; for non-streaming, this is just one call.
2. **Replace `proposed_output` with `safe_output` when `verdict = Rewrite`.** Customer should never see the unsafe text.
3. **Surface `Block` and `Escalate` cleanly.** Don't crash; emit a configurable fallback message ("Let me get a teammate") and log the trace_id.
4. **Pass through `Context.trace_id` to the customer's logging.** That's the join key between TrustLoopGuard's logs and theirs.

## Streaming variant

Voice and token-streaming chat use a different surface, defined in `tl-stream::StreamingChecker`:

```
fn push(chunk: String) -> StreamDecision
```

Where `StreamDecision = Continue | Interrupt { verdict, reason }`. The host adapter is responsible for actually interrupting the upstream model the moment `Interrupt` fires — TrustLoopGuard tells you *when*, not *how*.

## What the contract is NOT

- It is **not** a tool/function-call permission check. That's Clawvisor's surface.
- It is **not** a place to do retrieval, memory, or RAG. The customer pre-processes; we evaluate.
- It is **not** stateful across calls. Each `check` is independent unless the customer threads `trace_id` and we look up history (we don't, today).

## Adding a new language binding

1. Generate types from `crates/tl-core/src/lib.rs` or hand-write them — but they must serialize to the same JSON.
2. Implement `Guard.check(draft, ctx)` over HTTP using the OpenAPI spec at `docs/openapi.yaml`.
3. Validate against the conformance suite (will live at `tests/sdk-conformance/` — TODO).
4. Mirror the doc surface: same examples, same option names, same default behaviors.

If you find yourself wanting to deviate from the contract because it's awkward in your language, file an issue against `tl-core` instead. The point of the contract is that customers can switch SDKs without re-learning the API.
