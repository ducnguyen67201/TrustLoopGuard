# TrustLoopGuard

Real-time guardrail runtime for AI agents. Drop the SDK into your agent
loop; get a `Decision` back in milliseconds telling you whether the
proposed output is safe to deliver.

> **Status: pre-1.0.** Wire formats are stable across `/v1/*`; SDK
> surfaces follow `docs/SDK_DRIVEN.md` discipline. See the 21-PR engine
> roadmap for what's still landing.

---

## Run with Docker (no toolchain required)

If you just want to try it — no Rust, no pnpm, no Postgres install:

```bash
git clone https://github.com/ducnguyen67201/TrustLoopGuard
cd TrustLoopGuard
docker compose up
```

Open <http://localhost:3000> for the dashboard. The server is at
<http://localhost:8080> (try `curl http://localhost:8080/health`).

What this brings up:

| Service | Port | What it is                                       |
| ------- | ---- | ------------------------------------------------ |
| `web`   | 3000 | Next.js dashboard                                |
| `server`| 8080 | `tl-server` with `/v1/check`, `/v1/agents`, …    |
| `db`    | —    | Postgres 16, schema auto-migrated, data persisted |

Stop the stack: `docker compose down`. Wipe data: `docker compose down -v`.
Edit `policies/*.yaml` and `docker compose restart server` to reload them.

No `.env` file is required. See `.env.example` for the optional knobs
(API key, custom LLM routing, escalation webhook, log level).

---

## Quickstart (without Docker)

Pick a language. Each block is copy-pasteable, runs against a local
`tl-server` you start in another terminal, and exits with a usable
status code so you can wire it into CI.

In every terminal: clone the repo and `cd` into it.

### 0. Start the server (all languages need this)

```bash
cargo run -p tl-server
```

Wait for `Listening on 0.0.0.0:8080`. Leave it running.

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
tears the server down. CI runs the same script via
`.github/workflows/quickstart.yml` (PR 10).

If the script breaks, that's a release blocker — see
`docs/SDK_DRIVEN.md` rule 3.

---

## Where things live

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

---

## Repo philosophy

TrustLoopGuard is open-source. Adoption is "stranger drops the SDK in
and ships." That makes the SDK surface the contract — not the engine
internals. See [`docs/SDK_DRIVEN.md`](docs/SDK_DRIVEN.md) for the four
rules every PR follows.

---

## License

Apache-2.0. See `LICENSE`.
