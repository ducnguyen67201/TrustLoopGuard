# Telemetry Capture

Telemetry capture accepts OpenTelemetry spans as additive evidence for a Featherlane
[Run](runs.md). It is an observability and correlation plane, not an authorization plane.
`POST /v1/events` remains the synchronous policy decision boundary.

## Direct OTLP/HTTP

`POST /v1/otel/v1/traces` accepts OTLP/HTTP protobuf. The Rust handler authenticates the caller,
resolves workspace and environment from trusted request context, applies strict request/count/value
limits, normalizes the spans, validates correlation, and commits valid spans before acknowledging
them. Invalid spans produce OTLP partial success. A transient storage failure returns a retryable
service error rather than acknowledging evidence that was not durable.

The required correlation attributes are:

- `featherlane.run.id` — an existing Rust-created Run UUID in the authenticated environment.
- `featherlane.agent.id` — a registered participating agent.

`featherlane.run.event.id` optionally links a span to an event that belongs to that Run.
`featherlane.flush.id` optionally provides the durable receipt used by the capture barrier.
`gen_ai.agent.id` is stored only as an external provider identity, and
`gen_ai.conversation.id` is retained only when the integrator genuinely has one. Run IDs,
conversation IDs, decision trace IDs, OTel trace IDs, and OTel span IDs remain distinct.

Caller-provided attributes and baggage cannot select a workspace, authorize an agent, or create a
Run. Rust validates every association against authenticated storage state.

## Privacy

Metadata-only capture is the default. Prompt, completion, tool argument, body, and content fields are
removed unless both workspace data handling and the agent evaluation profile allow a stricter
mode. Redacted capture requires explicit redaction evidence. Encrypted-artifact mode stores bounded
artifact references and checksums, not raw content in span or snapshot rows.

Normalization records whether content was omitted, redacted, missing redaction evidence, or replaced
by an encrypted artifact reference. Attribute nesting, arrays, events, links, IDs, timestamps, and
body size are bounded before persistence.

## Collector

An OpenTelemetry Collector is optional. Direct OTLP/HTTP works immediately. The recipe under
`examples/otel-collector/` adds memory limiting, batching, attribute filtering, retry, and an
authenticated exporter, but the Collector remains stateless and replaceable. Server-side privacy
and authorization checks still run because callers may bypass the example Collector.

## Capture Timing

SDKs expose dependency-free correlation values and an optional `forceFlush` hook. Before finalizing,
the SDK generates a flush ID, asks the integrator's telemetry provider to flush spans carrying that
ID, then supplies it to the Run finalization request. Without controlled telemetry, the capture
barrier uses a bounded quiet period. The full snapshot/job semantics are defined in
[evaluations.md](evaluations.md).
