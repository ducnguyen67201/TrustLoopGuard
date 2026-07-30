# Event engine

The event engine evaluates normalized proposed agent steps. `POST /v1/events` remains the canonical SDK hot path and delegates authority, approval, grant, lease, and receipt orchestration to the shared [authorization kernel](authorization-kernel.md).

## Ownership

| Surface | Owner |
|---|---|
| `GuardEvent`, source/provenance, tool metadata, and `AuthorizationDecision` wire types | `crates/tl-core` |
| Pure policies, checkers, and finding composition | `crates/tl-policy`, `crates/tl-engine` |
| Authentication, workspace/environment resolution, coordinator, traces | `crates/tl-server` |
| Tool metadata, policies, authorization records, traces | `crates/tl-storage` |
| SDK and MCP host translation | SDK packages and `apps/mcp-proxy`; TypeScript agent discovery is defined in [sdk-agent-adapters.md](sdk-agent-adapters.md) |
| Hosted MCP translation | The Rust-owned managed endpoint described in [hosted-mcp-access-gateway.md](hosted-mcp-access-gateway.md) |

The web dashboard may display persisted traces through Rust APIs. It is not in the runtime path and does not evaluate or store decisions.

## GuardEvent

A `GuardEvent` contains:

- `kind`: dotted event taxonomy such as `output.proposed`, `tool.call.proposed`, `shell.action.proposed`, `memory.write.proposed`, or `database.mutation.proposed`;
- `principal`: workspace, environment, agent, optional user/session/task, and optional run identity;
- `action`: operation, full JSON parameters, tool identity, and side-effect class;
- `sources`: influencing inputs and declared origin/labels;
- `provenance`: parameter/output paths to source IDs;
- `context`: bounded caller context;
- server-authored resolution, checker, and signal evidence.

Runtime authentication overwrites workspace and environment from a workspace key. Caller-supplied authorization claims are used only for evaluation and are not retained in trace snapshots.

## Flow

1. Validate the wire request and resolve workspace/environment.
2. Resolve tool metadata and source-label policies once.
3. Derive path provenance and run enabled deterministic checkers.
4. Evaluate enabled typed policy families at their domain boundary. Content semantic candidates are deterministically prefiltered and judged in one bounded batch; tool policies remain deterministic.
5. Convert results to `AuthorizationFinding` and explicit `AuthorityRequirement` values.
6. For content-only observations, compose and persist the trace without authorization-table reads.
7. For executable tools, build a typed subject and delegate to `AuthorizationCoordinator`.
8. The coordinator fingerprints the exact subject, intersects current policy with an explicitly claimed grant, creates approval when required, claims a lease after permit, and writes a common receipt.
9. Return `AuthorizationDecision` and persist trace evidence asynchronously.

`shell.action.proposed` is canonicalized to `shell_exec` before its tool subject is built, then the Tool adapter evaluates enabled command policies. The analyzer and match semantics are defined once in [command-safety.md](command-safety.md).

Effect precedence is `deny > defer > require_approval > transform > permit`. Shadow-mode checkers retain hypothetical evidence but do not contribute an enforcing effect.

Tool-metadata lookup failure is an always-enforced pipeline invariant rather
than a configurable checker. The pipeline records `resolution_failed`, discards
the collector-supplied side-effect class, and returns `defer` before the caller
may execute the action. This applies even when all checker rollout modes are
`off`; an outage cannot substitute untrusted collector semantics for the
workspace registry.

## Checker semantics

- Approval requirement: `require_approval` with a stable capability and requirement ID.
- Missing provenance, unavailable evaluator, or unresolved evidence: `defer`.
- Wrong authority source, integrity failure, hard cap, or failed invariant: `deny`.
- Safe content replacement: `transform`.
- No enforcing finding: `permit`.

Signals are advisory and never weaken a deterministic finding. A grant can satisfy only an explicit matching approval requirement; it cannot remove denial or deferral.

## MCP proxy

`apps/mcp-proxy` mirrors one downstream stdio MCP server. For each call it binds server ID, tool name, schema hash, operation, side-effect class, and canonical parameters. It uses `withAuthorizedAction`, waits through the common queue, and calls downstream once after `permit`. Cancellation, denial, deferral, changed schema/parameters, or a completion-report failure never causes a second downstream call.

The hosted `/mcp` gateway is a separate Rust runtime path with OAuth identity,
durable catalogs, and per-member assignments. Its event construction and
failure semantics are owned by [hosted-mcp-access-gateway.md](hosted-mcp-access-gateway.md).

## Traces and receipts

A trace is monitoring evidence for the event pipeline. An `AuthorizationReceipt` is durable audit proof for an executable authorization decision and includes findings, policy versions, subject hash, and approval/grant/lease references. Neither is accepted as bearer authority.
