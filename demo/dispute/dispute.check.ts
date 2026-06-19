// Standalone assert check (no test framework — demo/ has none, see
// agent-demo-adapter/workflow-ingest.check.ts). Pins the behavior contract: the
// injected dispute hijacks the standalone agent into refunding the attacker, the
// unprotected agent executes it against its own ledger, and the TrustLoopGuard
// bolt-on's block (not the harness) is what stops the money. Runs fully offline
// — no server, no API key. Run:
//   pnpm --filter @trustloopguard/demo dispute:check
import assert from 'node:assert/strict';

import type { Decision, GuardEvent, Verdict } from '@trustloopguard/sdk';

import { DisputeAgent } from './agent';
import { buildRefundEvent, trustloopGuard, type GuardClient } from './guard';
import { ATTACKER_ACCOUNT, DISPUTED_AMOUNT, benignMessage, customerMessage } from './scenario';

const AGENT_ID = 'demo-dispute-agent';

function decision(verdict: Verdict, safeOutput: string | null = null): Decision {
  return {
    trace_id: 't_test',
    verdict,
    reason: 'test decision',
    triggered_policies: [],
    safe_output: safeOutput,
    latency_ms: 1n,
    tier_results: [],
    redaction: null,
  };
}

function fakeClient(verdict: Verdict, calls?: { n: number }): GuardClient {
  return {
    async submitEvent(_event: GuardEvent): Promise<Decision> {
      if (calls !== undefined) calls.n += 1;
      return decision(verdict);
    },
  };
}

async function main(): Promise<void> {
  // 1. The injection hijacks the standalone agent into refunding the attacker's
  //    account — destination read from the text, not hard-coded — and the
  //    UNPROTECTED agent executes it against its own ledger (money moves).
  const rawAgent = new DisputeAgent();
  const raw = await rawAgent.handle(customerMessage());
  assert.equal(raw.action.kind, 'issue_refund', 'injected dispute should drive a refund action');
  if (raw.action.kind !== 'issue_refund') return; // narrow
  assert.equal(raw.action.account, ATTACKER_ACCOUNT, 'refund destination is the attacker account');
  assert.equal(raw.action.amount, DISPUTED_AMOUNT, 'refund amount matches the disputed charge');
  assert.equal(rawAgent.ledger.length, 1, 'unprotected agent executes the refund');
  assert.equal(rawAgent.ledger[0]?.account, ATTACKER_ACCOUNT, 'money went to the attacker');

  // 2. The proposed tool call marks the destination account as flowing from the
  //    untrusted conversation source — the signal the engine's parameter-auth
  //    checker blocks on (issue_refund.account registered authority-bearing).
  const event = buildRefundEvent(raw.action, AGENT_ID);
  assert.equal(event.kind, 'tool.call.proposed');
  assert.equal(event.action.operation, 'issue_refund');
  assert.equal(event.action.side_effect, 'api_mutation');
  assert.deepEqual(
    event.provenance.account,
    ['conversation'],
    'account provenance must point at the untrusted conversation source',
  );

  // 3. PROTECTED: same agent + the TrustLoopGuard interceptor. On block, no
  //    money moves and the reply is a safe message.
  const blockedAgent = new DisputeAgent();
  const blocked = await blockedAgent.handle(
    customerMessage(),
    trustloopGuard(fakeClient('block'), AGENT_ID),
  );
  assert.equal(blockedAgent.ledger.length, 0, 'guard block prevents the refund');
  assert.equal(blocked.executed, false);
  assert.ok(
    blocked.reply.length > 0 && !blocked.reply.includes('refunded'),
    'guarded reply is a safe message, not a refund confirmation',
  );

  // 4. Control: when the guard allows, the SAME path executes — proving the
  //    block in #3 is the guard's doing, not the agent refusing on its own.
  const allowedAgent = new DisputeAgent();
  const allowed = await allowedAgent.handle(
    customerMessage(),
    trustloopGuard(fakeClient('allow'), AGENT_ID),
  );
  assert.equal(allowedAgent.ledger.length, 1, 'guard allow lets the refund through');
  assert.equal(allowed.executed, true);

  // 5. A clean dispute (no injection) needs no refund and makes no guard call.
  const calls = { n: 0 };
  const benignAgent = new DisputeAgent();
  const benign = await benignAgent.handle(
    benignMessage(),
    trustloopGuard(fakeClient('allow', calls), AGENT_ID),
  );
  assert.equal(benign.action.kind, 'request_verification', 'clean dispute asks for verification');
  assert.equal(calls.n, 0, 'non-refund action makes no guard call');
  assert.equal(benignAgent.ledger.length, 0);

  process.stdout.write('dispute demo check: all assertions passed\n');
}

main().catch((error) => {
  process.stderr.write(
    `dispute demo check failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
