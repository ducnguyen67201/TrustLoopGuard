# Red-Team Runner Contract v1

This contract describes the private server-to-runner HTTP protocol used by
`tl-server` when `REDTEAM_RUNNER_URL` is configured. It is infrastructure
contract documentation only: browser clients never call this service directly,
and the runner does not own durable product state.

The public TrustLoopGuard API remains `/v1/redteam/*`. The runner only receives
validated loopback target URLs from `tl-server`, performs transient execution,
and returns scored attack results for Rust to persist.

## Transport

- Base URL: configured server-side with `REDTEAM_RUNNER_URL`.
- Payload casing: JSON camelCase.
- Authentication: deployment-owned. The public contract does not prescribe the
  private auth mechanism.
- Durable state: none. Runner job state may be transient while a run is active.

## `GET /health`

Returns `200 OK` when the runner is ready to accept jobs.

```json
{ "status": "ok" }
```

## `POST /redteam/jobs`

Creates a transient runner job.

Request:

```json
{
  "targetUrl": "http://127.0.0.1:9102",
  "profile": "fast"
}
```

Response:

```json
{
  "jobId": "runner-job-1"
}
```

## `GET /redteam/jobs/{jobId}`

Polls a runner job until it returns a terminal state.

Running:

```json
{
  "status": "running",
  "attacks": [],
  "error": null
}
```

Complete:

```json
{
  "status": "complete",
  "attacks": [
    {
      "caseId": "case-1",
      "track": "private_data_flow",
      "kind": "attack",
      "trialIndex": 0,
      "attack": "neutral contract attack",
      "goal": "verify result mapping",
      "outcome": "blocked",
      "landed": false,
      "prompt": "test prompt",
      "reply": "test reply",
      "traceId": "trace-1"
    }
  ],
  "error": null
}
```

Error:

```json
{
  "status": "error",
  "attacks": [],
  "error": "runner failed"
}
```

## Result Fields

| Field | Type | Notes |
|---|---|---|
| `caseId` | string or null | Stable case identity for comparison. |
| `track` | string or null | High-level benchmark/security track. |
| `kind` | string or null | `attack`, `benign`, or another agreed category. |
| `trialIndex` | number or null | Repetition index for repeated trials. |
| `attack` | string | Human-readable attack label. |
| `goal` | string | What the case attempts to verify. |
| `outcome` | string | `landed`, `blocked`, `clean`, or `error`. |
| `landed` | boolean | Whether the attack succeeded. |
| `prompt` | string or null | Prompt sent to the target, if retained for reporting. |
| `reply` | string | Target reply or error-safe placeholder. |
| `traceId` | string or null | Trace produced by the protected target, when available. |

Contract fixtures live in `docs/contracts/fixtures/redteam-runner/`.
