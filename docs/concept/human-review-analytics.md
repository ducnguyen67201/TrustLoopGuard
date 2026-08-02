# Human Review Analytics

Human review analytics record what a customer reviewer did after a guardrail decision. They measure customer-side intervention separately from Featherlane AI's automated authorization effects.

Featherlane AI stores review outcomes for audit and analytics only. It does not assign reviewers, own the customer's queue, or decide whether the customer's final artifact is correct.

## Ownership

Rust owns human review state.

```text
Dashboard / customer integration
  -> Next.js API proxy when same-origin is needed
  -> Rust /v1/traces/{trace_id}/review-events
  -> tl-storage human_review_events
```

The dashboard may render review status and aggregate charts, but it must not persist review outcomes in a web-owned database.

## Review Events

`human_review_events` is append-only. Each event links to a persisted trace and copies `run_id` and `run_event_id` from the trace when they exist.

Supported outcomes:

- `accepted`
- `corrected`
- `rejected`
- `false_positive`
- `missed_issue`
- `ignored`

The latest event is used as the current review outcome in run detail views. The full event list remains available for audit.

Reviewer notes are potentially sensitive. They are stored as review-event detail and are not echoed into analytics aggregates.

## Analytics

`GET /v1/analytics/human-review` returns Rust-computed aggregates for the dashboard. Automated guardrail intervention is `deny + transform + require_approval + defer`. Human intervention is `corrected + rejected + missed_issue`.

The human intervention rate uses total filtered traces as its denominator, so it can be compared with automated guardrail intervention rates without changing chart semantics.
