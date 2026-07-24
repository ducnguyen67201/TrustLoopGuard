# Demo Cookbook Test Log

Last checked: 2026-07-24 on Windows with Node.js 22.12.0, pnpm 10.23.0,
and Python.

This log covers the existing demo implementations referenced by the cookbook.
The cookbook itself is documentation-only and does not replace demo tests.

## Passing Checks

```powershell
pnpm --filter @trustloopguard/demo typecheck
pnpm --filter @trustloopguard/demo arena:check
pnpm --filter @trustloopguard/demo dispute:check
pnpm --filter @trustloopguard/demo dispute:scenarios:check
pnpm --filter @trustloopguard/demo financial-refund:check
pnpm --filter @trustloopguard/demo stripe-refund-agent:check

$env:PYTHONPYCACHEPREFIX=".local-run/cookbook-pycache"
python -m py_compile `
  demo/livekit/minimal_agent_guard.py `
  demo/livekit/guarded_healthcare_agent.py `
  demo/livekit/proxy_healthcare_agent.py
```

The TypeScript typecheck covers the agent-visibility example. The remaining
commands exercise the arena, dispute, financial-refund, Stripe refund, and
LiveKit entrypoints without requiring live provider credentials.

## Integrated Suites

The demo behavior covered by the integrated marketing suites passes. Their
umbrella commands retain Windows-specific failures in pre-existing tests:

| Command | Result | Remaining issue |
| --- | --- | --- |
| `pnpm test:contextual-demo` | 25/26 tests pass | The shared deployment test expects LF in a CRLF Dockerfile |
| `pnpm test:healthcare-demo` | 37/38 tests pass | The shared deployment test expects LF in a CRLF Dockerfile |
| `pnpm test:procurement-demo` | 27/28 tests pass | The shared deployment test expects LF in a CRLF Dockerfile |
| `$env:NODE_OPTIONS="--experimental-sqlite"; pnpm test:refund-demo` | 51/53 tests pass | The shared deployment test expects LF; one hosted test expects POSIX temporary paths |

On Node.js 22.12.0, the refund suite needs
`NODE_OPTIONS=--experimental-sqlite` because `node:sqlite` is still behind that
runtime flag. These failures are outside the documentation-only cookbook
change; the corresponding demo logic and API behavior tests pass.
