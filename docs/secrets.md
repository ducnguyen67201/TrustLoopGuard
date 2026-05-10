# Secrets and environment variables

TrustLoopGuard uses **Doppler** as the source of truth for secrets across
both the web app (`apps/web`) and the server (`crates/tl-server`).

Each runtime also has a typed env schema that validates whatever Doppler
(or a fallback `.env*` file) injects:

| Runtime          | Schema lives in                          | Validator              |
| ---------------- | ---------------------------------------- | ---------------------- |
| `apps/web`       | `apps/web/env.ts`                        | `@t3-oss/env-nextjs` + zod |
| `crates/tl-server` | `crates/tl-server/src/config.rs`        | `figment` + `serde`    |

If the env is missing or malformed, both runtimes fail fast at startup
with an error that names the offending key.

## Doppler is optional for contributors

You can run everything without Doppler. The schemas have sensible defaults
(`http://localhost:8080` for the server URL, `0.0.0.0:8080` for the
listener) and accept local `.env*` files. Doppler is the production +
shared-team path.

## One-time setup

```bash
brew install dopplerhq/cli/doppler   # macOS; see doppler.com/install for others
doppler login                         # opens browser, scopes a CLI token
doppler setup                         # reads doppler.yaml at the repo root
```

`doppler setup` reads the committed `doppler.yaml` and links each
declared path to the corresponding Doppler project + config. It writes
per-directory `.doppler.yaml` files (gitignored) that store *your*
project + config selection.

Two Doppler projects, isolated dashboards:

| Project                   | Path              | Owns env vars                          |
| ------------------------- | ----------------- | -------------------------------------- |
| `trustloopguard-web`      | `apps/web`        | `NEXT_PUBLIC_TL_SERVER_URL`, future client envs |
| `trustloopguard-server`   | `crates/tl-server`| `TL_SERVER_LISTEN_ADDR`, `TL_SERVER_POLICY_PATHS`, future backend secrets |

## Daily commands

From the repo root:

```bash
# Without Doppler (uses .env*.local + schema defaults)
pnpm dev                 # next dev
pnpm server              # cargo run -p tl-server

# With Doppler (secrets injected at process startup)
pnpm dev:doppler         # apps/web with doppler-injected env
pnpm server:doppler      # tl-server with doppler-injected env
```

Doppler injects secrets as plain env vars before exec'ing the wrapped
process. The runtime never knows whether the value came from Doppler,
a local `.env`, or the schema default.

## Adding a new env var

1. Add the key + zod / serde validation to the relevant schema
   (`apps/web/env.ts` or `crates/tl-server/src/config.rs`).
2. Add a documented example to the matching `.env.example`.
3. Add the key to the matching Doppler project (`dev`, `stg`, `prd`
   configs as needed).
4. Reference the typed value via `env.MY_KEY` (web) or `config.my_key`
   (server). Never reach into `process.env` / `std::env::var` directly.

## Production / CI

In CI and deployed environments, supply a Doppler service token:

```bash
DOPPLER_TOKEN=$STG_SERVICE_TOKEN doppler run -- pnpm build
```

Service tokens are project + config scoped and cannot be promoted, so
the blast radius of a leaked CI token is limited to one environment.
