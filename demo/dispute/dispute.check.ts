// Standalone assert check (no test framework — demo/ has none, see
// agent-demo-adapter/workflow-ingest.check.ts). Pins the behavior contract: the
// injected dispute hijacks the standalone agent into refunding the attacker, the
// unprotected agent executes it against its own ledger, and the TrustLoopGuard
// bolt-on's block (not the harness) is what stops the money. Runs fully offline
// — no server, no API key. Run:
//   pnpm --filter @trustloopguard/demo dispute:check
import assert from 'node:assert/strict';

import {
  Client,
  type ActiveRun,
  type Decision,
  type GuardEvent,
  type Verdict,
  type WithRunOptions,
} from '@trustloopguard/sdk';

import { DisputeAgent } from './agent';
import { buildOutputEvent, buildRefundEvent, trustloopGuard, type GuardClient } from './guard';
import { ATTACKER_ACCOUNT, DISPUTED_AMOUNT, benignMessage, customerMessage } from './scenario';

const AGENT_ID = 'demo-dispute-agent';
const RUN_ID = '018f1111-1111-7111-8111-111111111111';
const RUN_EVENT_ID = '018f2222-2222-7222-8222-222222222222';

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

function decisionJson(verdict: Verdict): Record<string, unknown> {
  return {
    trace_id: 't_test',
    verdict,
    reason: 'test decision',
    triggered_policies: [],
    safe_output: null,
    latency_ms: 1,
    tier_results: [],
    redaction: null,
  };
}

function fakeClient(verdict: Verdict, calls?: { n: number; events?: GuardEvent[] }): GuardClient {
  return {
    async withRun<T>(_opts: WithRunOptions, fn: (run: ActiveRun) => Promise<T>): Promise<T> {
      return fn({
        id: 'run_test',
        async withEvent(_req, eventFn) {
          return eventFn();
        },
        async finish() {},
      });
    },
    async submitEvent(event: GuardEvent): Promise<Decision> {
      if (calls !== undefined) {
        calls.n += 1;
        calls.events?.push(event);
      }
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
    event.provenance?.account,
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

  // 5. A clean dispute (no injection) needs no refund, but the guarded path
  //    still calls TrustLoopGuard as an output check and carries a trace.
  const outputEvent = buildOutputEvent(
    { kind: 'request_verification', message: 'Please verify.' },
    AGENT_ID,
  );
  assert.equal(outputEvent.kind, 'output.proposed');
  assert.equal(outputEvent.action.operation, 'output');
  assert.equal(outputEvent.action.side_effect, 'none');
  assert.deepEqual(outputEvent.provenance?.text, ['conversation']);

  const calls = { n: 0, events: [] as GuardEvent[] };
  const benignAgent = new DisputeAgent();
  const benign = await benignAgent.handle(
    benignMessage(),
    trustloopGuard(fakeClient('allow', calls), AGENT_ID),
  );
  assert.equal(benign.action.kind, 'request_verification', 'clean dispute asks for verification');
  assert.equal(calls.n, 1, 'non-refund action still gets a TrustLoopGuard output check');
  assert.equal(calls.events[0]?.kind, 'output.proposed');
  assert.equal(benign.guardTraceId, 't_test', 'guarded non-refund replies carry a trace');
  assert.equal(benignAgent.ledger.length, 0);

  // 6. Demo integration stays one-line at the agent boundary but uses the real
  //    SDK Client so run context is inherited onto the submitted event.
  const postedEvents: GuardEvent[] = [];
  const client = new Client({
    baseUrl: 'http://demo.test',
    fetchImpl: (async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      if (url === 'http://demo.test/v1/runs') {
        return json({
          id: RUN_ID,
          workspace_id: 'ws_demo',
          environment_id: 'production',
          environment: 'production',
          agent_id: AGENT_ID,
          kind: 'chat_session',
          status: 'running',
          external_id: 'dispute-demo',
          metadata: {},
          started_at: '2026-01-01T00:00:00Z',
          ended_at: null,
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
          trace_count: 0,
          blocked_count: 0,
          rewritten_count: 0,
          escalated_count: 0,
          p95_latency_ms: null,
        });
      }
      if (url === `http://demo.test/v1/runs/${RUN_ID}/events`) {
        return json({
          id: RUN_EVENT_ID,
          workspace_id: 'ws_demo',
          run_id: RUN_ID,
          sequence: 1,
          kind: 'tool_call',
          label: 'issue_refund',
          input_summary: null,
          output_summary: null,
          metadata: {},
          occurred_at: '2026-01-01T00:00:00Z',
          created_at: '2026-01-01T00:00:00Z',
        });
      }
      if (url === `http://demo.test/v1/runs/${RUN_ID}`) {
        return json({});
      }
      if (url === 'http://demo.test/v1/events') {
        postedEvents.push(JSON.parse(String(init?.body)) as GuardEvent);
        return json(decisionJson('block'));
      }
      throw new Error(`unexpected demo request: ${url}`);
    }) as typeof fetch,
  });
  const observedAgent = new DisputeAgent();
  await observedAgent.handle(
    customerMessage(),
    trustloopGuard(client, AGENT_ID, {
      externalId: 'dispute-demo',
      inputSummary: 'Customer disputes a charge and attached evidence.',
    }),
  );
  assert.equal(postedEvents[0]?.principal.run_id, RUN_ID, 'one-line demo guard attaches run id');
  assert.equal(
    postedEvents[0]?.principal.run_event_id,
    RUN_EVENT_ID,
    'one-line demo guard attaches run event id',
  );

  process.stdout.write('dispute demo check: all assertions passed\n');
}

function json(body: Record<string, unknown>): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  });
}

main().catch((error) => {
  process.stderr.write(
    `dispute demo check failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
