# Integration guide

TrustLoopGuard evaluates proposed agent behavior before it reaches a user or
external system. The primary integration is the SDK decorator.

Just look at this:

```bash
npm install @trustloopguard/sdk
```

```ts
import { guardAgent } from '@trustloopguard/sdk';

const agent = guardAgent(createAgent(), { agentId: 'support-agent' });

const reply = await agent.reply(userMessage);
sendToUser(reply);
```

Install one package, decorate the agent once where it is created, and keep the
rest of the app calling `agent.reply(...)`. Do not add a guard check to every
handler or helper.

## Agent reply

Create an agent and runtime key in the dashboard, then install the package:

```bash
npm install @trustloopguard/sdk
export TLG_URL=https://api.gettrustloop.app
export TLG_API_KEY=tl_live_...
```

Decorate the agent once:

```ts
import { guardAgent } from '@trustloopguard/sdk';

const agent = guardAgent(createAgent(), { agentId: 'support-agent' });

const reply = await agent.reply(userMessage);
sendToUser(reply);
```

The decorator calls the original `reply()`, submits its returned draft directly
to `POST /v1/events`, then applies the `AuthorizationDecision` before returning
to the caller. The dashboard is not in this runtime path.

For an existing helper like this:

```ts
async function generateReply(message: string): Promise<string> {
  return await agent.reply(message);
}

const reply = await generateReply(userMessage);
sendToUser(reply);
```

decorate the agent before `generateReply()` receives it:

```ts
const agent = guardAgent(createAgent(), { agentId: 'support-agent' });
```

The helper and every downstream call site stay unchanged.

This integration guards the final reply string. It does not automatically
observe hidden framework tool calls, payments, or other side effects, and the
output event does not contain the raw user message by default. Use the explicit
typed helpers below when the application owns one of those boundaries.

## Canonical response

Every runtime domain returns `AuthorizationDecision` with one effect:

| Effect | Caller behavior |
|---|---|
| `permit` | Proceed with the exact proposed subject. |
| `transform` | Proceed only with `transformed_value`; tools and financial actions do not support transform. |
| `deny` | Stop. A grant cannot override this. |
| `require_approval` | Wait for an authenticated approval or present a matching saved grant. |
| `defer` | Stop until evidence or system state changes. Approval cannot bypass it. |

All findings remain in `decision.findings`. Preserve `trace_id` and `receipt_id` for support and audit.

## Tool call

```ts
const result = await client.withAuthorizedAction(
  {
    agentId: 'support-agent',
    operation: 'send_email',
    toolIdentity: { server_id: 'mail', tool_name: 'send_email', schema_hash: 'sha256:...' },
    parameters: { to: 'customer@example.com', body: 'Hello' },
    sideEffect: 'external_communication',
  },
  async () => mail.send({ to: 'customer@example.com', body: 'Hello' }),
);
```

The callback is called at most once. Network retry applies to evaluation, polling, and lease completion—not to the callback.

## Financial action

Create a `family: financial` policy through Rust and use canonical controls:

```ts
await client.createFinancialPolicy({
  id: 'refund-controls',
  description: 'Bound support refunds',
  severity: 'high',
  meter: 'actions',
  when: { agents: ['refund-bot'], action_kinds: ['refund'], operations: ['issue_refund'], currencies: ['USD'], rails: ['payment_http'] },
  per_transaction_minor: 10_000n,
  approval_threshold_minor: 5_000n,
  grant_required: true,
  require_approval_for_new_counterparty: false,
  allowed_counterparty_ids: [],
  denied_counterparty_ids: [],
  approver_roles: ['admin'],
  required_preconditions: ['order_exists', 'payment_captured'],
  missing_evidence_effect: 'defer',
  failed_precondition_effect: 'deny',
  on_breach: 'deny',
});
```

`FinancialActionRecord.authorization_effect` and `authorization_status` describe authority. `execution_status` separately describes provider execution. The later execute call may present `authorization` and `attempt_id`; current policy, evidence, eligibility, grant state, and live budget are rechecked before provider execution.

## Approval and saved authority

`/approvals` is the only actionable queue. A reviewer sees an immutable envelope and sends its `envelope_hash` when approving or denying. Exact-once approval creates a one-use fingerprint-bound grant. Scoped approval is available only when the adapter proposed a typed reusable scope.

`/grants` can also create explicit user-intent authority before an action exists. All grants are revocable and are intersected with the current request and policy boundary. They are not policy exceptions.

## Failure behavior

Fail closed for actions with side effects. If the server cannot determine safety, return or handle `defer`; do not invent an approval or reuse a receipt as authority. Keep the same stable attempt ID across transport retries and use a new attempt ID for a new execution attempt.

See [SDK_DRIVEN.md](SDK_DRIVEN.md) and [authorization-kernel.md](concept/authorization-kernel.md).
