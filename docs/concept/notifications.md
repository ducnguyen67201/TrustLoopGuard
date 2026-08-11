# Notifications

Notifications close the production feedback loop after durable Gateway and evaluation outcomes.
They are an asynchronous delivery plane, not part of the authorization hot path.

## Ownership

- `tl-core` owns notification rule and delivery wire contracts.
- `tl-storage` owns rules, the durable delivery outbox, deduplication, attempts, and leases.
- `tl-server` owns producers, dashboard APIs, worker orchestration, and SMTP transport.
- `apps/web` is a same-origin proxy and configuration UI only.

No notification rule, delivery state, or SMTP credential is stored in Next.js.

## Rules and events

An enabled email rule belongs to one workspace and environment and may be scoped to an agent. It
selects one or more event kinds:

- `evaluation_failed`
- `evaluation_inconclusive`
- `evaluation_error`
- `provider_terminal_failure`
- `test`

Evaluation producers enqueue only after the result or terminal error is durable. Gateway enqueues a
provider failure only after its bounded reliability plan is exhausted and the failed attempt
evidence is durable. Repeated producer calls converge on the unique rule, event, subject, and subject
version identity.

## Delivery

Postgres is the outbox. Workers claim pending or expired leased deliveries with
`FOR UPDATE SKIP LOCKED`, mark a short sending lease, and perform SMTP outside the transaction.
Successful sends become `sent`; temporary failures are retried with bounded backoff; exhausted or
permanent failures become `failed`. A stopped worker leaves durable work that another replica can
reclaim.

Delivery is at least once. An SMTP timeout can occur after a remote server accepted a message, so a
retry may produce a duplicate. The delivery UUID is stable across attempts and is used as the
message identity to support downstream deduplication.

Messages contain sanitized outcome metadata and a `TL_DASHBOARD_URL` link to the Run. They do not
contain provider response bodies, prompt bodies, SMTP configuration, or credentials. Authorized
dashboard users may read the configured recipient and bounded delivery error category.

## Configuration and readiness

The durable worker starts in Postgres mode when both `TL_NOTIFICATION_SMTP_URL` and
`TL_NOTIFICATION_EMAIL_FROM` are configured. `TL_DASHBOARD_URL` supplies absolute links. Missing
SMTP does not discard rules or deliveries: readiness reports the transport gap and pending work is
not marked sent. Creating an enabled rule or enabling a draft while transport is unavailable returns
`409` with the typed `unavailable` code; disabled drafts may still be saved.

Production-loop activation may skip rule creation only when the request explicitly sets
`alerts_deferred`. This is a recoverable setup state, not readiness: the route remains
`needs_attention` until an enabled rule scoped to the activated agent and a configured SMTP
transport are both present.

Dashboard management endpoints are:

- `GET/POST /v1/notification-rules`
- `PATCH/DELETE /v1/notification-rules/{id}`
- `POST /v1/notification-rules/{id}/test`
- `GET /v1/notification-deliveries`
- `GET /v1/notifications/readiness`

Workspace runtime keys cannot call these control-plane endpoints.
