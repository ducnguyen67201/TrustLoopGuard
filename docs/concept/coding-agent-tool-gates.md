# Coding-agent tool gates

The coding-agent tool gate is a user-owned adapter that pauses a host tool call, submits the
proposed action to the Rust runtime, and allows execution only after a `permit`. It does not move
policy evaluation into the CLI and it does not weaken the host's own permission system.

`@trustloopguard/cli` installs the gate for Claude Code, Codex, and OpenCode:

```bash
export TLG_API_KEY="<workspace runtime key>"
npx @trustloopguard/cli install \
  --agent-id coding-agent \
  --url https://api.gettrustloop.app \
  --target claude,codex,opencode
```

The key remains in the environment of the process that launches the coding agent. The installer
does not accept a key argument, edit shell profiles, or persist the key.

## Ownership and layout

The npm CLI owns host configuration and the local bridge only. Rust still owns `GuardEvent`,
policy evaluation, approvals, grants, execution leases, receipts, and traces.

```text
platform config directory
├── trustloopguard/
│   ├── registry.json        project root → URL, agent id, host targets
│   ├── runtime/             dependency-free copied bridge
│   └── state/               pending execution-lease files
├── opencode/plugins/trustloopguard.mjs
├── ~/.claude/settings.json  managed hook groups
└── ~/.codex/hooks.json      managed hook groups
```

The registry contains no credentials. A longest-root, path-segment match selects the managed
project, so `/repo/app` does not match `/repo/application`. Calls outside a registered project
receive no TrustLoopGuard override and retain native host behavior.

Runtime and lease files live outside the guarded workspace. TrustLoopGuard-owned directories
reject symbolic links and foreign ownership, use restrictive permissions where supported, and
receive atomic writes. Host JSON is merged in place with a one-time `.tlg.bak`; unrelated hooks and
settings survive install and uninstall.

## Blocking flow

For every host-emitted before-tool event in a registered project:

1. The adapter normalizes the host tool name and arguments into a Rust-owned `GuardEvent`.
2. The bridge sends one authenticated `POST /v1/events`.
3. `deny`, `defer`, `transform`, malformed responses, transport failures, missing credentials, and
   missing tool-call ids stop execution.
4. `require_approval` polls the existing approval resource, resubmits the same invocation with the
   resulting grant, and requires an execution lease.
5. A returned lease is persisted before the adapter permits execution.
6. The matching success or failure lifecycle event consumes or cancels that exact lease.

The bridge never sends tool output to `/v1/events`. Post-tool events only reconcile the stored
lease. Completion retries retain state after failure so delivery can be retried or expire through
the authorization kernel's existing five-minute lease bound.

## Host coverage

Coverage is defined by the blocking events the host actually emits, not by the presence of a
configuration file.

| Host | Before-tool boundary | Completion boundary | Reported coverage |
|---|---|---|---|
| Claude Code | `PreToolUse` without a matcher | `PostToolUse`, `PostToolUseFailure` | `universal` for Claude-emitted calls, including MCP tools |
| OpenCode | `tool.execute.before` | `tool.execute.after`, tool-error events | `universal` for OpenCode-emitted calls |
| Codex | `PreToolUse` without a matcher | `PostToolUse`, `Stop`, `SessionEnd` | `host_emitted_only` |

Codex handlers opt into hook payloads individually. Shell, patch, MCP, and agent handlers may be
covered while read/search or newly introduced handlers remain invisible. Codex also requires the
user to review and trust hook commands. The CLI lists these exceptions and never writes trusted
hashes or recommends bypassing hook trust.

An adapter permit means only that TrustLoopGuard permits the action. A stricter Claude Code,
Codex, or OpenCode native permission can still ask or deny.

## Operations

`status` is local and read-only. `doctor` additionally checks host versions, runtime hashes, key
presence, server health, activation guidance, and coverage exceptions; a successful public health
request proves reachability, not API-key validity. A real guarded call proves the runtime key.

Uninstall removes a project/host mapping first. A shared host adapter remains while another
registered project uses it, and the copied runtime is removed only after the final registration.

MCP setup is a separate integration path. `claude mcp add`, `codex mcp add`, and OpenCode MCP
configuration expose calls routed through that MCP server; they cannot intercept native or other
MCP tools. See [hosted-mcp-access-gateway.md](hosted-mcp-access-gateway.md).
