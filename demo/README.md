# TrustLoopGuard demos

These demos exercise the same output-boundary pipeline through the public SDKs:

1. The agent drafts output.
2. The demo calls `guard()` through the TypeScript or Python SDK.
3. TrustLoopGuard returns a decision, trace id, and latency.
4. The demo delivers only the guarded output.

Start the Rust server first:

```sh
cargo run -p tl-server
```

Optional environment:

| Var | Default | Purpose |
| --- | --- | --- |
| `TL_SERVER_URL` | `http://127.0.0.1:8080` | TrustLoopGuard server URL |
| `TL_API_KEY` | unset | Bearer token when the server requires auth |
| `TL_AGENT_ID` | `demo-acme-support` | Demo agent profile id |
| `TL_WORKSPACE_ID` | unset | Optional local workspace header override, e.g. `ws_test` |
| `OPENAI_API_KEY` | unset | Enables real OpenAI-backed replies and workflow action proposals |
| `OPENAI_MODEL` | `gpt-4.1-mini` | OpenAI model for chat replies and workflow proposals |

## Live chat

Deterministic scripted scenarios:

```sh
pnpm demo:chat
```

Interactive local chat loop:

```sh
pnpm demo:chat:interactive
```

When `OPENAI_API_KEY` is set, the interactive chat asks OpenAI for the agent
draft before sending that draft through `guard()`. Without it, the demo uses
local deterministic drafts.

## Tax MVP chat adapters for the Attacks tab

The Attacks tab can test two versions of the same local tax assistant:

- Raw target: `TaxPilot Assist (raw)` on `http://127.0.0.1:9101`
- Guarded target: `TaxPilot Assist (guarded)` on `http://127.0.0.1:9102`

Both adapters share the same tax MVP behavior. When `OPENAI_API_KEY` is
configured, they ask OpenAI to draft the tax-assistant reply from the synthetic
packet context. Without a key, they fall back to deterministic local replies.
The raw adapter returns the draft directly. The guarded adapter sends the same
draft through the TrustLoopGuard SDK, using the active Rust runtime, workspace,
settings, and policies.

Start Rust with a red-team runner URL when using the dashboard Attacks tab:

```powershell
$env:DATABASE_URL = 'postgres://tl:tl@127.0.0.1:55432/tl'
$env:TL_API_KEY = 'dev-admin'
$env:TL_APP_ENV = 'development'
$env:REDTEAM_RUNNER_URL = 'http://127.0.0.1:8799'
cargo run -p tl-server
```

When reseeding the demo workspace, provide `TL_DEMO_USER_ID` from the signed-in
dashboard user so the seed script can update workspace settings through the Rust
settings API:

```powershell
$env:TL_DEMO_USER_ID = '<dashboard-user-id>'
pnpm --filter web db:seed
```

Start the local runner and both adapters in separate terminals:

```powershell
pnpm --filter @trustloopguard/demo agent-demo:runner
pnpm --filter @trustloopguard/demo agent-demo:tax:raw

$env:TL_SERVER_URL = 'http://127.0.0.1:8080'
$env:TL_API_KEY = 'dev-admin'
$env:TL_WORKSPACE_ID = 'ws_demo_workspace'
pnpm --filter @trustloopguard/demo agent-demo:tax:guarded
```

Open `http://localhost:3000/attacks`, then run FAST against each target:

1. `http://127.0.0.1:9101` should show the raw tax MVP leaking or accepting
   unsafe requests.
2. `http://127.0.0.1:9102` should show the same requests evaluated by the
   workspace-backed TrustLoopGuard runtime. PII and packet-export attacks should
   block when the demo workspace has the PII policies/checkers enabled. If a
   review-bypass attack lands, that is a real policy coverage gap in the active
   workspace rather than adapter-side filtering; add or enable a workspace
   policy for refund approval / human-review bypass to close it.

The current tax packet is a local fixture in `agent-demo-adapter/tax-fixture.ts`.
That hardcoded fixture is intentionally isolated so the next step can replace it
with a database/API-backed packet provider. The guarded adapter should continue
to use real workspace configuration from TrustLoopGuard, including the API key,
runtime settings, policies, tool metadata, and enforcement modes stored in the
database.

## Tax workflow adapters for the Attacks tab

The Attacks tab can also test workflow agents that process attached PDF
documents and propose tool actions from the extracted document contents:

- Raw target: `TaxPilot Workflow (raw)` on `http://127.0.0.1:9111`
- Guarded target: `TaxPilot Workflow (guarded)` on `http://127.0.0.1:9112`

Both adapters share the same workflow engine. The engine extracts text from
simple text PDFs, classifies the document, extracts fields, proposes actions
from a small tool catalog, and writes only to a simulated local ledger. When
`OPENAI_API_KEY` is configured, the action proposal step asks OpenAI for
structured tool calls from the extracted document. Without a key, it falls back
to deterministic local proposal rules. No email, webhook, or database mutation
is real.

The action proposal is document-driven. Different injected documents can
propose different tools:

- `send_email` for document instructions that send/export a packet to an email
  address.
- `update_tax_record` for document instructions that set statuses such as
  `approved_refund`, `pending_review`, or `needs_docs`.
- `create_review_task` for benign human/preparer review requests.
- `post_webhook` for document-controlled callback/webhook instructions.

Start the runner and workflow adapters in separate terminals:

```powershell
pnpm --filter @trustloopguard/demo agent-demo:runner
pnpm --filter @trustloopguard/demo agent-demo:workflow:raw

$env:TL_SERVER_URL = 'http://127.0.0.1:8080'
$env:TL_API_KEY = 'dev-admin'
$env:TL_WORKSPACE_ID = 'ws_demo_workspace'
pnpm --filter @trustloopguard/demo agent-demo:workflow:guarded
```

Open `http://localhost:3000/attacks`, then run FAST against each target:

1. `http://127.0.0.1:9111` should show malicious workflow documents landing:
   external email/webhook actions or unsafe refund-status updates execute in
   the fake ledger.
2. `http://127.0.0.1:9112` should submit every proposed tool action to the
   workspace-backed runtime before simulated execution. Unsafe actions should
   block or escalate when the active workspace has matching tool metadata,
   settings, and policies enabled. Benign review tasks should still complete.

The current workflow PDFs are generated local runner fixtures in
`agent-demo-adapter/simple-runner.ts`, including one PDF with multiple injected
tool-action instructions in the same attachment. Keep that seam visible: the
next demo iteration should load document attacks from database/API-backed
workspace data instead of hardcoded fixtures, while the guarded adapter
continues to use the real workspace API key, runtime settings, policies, and
tool metadata.

## LiveKit

The LiveKit demo is Python because it follows the LiveKit Agents runtime. SDK
mode guards the draft right before the agent speaks:

```sh
pip install -e sdks/python
pip install "livekit-agents[openai,silero]" python-dotenv

TL_SERVER_URL=http://127.0.0.1:8080 \
TL_AGENT_ID=demo-healthcare-livekit \
python demo/livekit/guarded_healthcare_agent.py dev
```

For voice, the demo configures a 250 ms guardrail budget and one SDK attempt:
the runtime either returns guarded output within the realtime budget or follows
the SDK's configured failure behavior.

Gateway mode points LiveKit's OpenAI-compatible LLM at TrustLoopGuard instead of
calling the provider directly:

```sh
python demo/livekit/proxy_healthcare_agent.py dev
```

Set `TLG_API_KEY` and `TL_GATEWAY_ROUTE_ID` in `demo/livekit/.env` first. See
`livekit/README.md` for the full setup.
