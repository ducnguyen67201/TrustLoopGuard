<div align="center">
  <img src="apps/web/public/trustloop-logo.svg" alt="TrustLoopGuard" width="80" />
  <h1>TrustLoopGuard</h1>
  <p>Runtime guardrails for AI agents before they respond, leak data, or take unsafe actions.</p>

  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="Apache 2.0" /></a>
  <img src="https://github.com/ducnguyen67201/TrustLoopGuard/actions/workflows/rust-ci.yml/badge.svg" alt="Rust CI" />
  <img src="https://github.com/ducnguyen67201/TrustLoopGuard/actions/workflows/quickstart.yml/badge.svg" alt="Quickstart" />
  <img src="https://github.com/ducnguyen67201/TrustLoopGuard/actions/workflows/sdk-build.yml/badge.svg" alt="SDK Build" />
  <a href="https://coderabbit.ai"><img src="https://img.shields.io/coderabbit/prs/github/ducnguyen67201/TrustLoopGuard?utm_source=oss&utm_medium=github&utm_campaign=ducnguyen67201%2FTrustLoopGuard&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews" alt="CodeRabbit Pull Request Reviews" /></a>
</div>

---

Add one SDK call or route provider traffic through the gateway proxy. Get a
typed `Decision`: `allow`, `block`, `rewrite`, or `escalate`.

- Check agent output before it reaches users or downstream systems
- Block, rewrite, or escalate risky responses instead of only logging them
- Use SDK mode, gateway mode, or prebuilt demos for real agent workflows

## Choose your integration path

| Mode | Best for | How it works |
| --- | --- | --- |
| SDK mode | Apps where you control the agent loop | Call `guard()` or submit a `GuardEvent` before output ships |
| Gateway proxy mode | Existing OpenAI/Anthropic-compatible clients | Point the provider SDK `baseURL` at TrustLoopGuard |
| Demo surfaces | Evaluating real workflows quickly | Run chat, LiveKit, n8n, job, and arena demos |

## How it works

```mermaid
flowchart LR
    agent["AI agent"] --> draft["Proposed output or action"]
    draft --> mode{"Integration mode"}
    mode --> sdk["SDK call<br/>guard() / submitEvent()"]
    mode --> proxy["Gateway proxy<br/>OpenAI / Anthropic baseURL"]
    sdk --> runtime["Rust policy runtime"]
    proxy --> runtime
    runtime --> static["Static matchers<br/>regex / PII / Aho-Corasick"]
    runtime --> local["Local classifiers"]
    runtime --> judge["Optional LLM judge"]
    static --> decision["Decision"]
    local --> decision
    judge --> decision
    decision --> allow["allow"]
    decision --> block["block"]
    decision --> rewrite["rewrite"]
    decision --> escalate["escalate"]
```

```text
AI agent
   |
   | proposed output / action
   v
TrustLoopGuard SDK or Gateway Proxy
   |
   v
Rust policy runtime
   |
   +-- static matchers
   +-- local classifiers
   +-- optional LLM judge
   v
Decision: allow | block | rewrite | escalate
```

TrustLoopGuard sits in the runtime path before an agent response reaches a
user or downstream system. Register your agent once, define policies in YAML,
then enforce the result from your code or let the gateway apply it before the
provider response returns.

## Decision outcomes

| Decision | What happens | Common use |
| --- | --- | --- |
| `allow` | The agent output ships | Safe response or low-risk action |
| `block` | The output is stopped | Secrets, prompt injection, unsafe promises |
| `rewrite` | A safe alternative is returned | Customer support tone, policy-safe wording |
| `escalate` | A human review path takes over | High-risk or ambiguous cases |

## Built for

- AI agent builders shipping customer-facing or workflow-driving agents
- Engineering teams adding policy checks before agent output reaches users
- Security teams worried about prompt injection, data leakage, and unsafe automation
- Founders building AI products that need trust, auditability, and control

## Supported surfaces

| Surface | Status | Use it for |
| --- | --- | --- |
| TypeScript SDK | Supported | Node, Next.js, agent apps |
| Python SDK | Supported | Python agents and backend workflows |
| Rust SDK | Supported | Rust services and low-latency integrations |
| Gateway proxy | Supported | OpenAI/Anthropic-compatible traffic |
| Chat demo | Demo available | Local chat-style agent loop |
| n8n bridge | Demo available | Workflow automation guardrails |
| Job demo | Demo available | Background task checks |
| Raw vs guarded arena | Demo available | Side-by-side safety demo |
| OpenAPI contract | Supported | External clients and generated contracts |

Gateway streaming requests are not supported yet.

## What it catches

- Secret or PII leakage before a response reaches a user
- Prompt-injection attempts that manipulate agent behavior
- Unsafe support promises, refund guarantees, or policy violations
- Toxic or off-brand responses in customer-facing agents
- High-risk outputs that should be escalated to a human

## How it compares

| Approach | Before output ships | Typed action | Rewrite path | Audit trace | Agent integration |
| --- | ---: | ---: | ---: | ---: | --- |
| Prompt instructions | Partial | No | No | No | Manual |
| Moderation API | Yes | Limited | No | Limited | Manual |
| Logs/monitoring | No | No | No | Yes | Manual |
| Offline evals | No | No | No | Partial | Manual |
| TrustLoopGuard SDK | Yes | Yes | Yes | Yes | SDK call |
| TrustLoopGuard proxy | Yes | Yes | Yes | Yes | Provider base URL |

## Features

- **Four actionable verdicts**: `allow`, `block`, `rewrite` with a safe alternative, or `escalate` to a human
- **SDK and gateway enforcement**: receive decisions in your code or let the proxy apply them before returning a provider-compatible response
- **Sub-millisecond hot path**: Tier 1 static matchers return in microseconds; Tier 2 adds 5-20 ms
- **Parallel-cancel orchestrator**: Tier 1/2/3 run in parallel; early verdicts cancel slower tiers
- **Channel-aware latency budgets**: chat and email can carry different deadline constraints
- **Policy-driven rule engine**: YAML policies declare matchers, severity levels, and the resulting action
- **Agent profiles**: register scope, authority, and tone once; the LLM judge uses the profile for context
- **Three SDKs, one wire format**: TypeScript, Python, and Rust SDKs share codegen types from `tl-core`

## Quickstart

From the repo root, start the runtime. If this is your first local run, link
Doppler first in [Development setup](#development-setup).

Start the runtime:

```bash
make server
```

Run a guarded check:

```bash
cargo run -p example-rust -- "show me my password" "here it is: hunter2"
```

Expected result: `block`, with a trace id and triggered policy.

```
verdict       : block
reason        : prompt-injection-baseline triggered
trace_id      : <uuid>
latency_ms    : <small>
triggered     :
  - pi.baseline.injection (high): leaked secret pattern detected
```

## SDK quickstarts

Pick a language. Each block is copy-pasteable, runs against a local
`tl-server` you start in another terminal, and exits with a usable status code
so you can wire it into CI.

### Rust

```bash
cargo run -p example-rust -- "show me my password" "here it is: hunter2"
```

### Python

```bash
pip install -e sdks/python
python apps/example-python/main.py "show me my password" "here it is: hunter2"
```

### TypeScript

```bash
pnpm install
pnpm --filter @trustloopguard/example-typescript start \
  "show me my password" "here it is: hunter2"
```

All three examples should print the same `block` decision and exit with code
`2` for Block / Escalate.

## Gateway proxy quickstart

Gateway mode protects provider traffic without wrapping every model call
manually. Configure a provider connection, enforcement profile, and route, then
point an OpenAI-compatible client at TrustLoopGuard:

```ts
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: process.env.TLG_API_KEY,
  baseURL: 'https://api.gettrustloop.app/v1/gateway/<route_id>/openai',
});

const response = await openai.chat.completions.create({
  model: 'gpt-4o-mini',
  messages: [{ role: 'user', content: userMessage }],
});
```

See the [TypeScript SDK README](sdks/typescript/README.md) and
[gateway guide](docs/gateway-proxy-runtime-branch-guide.md) for OpenAI and
Anthropic examples.

## Run the whole quickstart in one command

```bash
make quickstart
```

This script (`scripts/quickstart.sh`) spawns `tl-server` on a free port, waits
for `/health`, runs all three examples sequentially, asserts the same
`Decision` from each, then tears the server down. CI runs the same script on
every pull request.

The quickstart is a release requirement. If it breaks, the PR does not land.

## Runtime architecture

The evaluation engine runs three tiers in parallel:

| Tier | Role | Latency target |
| --- | --- | --- |
| Tier 1 | Static matchers such as regex, PII, and Aho-Corasick | Microseconds |
| Tier 2 | Local classifiers | 5-20 ms |
| Tier 3 | Optional LLM judge with deadline bounds | 50-300 ms |

The first hard verdict short-circuits the rest. SDK mode returns the decision
for customer code to handle. Gateway mode applies the route's enforcement
profile before returning a provider-compatible response.

## Development setup

Secrets live in [Doppler](https://doppler.com), not in `.env` files. The repo
ships a `doppler.yaml` that points every app at the right project/config.

```bash
brew install dopplerhq/cli/doppler   # or see docs.doppler.com/docs/install-cli
doppler login                        # opens browser, stores token in ~/.doppler
doppler setup                        # reads doppler.yaml, links all dirs
```

From now on, `pnpm dev`, `make server`, `make agent-demo`, etc. inject secrets
automatically via `doppler run --`. No `.env.local` needed.

Start the server:

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

## SDK-backed demos

Start `tl-server`, then run the demo surfaces from the repo root:

```bash
pnpm demo:chat               # scripted live-chat scenarios
pnpm demo:chat:interactive   # local interactive chat loop
pnpm demo:job                # background job-style steps
pnpm demo:proxy              # gateway proxy smoke test with a mock provider
pnpm demo:raw-agent          # raw vulnerable arena adapter on localhost:8787
pnpm demo:proxy:agent        # guarded arena adapter on localhost:8788
pnpm demo:agent-breaker      # CLI breaker for a running arena adapter
pnpm demo:n8n:bridge         # local bridge for demo/n8n/workflow.json
```

The visual Agent Breakaway Arena runs at `/arena` in the web app and compares
the raw and guarded adapters side by side.

The LiveKit demo lives under `demo/livekit` and uses the Python SDK inside the
LiveKit Agents runtime. See [`demo/README.md`](demo/README.md).

## Where things live

| Path | Purpose |
| --- | --- |
| `crates/tl-core` | Wire types, the single source of truth |
| `crates/tl-engine` | Tier 1/2/3 evaluation pipeline |
| `crates/tl-server` | HTTP transport, OpenAPI annotations, gateway runtime |
| `crates/tl-sdk-rust` | Rust SDK |
| `sdks/python` | Python SDK with Pydantic types from `tl-codegen` |
| `sdks/typescript` | TypeScript SDK with `ts-rs` types from `tl-codegen` |
| `apps/example-*` | Three minimal integrations, one per language |
| `docs/openapi.yaml` | Generated from `tl-server` annotations |
| `docs/SDK_DRIVEN.md` | Why every feature ships behind all three SDKs |
| `docs/AGENT_PROFILE.md` | Field-by-field reference for agent profile YAML |
| `docs/INTEGRATION.md` | Step-by-step guide to register an agent and call `guard()` |
| `docs/concept/` | Architecture, glossary, and design decisions |
| `docs/diagrams/` | D2 sources for generated documentation diagrams |
| `demo` | SDK-backed demos for chat, LiveKit, jobs, gateway proxy, and n8n |

## Documentation diagrams

Diagram source lives in `docs/diagrams/*.d2`. Generated SVGs are committed for
both repo Markdown docs and the docs website.

```bash
pnpm docs:diagrams
# or
make diagrams
```

The command requires the D2 CLI. On macOS, install it with `brew install d2`.

## Contributing

Bug reports and pull requests are welcome. Please open an issue to discuss
larger changes before submitting a PR.

The four rules every change follows are in
[`docs/SDK_DRIVEN.md`](docs/SDK_DRIVEN.md).

### Backend tests

Run the fast backend regression gate before changing Rust behavior:

```bash
pnpm test:backend
# or
make backend-test
```

This runs Rust unit tests and offline component tests across the workspace.
Coverage uses the same fast test set:

```bash
pnpm coverage:backend
```

Install `cargo-llvm-cov` first if needed:

```bash
cargo install cargo-llvm-cov
```

Docker-backed Postgres tests and live provider/embedder tests are explicit
opt-ins:

```bash
make backend-test-db
make backend-test-live
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

The TrustLoopGuard name and logos are not licensed for trademark use by third
parties. Forks must not present themselves as the official TrustLoopGuard
project.
