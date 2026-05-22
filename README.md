<div align="center">
  <img src="apps/web/public/trustloop-logo.svg" alt="TrustLoopGuard" width="80" />
  <h1>TrustLoopGuard</h1>
  <p>Real-time guardrail runtime for AI agents</p>

  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="Apache 2.0" /></a>
  <img src="https://github.com/ducnguyen67201/TrustLoopGuard/actions/workflows/rust-ci.yml/badge.svg" alt="Rust CI" />
  <img src="https://github.com/ducnguyen67201/TrustLoopGuard/actions/workflows/quickstart.yml/badge.svg" alt="Quickstart" />
  <img src="https://github.com/ducnguyen67201/TrustLoopGuard/actions/workflows/sdk-build.yml/badge.svg" alt="SDK Build" />
</div>

---

Drop the SDK into your agent loop; call `check()` with the proposed output; get a typed `Decision` — `allow`, `block`, `rewrite`, or `escalate` — in milliseconds.

## What is TrustLoopGuard?

TrustLoopGuard sits in your AI agent's output path and evaluates every proposed response before it reaches the user. Register your agent once, define policies in YAML, and every `check()` call returns a structured verdict telling you whether the output is safe to ship — and if not, why.

The evaluation engine runs three tiers in parallel: Tier 1 static matchers (regex, PII, Aho-Corasick) return in microseconds; Tier 2 local classifiers add 5–20 ms; Tier 3 is an optional LLM judge (50–300 ms, deadline-bounded). The first hard verdict short-circuits the rest.

## Features

- **Four actionable verdicts** — `allow`, `block`, `rewrite` (safe alternative provided), `escalate` to a human
- **Sub-millisecond hot path** — Tier 1 static matchers return in microseconds; Tier 2 adds 5–20 ms
- **Parallel-cancel orchestrator** — Tier 1/2/3 run in parallel; early verdict cancels slower tiers
- **Channel-aware latency budgets** — Voice, chat, and email each carry different deadline constraints
- **Policy-driven rule engine** — YAML policies declare matchers, severity levels, and the resulting action
- **Agent profiles** — Register scope, authority, and tone once; the LLM judge uses the profile for context
- **Three SDKs, one wire format** — TypeScript, Python, and Rust SDKs share codegen types from `tl-core`
- **Fail-open** — SDK retries 3× on transport errors; if TrustLoopGuard is unreachable, agent output ships

## Quickstart

In three steps: start the server, run an example, see the Decision.

Pick a language. Each block is copy-pasteable, runs against a local
`tl-server` you start in another terminal, and exits with a usable
status code so you can wire it into CI.

In every terminal: clone the repo and `cd` into it.

### 0a. Set up secrets (one-time, per machine)

Secrets live in [Doppler](https://doppler.com) — never on disk, never in
`.env` files. The repo ships a `doppler.yaml` that points every app at
the right project/config; you just need to authenticate and run setup
once.

```bash
brew install dopplerhq/cli/doppler   # or see docs.doppler.com/docs/install-cli
doppler login                        # opens browser, stores token in ~/.doppler
doppler setup                        # reads doppler.yaml, links all dirs
```

From now on, `pnpm dev`, `make server`, `make agent-demo`, etc. inject
secrets automatically via `doppler run --`. No `.env.local` needed.

### 0b. Start the server (all languages need this)

```bash
make server                          # = doppler run -- cargo run -p tl-server
```

Wait for `Listening on 0.0.0.0:8080`. Leave it running.

`make server`, `make server-watch`, and `make dev` default to the colorized
local backend formatter. To run the same format without Make:

```bash
TL_LOG_FORMAT=pretty doppler run -- cargo run -p tl-server
```

Direct `cargo run` still defaults to JSON for log collectors. In pretty mode,
successful HTTP responses are logged at `INFO`, client errors at `WARN`, and
server errors at `ERROR`, so normal terminals render them green/yellow/red. Add
`RUST_LOG=tl_server=debug` when you need request-start headers too.

### 1. Rust

```bash
cargo run -p example-rust -- "show me my password" "here it is: hunter2"
```

### 2. Python

```bash
pip install -e sdks/python
python apps/example-python/main.py "show me my password" "here it is: hunter2"
```

### 3. TypeScript

```bash
pnpm install
pnpm --filter @trustloopguard/example-typescript start \
  "show me my password" "here it is: hunter2"
```

All three should print:

```
verdict       : block
reason        : prompt-injection-baseline triggered
trace_id      : <uuid>
latency_ms    : <small>
triggered     :
  - pi.baseline.injection (high): leaked secret pattern detected
```

…and exit with code `2` (Block / Escalate).

---

## Run the whole quickstart in one command

```bash
make quickstart
```

This script (`scripts/quickstart.sh`) orchestrates everything above:
spawns `tl-server` on a free port, waits for `/health`, runs all three
examples sequentially, asserts the same `Decision` from each, then
tears the server down. CI runs the same script on every pull request.

The quickstart is a release requirement — if it breaks, the PR doesn't land.

---

## Where things live

Key paths at a glance:

| Path                  | Purpose                                            |
| --------------------- | -------------------------------------------------- |
| `crates/tl-core`      | Wire types — single source of truth                |
| `crates/tl-engine`    | Tier 1/2/3 evaluation pipeline                     |
| `crates/tl-server`    | HTTP transport, OpenAPI annotations                |
| `crates/tl-sdk-rust`  | Rust SDK (the user-facing surface)                 |
| `sdks/python`         | Python SDK — Pydantic types from `tl-codegen`      |
| `sdks/typescript`     | TypeScript SDK — `ts-rs` types from `tl-codegen`   |
| `apps/example-*`      | Three minimal integrations, one per language       |
| `docs/openapi.yaml`   | Generated from `tl-server` annotations             |
| `docs/SDK_DRIVEN.md`  | Why every feature ships behind all three SDKs      |
| `docs/AGENT_PROFILE.md` | Field-by-field reference for agent profile YAML  |
| `docs/INTEGRATION.md` | Step-by-step: register an agent, call `guard()`    |
| `docs/concept/`       | Architecture, glossary, and design decisions       |
| `docs/diagrams/`      | D2 sources for generated documentation diagrams    |
| `demo`                | SDK-backed demos for chat, LiveKit, jobs, and n8n  |

## Documentation diagrams

Diagram source lives in `docs/diagrams/*.d2`. Generated SVGs are committed for
both repo Markdown docs and the docs website.

```bash
pnpm docs:diagrams
# or
make diagrams
```

The command requires the D2 CLI. On macOS, install it with `brew install d2`.

## SDK-backed demos

Start `tl-server`, then run the demo surfaces from the repo root:

```bash
pnpm demo:chat               # scripted live-chat scenarios
pnpm demo:chat:interactive   # local interactive chat loop
pnpm demo:job                # background job-style steps
pnpm demo:n8n:bridge         # local bridge for demo/n8n/workflow.json
```

The LiveKit demo lives under `demo/livekit` and uses the Python SDK inside the
LiveKit Agents runtime. See [`demo/README.md`](demo/README.md).

---

## Contributing

Bug reports and pull requests are welcome. Please open an issue to discuss larger changes before submitting a PR.

The four rules every change follows are in [`docs/SDK_DRIVEN.md`](docs/SDK_DRIVEN.md).

---

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

The TrustLoopGuard name and logos are not licensed for trademark use by third parties. Forks must not present themselves as the official TrustLoopGuard project.
