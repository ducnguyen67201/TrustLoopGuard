# Event Engine

The event engine is the Rust-owned contract for deciding whether a proposed agent step may happen. It keeps TrustLoopGuard SDK-first: customer runtimes call the SDKs, SDKs call Rust, and every adapter converges on the same `tl-core` vocabulary.

## Ownership

| Surface | Owner | Responsibility |
|---|---|---|
| Event and decision wire types | `crates/tl-core` | Defines `GuardEvent`, `EventKind`, labels, provenance, tool metadata, and additive `Decision` evidence fields. |
| Runtime evaluation seams | `crates/tl-engine` | Normalizes compatibility requests, resolves event context, runs checks, composes decisions, and exposes no-op stage traits. |
| HTTP entry point | `crates/tl-server` | Accepts `/v1/check`, resolves workspace/environment, applies redaction policy, loads enabled policies, and returns a `Decision`. |
| Trace persistence | `crates/tl-storage` | Persists decision traces through the existing trace writer. |

`apps/web` may display traces and call same-origin proxy routes, but it does not own event-engine contracts, runtime checks, or trace storage.

## Contract Vocabulary

`CheckRequest` is the public `/v1/check` compatibility request. `GuardEvent` is the normalized event shape that SDKs, gateway code, and host adapters can share internally.

A `GuardEvent` contains:

- `kind` - the dotted event taxonomy, such as `output.proposed`, `tool.call.proposed`, or `database.mutation.proposed`.
- `principal` - resolved workspace, environment, agent, user/session/task, and optional run/run-event identity.
- `action` - the operation being proposed, its parameters, and the side-effect class.
- `sources` - inputs that influenced the proposed step, with origin and labels.
- `provenance` - a map from output or parameter paths to source ids.
- `context` - caller-supplied JSON that travels with the event.

Tool metadata describes known tools independently of a specific event: side-effect class, reversibility, parameter roles, allowed sources, approval requirements, and sandbox hints.

`Decision` remains the result contract. Evidence fields such as `violated_rule`, `remediation`, `source_chain`, `risk_source`, `failure_mode`, `harm_class`, and `constraints` are optional and omitted when empty, so existing `/v1/check` callers keep the same response shape.

## Current Runtime Flow

```text
--------------------+        +-------------------------+
| SDK / gateway /    | -----> | POST /v1/check          |
| embedded caller    |        | CheckRequest            |
+--------------------+        +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Server redaction, auth, |
                              | workspace/environment  |
                              +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Engine compatibility   |
                              | normalizer available   |
                              | CheckRequest ->        |
                              | output.proposed event  |
                              +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Existing parallel tier |
                              | orchestrator           |
                              +------------+------------+
                                           |
                                           v
                              +------------+------------+
                              | Decision JSON + async  |
                              | trace side effect      |
                              +-------------------------+
```

The compatibility normalizer exists in `tl-engine::event_pipeline` and can map a legacy request into `GuardEvent { kind: output.proposed, action.operation: "output", ... }`. The default event pipeline collaborators are no-ops, so they do not introduce new writes, network calls, storage schema, or verdict changes.

## Stage Seams

The event pipeline exposes small trait seams so each concern can be implemented independently:

- `Normalizer` builds the canonical event.
- `PrincipalResolver` attaches workspace, environment, and identity context.
- `ToolMetadataProvider` looks up side-effect and approval metadata.
- `LabelResolver` attaches trust, confidentiality, and integrity labels.
- `ProvenanceResolver` records which sources influenced which output paths.
- `Checker` produces blocking or rewriting findings.
- `SignalProvider` adds advisory signals.
- `DecisionComposer` turns findings and signals into a `Decision`.
- `TracePersister` enqueues trace side effects.

The no-op context wires all of these as inert implementations. That makes the stage boundaries real without changing the customer-visible runtime.

## Compatibility Rules

- Old `CheckRequest` JSON must keep deserializing.
- Empty evidence on `Decision` must not appear in serialized `/v1/check` responses.
- `/v1/check` verdict, reason, policy, trace, run, redaction, cache, escalation, and latency semantics stay owned by the existing Rust runtime path.
- New SDK-visible capabilities start in `tl-core`, then flow through OpenAPI and generated SDK types.
- Durable event storage is introduced only when the owning Rust storage path and trace API are defined.
