# SDK agent adapters

TypeScript agent adapters are the framework-facing layer behind
`guardAgent(agent, options)`. They discover local executable tools without
adding runtime dependencies on agent frameworks, then route those tools through
the same Rust authorization contract used by explicit SDK callers.

## Supported discovery seams

| Host shape | Discovery seam | Enforcement seam |
|---|---|---|
| OpenAI Agents JS | mutable `agent.tools` array | local function-tool `execute()` |
| LiveKit Agents for Node.js | `agent.toolCtx.tools` and `toolCtx.updateTools()` | local function-tool `execute()` |
| Mastra | `getToolsForExecution()` result | each resolved tool `execute()` |
| Compatible custom agent | `agent.tools` object map or array | each value's `execute()` |

Adapters use structural inspection only. TrustLoopGuard does not import Mastra,
OpenAI Agents JS, or LiveKit at runtime.

## Decoration flow

1. `guardAgent()` resolves one TrustLoopGuard `Client`.
2. For each `reply()`, the decorator reuses the active Run or automatically
   creates a `chat_session` Run for the configured agent identity.
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

If the agent exposes `reply(message, ...)`, the same decorator also preserves
the existing output-boundary behavior for `output.proposed`. Tool and output
events emitted by that reply inherit the automatic Run ID. Run creation and
completion failures do not replace the guard result or the agent's own error.

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
