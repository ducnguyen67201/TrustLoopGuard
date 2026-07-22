# TrustLoopGuard Cookbook

Small examples that make one TrustLoopGuard concept visible at a time. Each
folder is designed to be read, copied, changed, and run independently.

## Start Here

[`00_quickstart`](00_quickstart/README.md) protects a deterministic support
agent at its output boundary. It shows the unsafe draft, the decision returned
by Rust, and the only reply the application is allowed to deliver.

## Which Example Directory Should I Use?

| Directory | Use it for |
| --- | --- |
| `cookbook/` | Small learning examples with one concept per folder |
| `demo/` | Complete product scenarios with agents, providers, and UI surfaces |
| `recipes/` | Canonical SDK snippets synchronized into other documentation |

## Cookbook Standard

Every runnable folder includes:

- a `README.md` with the concept, prerequisites, setup, and run commands;
- a `TEST_LOG.md` with the last verified commands and results;
- an offline contract test for the behavior that must not regress.

Run every cookbook check from the repository root:

```bash
pnpm --filter @trustloopguard/cookbook test
pnpm --filter @trustloopguard/cookbook typecheck
```
