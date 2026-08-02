# SDK agent adapters

SDK agent adapters attach Featherlane AI to framework-owned execution seams.
They route local executable tools and supported final-output boundaries through
the same Rust authorization contract used by explicit SDK callers, while
preserving the framework's agent object and run lifecycle.

## Supported discovery seams

| SDK / host | Discovery seam | Tool enforcement seam | Final-output seam |
|---|---|---|---|
| TypeScript / OpenAI Agents JS | mutable `agent.tools` array | local function-tool `execute()` | decorated `reply()` |
| TypeScript / LiveKit Agents | `agent.toolCtx.tools` and `toolCtx.updateTools()` | local function-tool `execute()` | decorated `reply()` when present |
| TypeScript / Mastra | `getToolsForExecution()` result | each resolved tool `execute()` | decorated `reply()` when present |
| TypeScript / compatible custom agent | `agent.tools` object map or array | each value's `execute()` | decorated `reply()` when present |
| Python / AG2 1.0 | public `agent.tools` and per-call tool event | outer `on_tool_execution()` middleware | outer `on_turn()` middleware |
| Python / Agno 2.x | `Function`, callable, `Toolkit`, and `run_context.tools` | first agent tool hook | last agent post-hook |

TypeScript adapters use structural inspection and do not import Mastra, OpenAI
Agents JS, or LiveKit at runtime. Python framework imports live only in their
matching optional integration module. Importing base `featherlane-ai` or
`featherlane_ai.integrations` does not require AG2 or Agno.

## Decoration flow

1. `guardAgent()` resolves one Featherlane AI `Client`.
2. The decorator creates one Run controller. Reply scope creates a
   `chat_session` Run per `reply()`; session scope lazily creates and reuses
   one Run for a supplied framework lifecycle.
3. The adapter finds a supported local tool registry.
4. Each tool's name, description, input schema, output schema when present, and
   execution function are normalized.
5. The input schema is canonicalized into a stable non-cryptographic schema
   identity. It is an execution identity, not a security digest.
6. The tool's `execute()` is replaced with a wrapper that submits
   `tool.call.proposed` through `Client.withAuthorizedAction()`.
7. The original execution context arguments are preserved. The proposed input
   is replaced with the exact parameters authorized by the Rust service.
8. The original tool executes at most once after `permit` or a successfully
   resumed approval. Deny, defer, failed approval, cancellation, and transport
   failure do not execute it.

Python uses framework-native hooks instead of replacing registry functions.
`guard_ag2()` installs one outer async middleware and uses the AG2 tool-call ID
as invocation identity. `guard_agno()` installs synchronous hooks for `Client`
and asynchronous hooks for `AsyncClient`; the selected client must match
`run()` or `arun()`. Each Agno hook call receives a fresh invocation identity.
Both adapters submit copied arguments, public schema identity, one
unknown-origin argument source, top-level provenance, and bounded framework
IDs. Agno run IDs stay in event context and are never represented as
Featherlane AI Run IDs.

AG2 attachment intentionally uses `guard_ag2(agent, ...)` after Agent
construction instead of returning an `ag2.Plugin`. AG2 applies plugin
middleware after middleware already present on the Agent, which makes that
middleware inner rather than outer. The attachment helper uses AG2's public
`insert_middleware()` API so the authorization boundary runs before existing
middleware and can prevent an unapproved tool callback from being reached.

The adapter calls the existing guarded-action helper for approval polling,
grant resume, current-policy re-evaluation, lease claim, and completion. See
[authorization-kernel.md](authorization-kernel.md) for that contract. Tool
`transform` is a non-execution result because silently executing changed
arguments would no longer be the framework's proposed call.

Missing Python side-effect metadata defaults conservatively to
`api_mutation` and emits a warning once per tool. Customers should classify
known read-only tools explicitly to avoid unnecessary approval or blocking.

If the agent exposes `reply(message, ...)`, the same decorator also preserves
the existing output-boundary behavior for `output.proposed`. It records the raw
message as an unguarded `user_turn`, records the proposed reply as an
`assistant_turn`, and links the output decision trace to that assistant turn.
Tool and output events emitted by the reply inherit the automatic Run ID. Run,
turn-event, and completion failures do not replace the guard result or the
agent's own error. Input observation never creates an authorization decision;
local tools/actions and proposed output remain the enforcement boundaries.

## Session lifecycle adapters

Session scope is explicit because a framework agent object can be shared by
unrelated users. It requires a stable external session ID and a
`registerEnd` callback. `agentId` is never used as a session key. An
explicit `client.withRun(...)` scope still takes precedence for its async
boundary.

`liveKitRun(session, options)` supplies that contract for LiveKit Agents for
Node.js without importing the framework package. It structurally subscribes to
the AgentSession `close` event, defaults the Run kind to `live_call`, and
maps close reasons as follows:

| LiveKit close evidence | Run status |
|---|---|
| `error` reason or a non-null error | `failed` |
| `job_shutdown` | `canceled` |
| participant disconnect, user initiation, task completion | `completed` |
| unknown reason | `failed` when an error exists; otherwise `completed` |

The first guarded output or local tool starts the Run. This supports tool-only
LiveKit agents that do not expose `reply()`. Concurrent first boundaries
share one create request, session close sends one terminal update, and closing
before any guarded activity creates no empty Run.

Other framework adapters may return the same session option shape only when
they can supply a deterministic end registration. Agent object lifetime,
garbage collection, and process exit are not valid lifecycle hooks.

## Metadata registration

Discovery and runtime authorization do not require a control-plane write.
Events carry operation, parameters, framework/tool identity, and schema
identity even when the tool is unregistered.

Registration is therefore off by default. `best_effort` and `strict` modes
upsert `/v1/tool-metadata` lazily before the first call to each tool.
Applications provide authoritative side-effect, reversibility, parameter-role,
approval, and sandbox metadata through `inferMetadata`. If registration is
enabled without an override, the SDK uses conservative mutation metadata.

## Limits

The adapter can enforce only a local execution function it can replace.
Provider-hosted tools, remote MCP execution hidden by a framework, closure-only
registries, dynamic tools that bypass the supported resolver, direct network
calls outside a tool, and business state not present in tool parameters remain
outside this boundary.

Unsupported entries can be surfaced through `onDiscoveryWarning`. The caller
then uses a host adapter or `withAuthorizedAction()` at the execution boundary
it owns.

The Python adapters guard local function tools and final plain-text output.
They do not guard input as a separate authorization event or guard tool results.
Agno structured output passes through unchanged with a warning; AG2 response
schema parsing can fail when a safe text replacement does not match the
customer's schema.

Final middleware and post-hooks cannot retract streaming chunks already
delivered to a consumer. Use a non-streaming call or an outer buffer that
guards the complete draft before emitting it. Provider-hosted dictionary tools
and remote execution hidden from a local hook are not intercepted.

The AG2 adapter targets the current `ag2` 1.0 object model, not classic
`autogen.ConversableAgent`. Agno approval waits inside the tool hook; it is not
a durable Agno paused run and does not create an Agno `RunRequirement`. Guard
each Agno member Agent explicitly; Team and Workflow transitive protection is
not implied.
