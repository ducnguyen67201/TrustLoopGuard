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
Workspace -> Environment -> Agent -> Run -> Run event -> Trace / Decision
```

`POST /v1/events` evaluates one proposed event and returns one `Decision`. The run and trace are stamped with the resolved environment. When the event principal includes `run_id` and optionally `run_event_id`, the async trace writer persists those IDs on the trace row.

If an event references a `run_id`, that run must belong to the same resolved environment as the runtime key or trusted dashboard context. Cross-environment run linkage is rejected so dev traffic cannot be attached to production run history.

Clients can omit `run_id` and `run_event_id`; those traces remain valid and ungrouped.
The TypeScript `guardAgent(...)` decorator does not omit them by default: each
`reply()` creates one `chat_session` Run when no Run is already active, and all
guarded tool/output events inside that reply inherit its ID. It also records
the raw input as a `user_turn` and the proposed output as an `assistant_turn`.
The user turn is observability only; it does not create a runtime decision.
`run: false` explicitly disables automatic Run and transcript persistence while
leaving tool/action/output enforcement enabled.

SDKs also expose scoped run helpers so callers do not have to pass ids into every guard call. TypeScript uses `client.withRun(...)` and nested `run.withEvent(...)`; Python uses `with client.run(...)` / `async with client.run(...)`; Rust uses `client.with_run(...)` with an explicit scoped `RunClient`. Inside those scopes, `submitEvent` / `submit_event`, high-level output `guard()` calls, and tool-call helpers attach the active `run_id` and optional `run_event_id` unless the caller already set those fields.

Automatic TypeScript Runs have two lifecycle modes:

- Reply scope is the default and creates one one-shot Run for each `reply()`.
  This is the safe generic boundary because one agent object may serve several
  unrelated users.
- Session scope requires a stable `externalId` plus a deterministic
  `registerEnd` callback. The first guarded output or local tool call lazily
  creates one Run, every guarded boundary in that wrapped session reuses its
  ID, and the callback completes, fails, or cancels it.

The dependency-free `liveKitRun(...)` helper implements session scope for a
LiveKit AgentSession. It uses the room SID supplied by the caller as
`external_id`, defaults the kind to `live_call`, and maps the framework
close event to a terminal Run status. `agentId` remains the stable registered
agent identity; it is not a session key.

An explicit `client.withRun(...)` scope still wins for the current async
boundary and is never nested. `run: false` keeps traces ungrouped. Automatic
Run persistence is observability bookkeeping: start, turn-event, and finish
storage failures may emit a typed lifecycle warning but do not replace the
guard result or the original agent error.

The TypeScript SDK performs automatic grouping and transcript capture only when
its supported Node.js runtime provides isolated async context. In unsupported
browser/edge fallbacks, authorization still runs but automatic Run observation
is skipped rather than risk linking one concurrent session's text to another.

Human review outcomes can be appended to a trace after the decision. Run detail views display the latest linked review outcome for each trace, but review event ownership and analytics are described in [human-review-analytics.md](human-review-analytics.md).

## Events

Run events are the ordered timeline inside a run. They are deliberately generic so chat sessions, live calls, workflows, and background jobs share one monitoring model:

- Chat sessions use `user_turn` and `assistant_turn`.
- Live calls use `user_turn`, `assistant_turn`, `interruption`, and `retry`.
- Workflows use `workflow_step` and `tool_call`.
- Jobs use `workflow_step`, `system_event`, or `other`.

Each event may include a label, input summary, output summary, and metadata.
The TypeScript `guardAgent(...)` integration captures raw input and proposed
output in turn summaries by default when automatic Runs are enabled. Other
integrations should treat raw prompt, transcript, and tool-payload capture as
an explicit integration contract; summaries otherwise remain monitoring
context rather than an authorization input.

Events are written explicitly with `POST /v1/runs/{run_id}/events`. Runtime decisions link to an existing run event by passing `run_id` and `run_event_id` inside `GuardEvent.principal`.

`run_event_id` is only accepted with a matching `run_id`; the server rejects a runtime event when the event id does not belong to the provided run in the resolved environment.

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

For an automatic session Run, the TypeScript SDK deduplicates concurrent first
boundaries through one in-flight create request. A framework end that races
creation waits for that request and active guarded boundaries before sending
one terminal update. Ending an idle session creates no empty Run. A failed
create leaves that boundary ungrouped and may be retried by a later independent
boundary; nested calls do not retry inside the same failed boundary.

Automatic Run metadata contains the integration marker plus caller-supplied
metadata. Raw input and output are stored on their turn events, not copied into
Run metadata. Durable state and environment validation remain owned by the
Rust Run API.

Gateway integrations create runs automatically. Each accepted provider-compatible gateway request becomes one `chat_session` run, and the gateway links its input/output policy checks to that run. If the request carries `X-TLG-Run-External-Id`, the gateway uses that value as the run `external_id` and reuses an existing run for the same route agent plus external id. Streaming integrations use this to group all model calls from one external session into a single dashboard run.

The dashboard run detail view uses the same Rust-owned run detail API and refreshes while the page is open so live demos can show new events and traces without manually reloading. The web server resolves the persisted `agent_id` against the Rust-owned agent profiles already loaded for the active workspace and environment. When the profile is available, the page shows its display name and links to that Agent configuration while keeping the raw identifier visible and copyable; when it is unavailable, the page labels that state and retains the raw identifier. Gateway-created chat sessions create one `user_turn` for the exact checked request and one `assistant_turn` for a successful provider response. Input and output checks link to their respective turns, so the timeline reads as a transcript instead of an ungrouped trace list. Gateway system events carry provider usage, deterministic LLM budget decisions, and semantic-judge usage as typed audit evidence. `RunDetail` exposes the latest provider and budget evidence plus every guardrail invocation without making run events a second enforcement or accounting store.

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

TrustLoopGuard generates and owns `run_id`. `external_id` is searchable support context and may be used by gateway integrations to find an existing run for the same observed upstream session. It is never used for authorization. Run list and detail responses include environment fields so dashboard rows and analytics can show where each execution happened.
