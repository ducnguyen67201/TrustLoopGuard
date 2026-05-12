# Load test

End-to-end RPS benchmarks against a running `tl-server`. Microbenches
in `crates/tl-engine/benches/check_pipeline.rs` measure the engine
in isolation; these scripts measure the wire round-trip including
axum, JSON encode/decode, and Postgres trace persistence.

## Prerequisites

- A built `tl-server` (`cargo build --release -p tl-server`)
- [`oha`](https://github.com/hatoo/oha) — `cargo install oha` or `brew install oha`
- Optional: Postgres (set `DATABASE_URL`) — without it the server runs memory-only

## Scenarios

| File | What it exercises |
|---|---|
| `scenarios/allow.json` | Benign request → `verdict: allow`. Measures the no-block hot path. |
| `scenarios/pii_block.json` | PII in `proposed_output` → `verdict: block`. Measures the universal-detector block path. |
| `scenarios/cache_hit.json` | Same as `allow.json` but identical across all requests, so after the first the cache serves every reply. Measures the cache-hit floor. |

## Run

```sh
# Terminal 1
cargo run --release -p tl-server

# Terminal 2
./loadtest/run.sh allow      # or pii_block / cache_hit
```

Default: 1000 requests, 50 concurrent. Override:

```sh
./loadtest/run.sh allow -n 10000 -c 200
```

## What to look for

- **p50 < 5 ms** for any scenario in memory-only mode
- **p99 < 30 ms** for non-cache scenarios
- **p99 < 3 ms** for `cache_hit` (every request bypasses the tier
  pipeline after the first)

These targets are committed in `docs/concept/v0-design-decisions.md §6`.

## What's NOT here

- **Real LLM Tier 3** — would need an OpenAI API key + `config/llm-routing.toml`.
  Locking those numbers belongs in a separate dated run, not this script.
- **Postgres-loaded throughput** — set `DATABASE_URL` and re-run to compare.
