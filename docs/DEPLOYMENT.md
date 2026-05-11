# Deployment

`docker compose up` is the supported local deployment. This doc covers
the surface area beyond "it works on my machine."

## Environment variables

All optional. `.env.example` is the canonical reference; this section
groups them by concern.

| Variable                    | Default                  | Purpose                                                       |
| --------------------------- | ------------------------ | ------------------------------------------------------------- |
| `TL_API_KEY`                | _unset_                  | Bearer token required on `/v1/*`. Unset = open instance.      |
| `TL_CORS_ALLOWED_ORIGINS`   | `http://localhost:3000`  | Comma-separated origins permitted to call the server. `*` = any. |
| `TL_LLM_CONFIG`             | `./config/llm-routing.toml` | Tier 3 LLM routing config inside the container.            |
| `TL_ESCALATION_WEBHOOK_URL` | _unset_                  | POST'd on escalation decisions.                               |
| `RUST_LOG`                  | `info`                   | `tracing-subscriber` EnvFilter.                               |
| `NEXT_PUBLIC_TL_SERVER_URL` | `http://localhost:8080`  | URL the browser uses to reach the server.                     |
| `DATABASE_URL`              | (set by compose)         | Postgres URL. Compose wires `postgres://tl:tl@db:5432/tl`.    |

Compose-internal credentials (Postgres user/password) are intentionally
not env vars — they're hardcoded in `docker-compose.yml` because the db
container is reachable only on the internal docker network.

## Backup and restore

```bash
# Snapshot
docker compose exec -T db pg_dump -U tl tl > backup.sql

# Restore (against a freshly wiped instance)
docker compose down -v
docker compose up -d db
sleep 5  # let db settle
docker compose exec -T db psql -U tl tl < backup.sql
docker compose up -d
```

## Upgrading

```bash
git pull
docker compose pull       # if you switched to ghcr.io images (PR 4)
docker compose build      # if you're still on local builds
docker compose up -d
```

`tl-server` runs migrations idempotently on every boot, so an upgrade
that adds schema changes is a single `up -d` away.

## Production hardening

The shipped compose file is a local-trial setup, not a production one.
Before exposing it publicly:

1. **Set `TL_API_KEY`** to a long random value. Without it, every
   `/v1/*` endpoint is unauthenticated.
2. **Move the db off the same machine** or pin a strong
   `POSTGRES_PASSWORD` and stop publishing port 8080 to the host.
3. **Restrict `TL_CORS_ALLOWED_ORIGINS`** to your real frontend
   origin(s); never leave it as `*` in prod.
4. **Front it with TLS** (Caddy, Traefik, nginx, or a managed LB).
   The server itself is plaintext HTTP.
5. **Use a managed Postgres** (RDS, Cloud SQL, Supabase) for backups,
   PITR, and HA. Drop the `db` service from compose; point
   `DATABASE_URL` at the managed instance.
6. **Pin image tags.** `:latest` is convenient locally; in prod use a
   git-sha or semver tag.

## Troubleshooting

**Port already in use.** Override the published port:
```bash
docker compose up --build -d   # then edit docker-compose.yml `ports:` and re-up
```

**Server can't reach Postgres.** `docker compose logs db` should show
`database system is ready to accept connections`. If not, the volume may
be corrupted from a previous incomplete shutdown — `docker compose down -v`
wipes and starts fresh.

**Web shows "failed to fetch."** Check that
`NEXT_PUBLIC_TL_SERVER_URL` matches a URL your browser can actually
reach. Inside the docker network the server is `http://server:8080`,
but the browser runs on your machine and needs `http://localhost:8080`.

**Edited `policies/*.yaml` but nothing changed.** Compose mounts
`./policies` over the baked image copy, but the server caches policies
at boot. Run `docker compose restart server`.
