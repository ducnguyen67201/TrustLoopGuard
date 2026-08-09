# Optional OpenTelemetry Collector

Featherlane AI accepts OTLP/HTTP protobuf directly at
`/v1/otel/v1/traces`. The Collector is optional and stateless; it is useful
for batching, retry, memory limiting, and defense-in-depth redaction.

Set:

- `FEATHERLANE_OTLP_ENDPOINT`, for example `https://api.example.com/v1/otel`
- `FEATHERLANE_API_KEY`
- `FEATHERLANE_ENVIRONMENT_ID`

Every exported span must carry the immutable correlation attributes returned
by the Featherlane run helper:

```text
featherlane.run.id=<run UUID>
featherlane.agent.id=<registered agent ID>
featherlane.run.event.id=<optional run-event UUID>
featherlane.flush.id=<optional force-flush receipt ID>
```

Do not map workspace identity from baggage or span attributes. The Rust OTLP
endpoint derives workspace and environment from authenticated request context.
OpenTelemetry GenAI conventions are still marked Development, so
`gen_ai.conversation.id`, `gen_ai.operation.name`, and `gen_ai.agent.id` are
captured as descriptive metadata only; none of them defines the Run boundary.

The server applies its own content policy even when this recipe is used.
Metadata-only capture is the default, and prompt/completion/tool/body fields
are stripped unless the agent and workspace explicitly permit a stricter
redacted or encrypted-artifact-reference mode.
