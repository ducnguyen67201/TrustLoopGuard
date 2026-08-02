# Featherlane AI CLI

Install Featherlane AI as a blocking tool-call gate for Claude Code, Codex, and OpenCode.

```bash
export FEATHERLANE_AI_API_KEY="<your workspace runtime key>"
npx @featherlane-ai/cli install \
  --agent-id coding-agent \
  --url https://api.featherlane.ai \
  --target claude,codex,opencode
```

Create the workspace runtime key with `principal_id` set to the same value passed to `--agent-id`
(`coding-agent` above). This binding lets the runtime key poll approvals and reconcile execution
leases for that agent without widening its scope.

The key is read from the environment and is never accepted as a command argument or written to
disk. The installer writes a user-owned runtime and project registry under the platform config
directory, then safely merges user-level host hooks.

## Commands

```bash
npx @featherlane-ai/cli install --agent-id coding-agent --target auto
npx @featherlane-ai/cli status
npx @featherlane-ai/cli doctor
npx @featherlane-ai/cli uninstall --target claude
npx @featherlane-ai/cli uninstall --all
```

`status` reports configuration without making network requests. `doctor` also checks the host
versions, runtime integrity, environment key presence, and Featherlane AI health endpoint.

Claude Code and OpenCode gate every tool event emitted through their blocking before-tool
extension points. Codex coverage is limited to tool handlers for which the installed Codex version
emits `PreToolUse`; `doctor` labels this as `host_emitted_only` and lists known exceptions.

MCP registration is separate. `claude mcp add`, `codex mcp add`, or an OpenCode MCP entry exposes an
MCP server but does not intercept native or third-party tool calls.

## Verify

- Restart each configured host after installation.
- In Claude Code, open `/hooks` and confirm the three Featherlane AI handlers.
- In Codex, open `/hooks`, review the command, and approve hook trust.
- In OpenCode, run a harmless tool after restart and confirm a trace appears.

Managed projects fail closed when the key, server, response, approval, or lease state is invalid.
Unregistered projects retain the host's native behavior.
