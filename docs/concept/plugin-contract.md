# Plugin contract

Host adapters translate framework-specific events into `GuardEvent` and return the shared `AuthorizationDecision`. Core engine code never depends on a host framework type.

## Required input

An adapter should collect, before the side effect:

- stable principal and invocation/run identity;
- event kind, operation, full parameters, and side-effect class;
- tool server/name/schema identity when applicable;
- sources and parameter-to-source provenance;
- bounded context needed for policy evaluation.

Do not place decision-bearing facts only in opaque metadata.

## Required output handling

- `permit`: execute the exact proposed subject.
- `transform`: content adapters use only `transformed_value`.
- `deny`: stop.
- `require_approval`: wait for the common approval/grant flow.
- `defer`: stop until evidence or system state changes.

Executable adapters use the common grant claim and one-attempt lease. Callback execution stays outside HTTP retry loops and occurs at most once. Cancellation before callback start prevents later execution. Callback failure cancels the lease and preserves the original error.

Adapters must not log raw sensitive snapshots, authorization claims, or provider credentials. Log trace, receipt, and stable subject identifiers instead.

## Streaming

Streaming content may use `tl-stream::StreamingChecker` to interrupt delivery, but final effect vocabulary and trace semantics remain canonical. Streaming does not create a second authorization lifecycle.

## Adding an adapter

1. Map the host boundary to `GuardEvent` or another tagged `AuthorizationSubject`.
2. Use generated SDK types and `/v1/events` rather than inventing a parallel contract.
3. Preserve stable attempt identity across transport retries.
4. Prove permit, deny, transform, approval, defer, cancellation, and callback-at-most-once behavior.

See [event-engine.md](event-engine.md) and [authorization-kernel.md](authorization-kernel.md).
