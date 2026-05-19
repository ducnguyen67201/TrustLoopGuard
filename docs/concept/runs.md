# Runs

Runs are TrustLoopGuard's execution-level observability container. A run is one customer agent execution after guardrails have been assigned: a chat session, live call, workflow execution, or background job.

TrustLoopGuard observes runs; it does not start, host, schedule, or orchestrate the customer agent.

## Ownership

Rust owns run state.

```text
Browser / SDK
  -> Next.js API proxy when same-origin is needed
  -> Rust /v1/runs
  -> tl-storage Postgres
```

The dashboard may display runs and proxy same-origin requests, but it must not store durable run state.

## Relationship to traces

The runtime decision boundary stays unchanged:

```text
Workspace -> Agent -> Run -> Run event -> Trace / Decision
```

`POST /v1/check` still evaluates one proposed output and returns one `Decision`. When the request includes `run_id` and optionally `run_event_id`, the async trace writer persists those IDs on the trace row. Callers may also include `run_event` inline on `CheckRequest`; the server creates that event before evaluation, then links the persisted trace to the created event. This keeps run grouping off the engine hot path while avoiding a separate event call for every simple turn.

Older clients can omit `run_id`, `run_event_id`, and `run_event`; those traces remain valid and ungrouped.

Human review outcomes can be appended to a trace after the decision. Run detail views display the latest linked review outcome for each trace, but review event ownership and analytics are described in [human-review-analytics.md](human-review-analytics.md).

## Events

Run events are the ordered timeline inside a run. They are deliberately generic so chat sessions, live calls, workflows, and background jobs share one monitoring model:

- Chat sessions use `user_turn` and `assistant_turn`.
- Live calls use `user_turn`, `assistant_turn`, `interruption`, and `retry`.
- Workflows use `workflow_step` and `tool_call`.
- Jobs use `workflow_step`, `system_event`, or `other`.

Each event may include a label, input summary, output summary, and metadata. Raw prompts, transcripts, and tool payloads should stay out of event summaries unless the customer explicitly opts into that level of capture; summaries are for monitoring context.

Events can be written explicitly with `POST /v1/runs/{run_id}/events` or implicitly by passing `run_event` on `POST /v1/check`. Explicit writes are useful when the integration observes timeline moments that do not need a guardrail decision. Inline writes are the default SDK path for turns/steps that are checked immediately.

## Lifecycle

Customers create and update runs through the SDK or HTTP API:

```text
POST   /v1/runs
GET    /v1/runs
GET    /v1/runs/{run_id}
PATCH  /v1/runs/{run_id}
POST   /v1/runs/{run_id}/events
GET    /v1/runs/{run_id}/events
GET    /v1/runs/{run_id}/traces
```

Supported kinds:

- `chat_session`
- `live_call`
- `workflow`
- `job`
- `other`

Supported statuses:

- `warming`
- `running`
- `completed`
- `failed`
- `canceled`

v1 treats statuses as flexible monitoring labels. It does not enforce a strict state machine.

## External ID

`external_id` is an optional customer/platform correlation key. Examples include Twilio call IDs, LiveKit room IDs, n8n execution IDs, and customer chat session IDs.

TrustLoopGuard generates and owns `run_id`. `external_id` is searchable support context only; it is never used for authorization or internal trace grouping.
