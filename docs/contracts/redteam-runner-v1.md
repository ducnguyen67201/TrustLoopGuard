# Red-Team Runner Contract v1

This contract describes the private server-to-runner HTTP protocol used by
`tl-server` when `REDTEAM_RUNNER_URL` is configured. It is infrastructure
contract documentation only: browser clients never call this service directly,
and the runner does not own durable product state.

The public TrustLoopGuard API remains `/v1/redteam/*`. The runner only receives
validated loopback target URLs from `tl-server`, performs transient execution,
and returns scored attack sessions for Rust to persist.

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
  "sessions": [],
  "error": null
}
```

Complete:

```json
{
  "status": "complete",
  "sessions": [
    {
      "sessionId": "session-1",
      "runnerSessionId": "runner-session-1",
      "seq": 0,
      "caseId": "case-1",
      "track": "private_data_flow",
      "kind": "attack",
      "trialIndex": 0,
      "attack": "neutral contract attack",
      "goal": "verify result mapping",
      "status": "complete",
      "outcome": "blocked",
      "landed": false,
      "traceId": "trace-1",
      "events": [
        {
          "eventId": "event-1",
          "seq": 0,
          "kind": "attack_prompt",
          "actor": "attacker",
          "contentText": "test prompt",
          "payload": {},
          "traceId": null
        },
        {
          "eventId": "event-2",
          "seq": 1,
          "kind": "target_reply",
          "actor": "target",
          "contentText": "test reply",
          "payload": {},
          "traceId": "trace-1"
        }
      ],
      "error": null
    }
  ],
  "error": null
}
```

Error:

```json
{
  "status": "error",
  "sessions": [],
  "error": "runner failed"
}
```

## Session Fields

| Field | Type | Notes |
|---|---|---|
| `sessionId` | string | Runner-local stable id for this independent test case. |
| `runnerSessionId` | string or null | Optional upstream session id when the runner has one. |
| `seq` | number | Ordering within the job. |
| `caseId` | string or null | Stable case identity for comparison. |
| `track` | string or null | High-level benchmark/security track. |
| `kind` | string or null | `attack`, `benign`, or another agreed category. |
| `trialIndex` | number or null | Repetition index for repeated trials. |
| `attack` | string | Human-readable attack label. |
| `goal` | string | What the case attempts to verify. |
| `status` | string | `running`, `complete`, or `error`. |
| `outcome` | string | `landed`, `blocked`, `clean`, or `error`. |
| `landed` | boolean | Whether the attack succeeded. |
| `traceId` | string or null | Trace produced by the protected target, when available. |
| `events` | array | Ordered transcript/scoring events. |
| `error` | string or null | Session-local error, if any. |

## Event Fields

| Field | Type | Notes |
|---|---|---|
| `eventId` | string | Stable id inside the session. |
| `seq` | number | Ordering inside the session. |
| `kind` | string | Examples: `attack_prompt`, `target_reply`, `guard_decision`, `scorer_decision`. |
| `actor` | string | Actor that produced the event, such as `attacker`, `target`, `guard`, or `scorer`. |
| `label` | string or null | Optional display/classification label. |
| `contentText` | string or null | Human-readable text for transcript rendering. |
| `payload` | object | Structured metadata for debugging and replay. |
| `traceId` | string or null | Trace associated with this event, when available. |

Contract fixtures live in `docs/contracts/fixtures/redteam-runner/`.
