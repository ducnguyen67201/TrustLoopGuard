# Authorization kernel

The authorization kernel is the single Rust-owned control plane for deciding whether an agent action may proceed. Content observations, tool calls, and financial actions retain typed domain evaluation, but they share one decision language and one durable authority model.

## Runtime contract

Every evaluation ends with one `AuthorizationEffect`:

- `permit`: the current subject may proceed.
- `transform`: only the returned transformed value may proceed.
- `deny`: a hard policy or invariant stops execution.
- `require_approval`: one or more explicit authority requirements remain.
- `defer`: evidence or system state is unresolved; approval cannot bypass it.

Composition is fail-closed: `deny > defer > require_approval > transform > permit`. The receipt retains every finding even when a stronger effect determines the result.

## Lifecycle at a glance

```text
intent -> policy and live checks -> approval (only when authority is missing)
       -> grant -> policy and live recheck -> one-attempt lease -> execution -> receipt
```

Only the [approval](glossary.md#authorization-approval) is a pending human decision. Approving it does not execute the action; it creates a revocable [grant](glossary.md#authorization-grant). The caller presents that authority as an [authorization claim](glossary.md#authorization-claim), then the kernel rechecks current policy and live state before issuing an [execution lease](glossary.md#execution-lease). The [authorization receipt](glossary.md#authorization-receipt) records why the attempt was permitted or stopped. Every receipt is authorization activity; only a `require_approval` record with a pending approval is human queue work.

The same lifecycle covers tool and financial work. A financial action remains a financial-domain record for execution and ledger history, but any human decision it needs appears in the common `/approvals` queue with the `financial` domain label.

## Authority flow

1. A typed domain adapter normalizes the subject and computes a versioned fingerprint.
2. Current policies and live domain checks emit findings and explicit authority requirements.
3. Active grants are matched by tenant, environment, principal, domain, capability, requirement IDs, typed scope, time, and version.
4. A matching grant may satisfy only its covered `require_approval` findings. It cannot remove `deny` or `defer`.
5. When approval is required, the server creates one immutable, hash-bound approval envelope.
6. An authenticated reviewer denies it or mints an `exact_once` or bounded `scoped` grant.
7. The caller retries the same subject with `grant_id` and a stable `attempt_id`.
8. The kernel re-evaluates current policy and live state before claiming a one-attempt execution lease.
9. The SDK, MCP proxy, Claude Code command hook, or financial executor runs once and completes the lease as consumed or canceled.
10. A common authorization receipt records the decision, findings, policy versions, and approval/grant/lease references. Domain execution evidence remains owned by that domain.

The effective boundary is always the intersection of the request, the active grant scope, and current policy. A grant can narrow authority but cannot widen policy.

## Ownership

- Wire contracts: `crates/tl-core/src/authorization.rs`
- Pure finding composition and scope matching: `crates/tl-engine/src/authorization.rs`
- Orchestration and HTTP API: `crates/tl-server/src/authorization.rs`
- Durable state: `crates/tl-storage`
- Dashboard: a thin Next.js proxy plus `/approvals` and `/grants`

`POST /v1/events` remains the direct SDK hot path. The dashboard never sits in the runtime path.

The Claude Code bridge is an execution-lease owner: it persists the returned lease before allowing the tool, then consumes or cancels that exact lease from the matching post hook. Its command-specific translation and failure behavior live in [command-safety.md](command-safety.md); the authority lifecycle remains the common kernel described here.

## Operator surfaces

`/approvals` is the Authorization screen. Its Activity tab lists recent permit, transform, deny, require-approval, and defer receipts; those are evaluated outcomes and never imply human sign-off. Needs approval is the only actionable human queue, and Approval history contains resolved human decisions. `/grants` lists and revokes reusable authority. The financial ledger shows authorization and execution facts but does not authorize actions. Human-review analytics are historical observations and never mint authority.

## HTTP surface

- `GET /v1/authorization/approvals` and `GET /v1/authorization/approvals/{id}`
- `POST /v1/authorization/approvals/{id}/decide`
- `GET|POST /v1/authorization/grants`
- `POST /v1/authorization/grants/{id}/revoke`
- `POST /v1/authorization/leases/{id}/complete`
- `GET /v1/authorization/receipts`
- `GET /v1/authorization/receipts/{id}`

Dashboard decisions, receipt listing, and grant mutations require an authenticated Owner/Admin or the internal service lane. The environment-scoped receipt list returns the most recent 200 records. Runtime keys cannot list workspace receipt activity, decide approvals, or manage grants. They may read only an approval or receipt bound to their stored workspace, environment, and principal, and may complete only a lease owned by that principal.

Reviewer signing is an authenticated state transition bound to the immutable `envelope_hash`. It is not a wallet signature or a portable bearer credential. Runtime authority remains the revocable grant row; receipts are audit evidence only.

## Persistence

The five canonical tables are `authorization_intents`, `authorization_approvals`, `authorization_grants`, `authorization_leases`, and `authorization_receipts`. Receipts carry optional principal, operation, and run linkage for activity and legacy-compatible reads. Approval decision plus grant minting is transactional. Grant use-count increment plus lease claim is transactional. Tenant and environment are part of every key and query.
