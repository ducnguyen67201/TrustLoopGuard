# SDK-driven integration

Customer agents call the TypeScript, Python, or Rust SDK directly. The SDK calls the Rust service; the Next.js dashboard is never in the runtime path.

## Default integration

The published SDK is the primary customer integration. Customers install the
package from their language registry; they do not clone this repository or run
the dashboard in their application.

TypeScript decorates the agent object once and preserves its interface:

```ts
const agent = guardAgent(createAgent(), { agentId: 'support-agent' });
const reply = await agent.reply(userMessage);
```

Python provides the equivalent decorator:

```python
@guarded(agent_id="support-agent")
async def generate_reply(message: str) -> str:
    return await agent.reply(message)
```

The Python helper remains an output-boundary adapter. The TypeScript decorator
also discovers supported local tool registries and replaces each exposed
`execute()` with the existing guarded-action path. OpenAI Agents JS tools are
read from `agent.tools`, LiveKit tools from `agent.toolCtx`, and Mastra tools
from `getToolsForExecution()`. The original tool runs at most once and only
after `permit` or a successfully resumed approval.

When `reply(message, ...)` exists, the TypeScript decorator also guards its
returned string. The output event contains the returned draft under
`action.parameters.text`; it does not include the raw user message by default.
When the caller has not opened an explicit Run, the decorator automatically
creates one `chat_session` Run per reply, stores the configured agent ID, links
the tool/output traces emitted during that reply, and completes or fails the
Run. Existing `client.withRun(...)` scopes are reused. Automatic Run
bookkeeping is best-effort and can be disabled with `run: false` without
disabling enforcement.

Long-lived frameworks can make that automatic Run session-scoped without
wrapping every turn. The caller supplies a stable external session ID and a
deterministic end registration; the first guarded boundary lazily starts one
Run, tool/output traces reuse it, and the framework end callback supplies the
terminal status. The dependency-free liveKitRun helper binds this contract to
the LiveKit AgentSession close event and defaults the Run kind to live_call.
agentId remains the registered agent identity and is never treated as a
customer-session identifier. Explicit client.withRun scopes still take
precedence for the active async boundary.

Provider-hosted tools, hidden closures, and remote execution surfaces that do
not expose a local `execute()` remain explicit integration boundaries.

Tool metadata registration is optional and lazy. Runtime tool-call visibility
does not require registration: the event still carries the operation,
parameters, framework/tool identity, and stable schema identity. Registration
adds authoritative side-effect, reversibility, parameter-role, approval, and
sandbox metadata through the existing Rust `/v1/tool-metadata` API.

The adapter contract and limitations are defined in
[sdk-agent-adapters.md](concept/sdk-agent-adapters.md).

## Runtime event flow

1. Build a `GuardEvent` with principal, operation, parameters, tool identity, sources, and provenance. Supported TypeScript agent adapters do this automatically for exposed local tools.
2. Submit it to `POST /v1/events` through the SDK.
3. Branch on `AuthorizationDecision.effect`:
   - `permit`: execute the unchanged subject.
   - `transform`: use only `transformed_value`.
   - `deny`: stop.
   - `require_approval`: wait for the common approval flow.
   - `defer`: stop until evidence or system state changes.
4. Store the `trace_id` or `receipt_id` for correlation.

Executable tool calls include stable invocation identity. Approval creates a common grant; resume sends `AuthorizationClaim { grant_id, attempt_id }`. The server re-evaluates current policy and issues a one-attempt lease before execution.

Shell helpers use `shell.action.proposed`, `shell_exec`, a stable invocation id, explicit `ToolIdentity`, and the shared `ShellActionParameters` shape. TypeScript exposes `guardShellCommand` and `withAuthorizedShellAction`; Python exposes sync/async `guard_shell_command` and `with_authorized_shell_action`. Rust re-exports `ShellActionParameters` and `ShellLanguage` for callers constructing a full `GuardEvent`. Transport retries and approval resume reuse the same invocation id and change only `action.authorization`.

## Guarded execution helpers

TypeScript exposes `withAuthorizedAction`; Python exposes sync and async `with_authorized_action`; Rust exposes `with_authorized_action`. Each helper:

- freezes or owns the proposed subject before waiting;
- polls only while approval remains pending;
- resumes with the grant and stable attempt ID;
- invokes the callback at most once and outside HTTP retry loops;
- consumes the lease after callback success;
- cancels the lease and rethrows the original callback error after callback failure;
- never re-executes the callback when completion reporting fails.

Host cancellation stops polling and prevents later execution.

## Authorization management

All SDKs expose the same control-plane operations:

- list/get/decide approvals;
- create/list/revoke grants;
- complete a lease;
- get a common authorization receipt.

Approval decisions and grant management require a privileged dashboard/internal credential. Runtime keys can read only approvals and receipts for their bound workspace, environment, and principal, and can complete only their own leases.

## Saved grants

A user-intent grant can be created before an action exists. A reviewer grant is created from a hash-bound approval envelope. Both use the same typed scopes and matcher.

```ts
const grant = await client.createGrant({
  principal_id: 'refund-bot',
  domain: 'financial',
  capability: 'financial:issue_refund',
  requirement_ids: ['financial:refund-controls:grant_required'],
  scope: {
    scope_type: 'financial',
    scope: {
      action_kinds: ['refund'],
      operation: 'issue_refund',
      rail: 'payment_http',
      currency: 'USD',
      maximum_amount_minor: 10_000n,
      counterparties: ['cust_123'],
      x402_hosts: [],
      x402_resources: [],
      x402_networks: [],
      x402_assets: [],
      x402_payees: [],
      required_preconditions: [],
    },
  },
});
```

The caller must explicitly claim that grant on evaluation and execution. A grant satisfies only matching `require_approval` requirements. Current hard policy, missing evidence, eligibility, provider preconditions, and live budgets still run.

## Financial actions

`verifyAction`/`guardPayment` submits a typed financial action and returns `FinancialActionRecord` with separate authorization and execution fields. `executeAction` accepts an optional authorization claim and stable attempt ID so an action can be reviewed and executed later without weakening the recheck boundary.

The common `AuthorizationReceipt` proves why execution was permitted. `FinancialReceipt` separately records ledger and provider execution proof and links to the authorization receipt.

## MCP proxy

`apps/mcp-proxy` uses the TypeScript guarded-action helper. It mirrors the downstream tool schema, submits exact parameters, waits for authorization, calls the downstream MCP server once after `permit`, and completes or cancels the lease. A timeout, cancellation, denial, deferral, changed schema, or changed parameters never reaches the downstream tool.

See [authorization-kernel.md](concept/authorization-kernel.md), [event-engine.md](concept/event-engine.md), [command-safety.md](concept/command-safety.md), and [financial-authorization.md](concept/financial-authorization.md).
