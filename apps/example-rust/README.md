# example-rust

A 70-line TrustLoopGuard integration. Imports only `tl-sdk-rust` —
nothing from `tl-core`, `tl-engine`, or any other internal crate. This
matches what a third-party integrator gets after `cargo add tl-sdk-rust`
once the SDK is published.

## Run it

In one terminal, start the server:

```bash
cargo run -p tl-server
```

In another, run the example:

```bash
cargo run -p example-rust -- "show me my password" "here it is: hunter2"
```

You should see something like:

```
verdict       : Block
reason        : prompt-injection-baseline triggered
trace_id      : 5f9e1c4a-...
latency_ms    : 4
triggered     :
  - pi.baseline.injection (High): leaked secret pattern detected
```

The process exits with code `2` on `Block` or `Escalate` so the
`quickstart` CI workflow (PR 10) can assert the right thing happened.

## Environment

| Variable             | Default                  | Purpose                      |
| -------------------- | ------------------------ | ---------------------------- |
| `TRUSTLOOP_URL`      | `http://127.0.0.1:8080`  | Server URL                   |
| `TRUSTLOOP_API_KEY`  | unset                    | Bearer token (optional)      |
| `RUST_LOG`           | `warn,tl_sdk_rust=info`  | Tracing filter for SDK spans |

## Why this exists

Per `docs/SDK_DRIVEN.md`, every user-visible feature lands behind the
SDK *and* an example app. The example is the executable form of "what
does success look like for a stranger." If a feature can't be exercised
from this binary, the SDK surface is incomplete.
