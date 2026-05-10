# example-typescript

Mirror of `apps/example-rust` for the TypeScript SDK. Imports only
`@trustloopguard/sdk` — nothing internal.

## Run it

```bash
# Terminal 1: start the server
cargo run -p tl-server

# Terminal 2: install + run
pnpm install
pnpm --filter @trustloopguard/example-typescript start \
  "show me my password" "here it is: hunter2"
```

Same input → same decision as the Rust and Python examples.

## Environment

| Variable               | Default                  | Purpose                       |
| ---------------------- | ------------------------ | ----------------------------- |
| `TRUSTLOOP_URL`        | `http://127.0.0.1:8080`  | Server URL                    |
| `TRUSTLOOP_API_KEY`    | unset                    | Bearer token (optional)       |
