# Red-Team Report Sharing

A **vulnerability report** is a presentation-ready view of a completed
[red-team job](redteam-dispatch.md) — "what your agent is exposed to" — optionally
comparing two runs of the same agent (a vulnerable baseline vs. a hardened re-run).
A **share token** turns one report into a public link a prospect can open without a
dashboard account: the sales/demo artifact that proves both the problem and the fix.

Rust owns the report data and the durable share token; the web layer only renders
the PDF. Severity classification ("what counts as a Critical finding") is product
semantics and lives in Rust, not in the renderer.

## Two surfaces

- **In-dashboard report** — `GET /v1/redteam/jobs/{id}/report` (authenticated) returns
  the structured [`RedteamReportPayload`] for a job, optionally `?compare={job_id}`.
  Used by the dashboard to render a report inline. When the
  `trajectory_diagnostic` LLM route is configured, this authenticated read can
  enrich deterministic trajectory diagnostics with model-authored root-cause
  wording; failed or absent LLM output falls back to the deterministic diagnostic.
- **Shareable report** — a workspace member mints a token
  (`POST /v1/redteam/reports`); anyone with the link reads a deterministic report
  payload unauthenticated (`GET /v1/redteam/reports/{token}`). The dashboard
  renders that payload as a branded PDF at `/r/{token}`.

## The report payload

`build_report` (`crates/tl-server/src/redteam/report.rs`) is a pure function over a
job summary and its attack sessions. It classifies each session into a finding, rolls up
aggregates, and derives an overall risk level:

- **Findings** — each attack becomes a `RedteamReportFinding` with a `category`
  (keyword-derived, e.g. `credential_disclosure`) and a `ReportSeverity`. Only
  *landed* attacks are live vulnerabilities; blocked and clean control cases are
  `info`, an errored attempt is `low`. Landed credential/prompt-leak attacks are
  `critical`. Evidence (a truncated reply excerpt) is attached only to landed
  findings.
- **Aggregates** — `total`, `attacks` (non-control denominator), `landed`, `blocked`,
  `clean`, `errored`, `success_rate` (`landed / attacks`), and `risk_level` (the worst
  landed severity, or `info` when nothing landed).
- **Comparison** (optional) — pairs the two runs' attacks by name and labels each
  `fixed` (landed → blocked), `still_vulnerable` (landed → landed), `regressed`
  (blocked → landed), or `unchanged`, plus `delta_points` (the percentage-point change
  in success rate). This is the before/after story for the
  [hardening loop](redteam-dispatch.md#hardening-loop).
- **Trajectory diagnostics** — landed findings carry optional
  `RedteamTrajectoryDiagnostic` evidence. The pure builder derives a
  deterministic diagnostic from checker findings, or a low-confidence
  `semantic_output` fallback when the attack landed without structured checker
  evidence. Authenticated report reads may enrich that diagnostic through the
  configured `trajectory_diagnostic` LLM route, but the LLM never changes the
  finding severity, outcome, evidence ids, or runtime verdict.

A comparison is only allowed between two **complete** jobs that target the **same
agent** (matched by `agent_id`, falling back to identical `target`).

## Share tokens

A share is a capability, not a session. The token is the sole bearer credential for
the public endpoint.

- **Minting** (`POST /v1/redteam/reports`, authenticated) validates that the job — and
  any compare job — is complete and same-agent, then stores a row keyed by a
  high-entropy, `rpt_`-prefixed token (32 bytes of OS randomness, URL-safe). It returns
  the token and a relative `path` (`/r/{token}`); the dashboard composes the absolute
  URL from its own origin, so the server never has to know its public hostname.
- **Reading** (`GET /v1/redteam/reports/{token}`, public) resolves the token, fetches
  the job(s) **scoped to the token's stored workspace** — never the request — so a
  token can never reach another workspace's data, and returns the payload. The read is
  rate-limited **per token** (a fixed window, `429` when exceeded): unknown tokens 404
  cheaply before the limiter, so the limiter map stays bounded to live shares and any
  single shared link can't be hammered into an expensive report build.
- **Expiry & revocation** — links default to a 30-day expiry (caller-settable, capped
  at 90 days) and can be revoked early (`POST /v1/redteam/reports/{token}/revoke`).
  A missing, expired, or revoked token all return `404`, so the public endpoint is not
  a token-validity oracle.

Because a report can contain attack evidence (including secrets a vulnerable agent
leaked), the link is treated as sensitive: unguessable token, default expiry,
revocation, and `noindex` on the rendered PDF. `build_report` keeps a redaction seam
for customer (non-demo) reports.

## API

| Method & path | Auth | Purpose |
|---|---|---|
| `GET /v1/redteam/jobs/{id}/report` | workspace | Structured report for a job (`?compare={id}` for a same-agent diff) |
| `POST /v1/redteam/reports` | workspace | Mint a share token for a complete job (`RedteamReportShare`) |
| `GET /v1/redteam/reports/{token}` | **public** | Read the report payload by token (`RedteamReportPayload`) |
| `POST /v1/redteam/reports/{token}/revoke` | workspace | Revoke a share; the public read then `404`s |

Wire types live in `crates/tl-core/src/redteam.rs` (`RedteamReportPayload`,
`RedteamReportFinding`, `RedteamReportAggregates`, `RedteamReportComparison`,
`RedteamComparedAttack`, `ReportSeverity`, `ComparedAttackStatus`, `CreateReportRequest`,
`RedteamReportShare`) and are reflected in `docs/openapi.yaml`.

## Storage

One workspace-scoped table in `crates/tl-storage` (`RedteamReportShareRepo`):

- `redteam_report_shares (token)` — `workspace_id`, `job_id`, optional `compare_job_id`,
  `created_at`, optional `expires_at`, optional `revoked_at`. A share is valid while
  `revoked_at` is null and `expires_at` is null or in the future; that predicate is
  enforced in SQL so validity is never a read-then-filter race.

The report payload itself is **not** stored — it is rebuilt from the job's durable
sessions/events on every read, so a report always reflects current data.

## Rendering

The PDF is produced in the web layer (`apps/web`), not Rust: the public route
`/r/{token}` fetches the payload from the public Rust endpoint and renders a branded
document (`@react-pdf/renderer`). This keeps a heavy PDF dependency out of the
latency-sensitive server and lets the report reuse the dashboard's design tokens,
while the durable token and the report data stay Rust-owned. The "Create report link"
control on the Attacks page mints a token and surfaces a copyable URL; both the
renderer and that control are feature-specific to red-team reports.

## Configuration

Sharing works in both Postgres and memory-only modes (memory-only shares do not
survive a restart, like all memory-only data). Authenticated report diagnostics
can be enriched by configuring `routes.trajectory_diagnostic` in
`config/llm-routing.toml`; public token reads do not call the LLM.
