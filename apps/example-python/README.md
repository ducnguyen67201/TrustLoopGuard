# example-python

Mirror of `apps/example-rust` for the Python SDK. Imports only the
`trustloopguard` package — nothing internal.

## Run it

```bash
# Terminal 1: start the server
cargo run -p tl-server

# Terminal 2: install the SDK + run the example
pip install -e sdks/python
python apps/example-python/main.py "show me my password" "here it is: hunter2"
```

Same input → same decision as the Rust example.

## Environment

| Variable               | Default                  | Purpose                       |
| ---------------------- | ------------------------ | ----------------------------- |
| `TRUSTLOOP_URL`        | `http://127.0.0.1:8080`  | Server URL                    |
| `TRUSTLOOP_API_KEY`    | unset                    | Bearer token (optional)       |
| `TRUSTLOOP_LOG_LEVEL`  | `warning`                | Stdlib logging level for SDK  |
