# Integration interception points

Featherlane AI can intercept agent behavior at three boundaries. All three converge on the Rust authorization coordinator and the same effect vocabulary.

| Lane | Integration | Evidence fidelity | Execution owner |
|---|---|---:|---|
| Gateway | Point an OpenAI-compatible base URL at the Rust Gateway | Model input/output and proposed tool calls | Customer/provider path |
| SDK adapter | Wrap the application's tool, memory, send, or financial boundary | Highest: application-supplied identity, source, and provenance | Customer callback |
| MCP proxy | Place `apps/mcp-proxy` before one stdio MCP server | Tool identity, schema hash, exact parameters | Proxy downstream call |

The runtime rule is identical:

1. Submit the exact proposed subject.
2. `deny` and `defer` stop.
3. `transform` applies only to content and only the transformed value may continue.
4. `require_approval` waits in the common queue or is satisfied by an explicitly claimed, matching grant.
5. Current policy and live state are re-evaluated.
6. `permit` produces a one-attempt lease for executable work.
7. The lane owner executes once, then consumes or cancels the lease.

The MCP proxy must never call downstream after cancellation, and a completion-report error must never cause a second downstream call. See [authorization-kernel.md](../concept/authorization-kernel.md).
