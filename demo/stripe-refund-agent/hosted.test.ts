import assert from 'node:assert/strict';
import test from 'node:test';

import { RefundDemoRequestBudget } from './auth';
import {
  RefundDemoBudgetExceededError,
  readHostedRefundDemoStatus,
  runHostedRefundDemo,
} from './hosted';

test('runs the hosted refund workflow in-process and removes request state', async () => {
  const events: string[] = [];
  const client = { name: 'demo-client' } as never;
  const state = {
    orders: [],
    refunds: [],
  };

  const response = await runHostedRefundDemo('Refund order ord_demo_1001 for $25.', {
    budget: new RefundDemoRequestBudget({ maxRequests: 1, windowMs: 60_000 }),
    createClient: () => client,
    createRequestId: () => 'request-123',
    temporaryDirectory: () => '/tmp/refund-demo-hosted-test',
    seedOrder: async (options) => {
      events.push(`seed:${options?.dbPath}`);
      return 'pi_test_123';
    },
    runAgent: async (prompt, receivedClient, options) => {
      events.push(`agent:${prompt}:${options?.dbPath}`);
      assert.equal(receivedClient, client);
      assert.equal(options?.refundGrantId, undefined);
      assert.equal(options?.allowGrantProvisioning, false);
      return {
        prompt,
        traces: [],
        finalMessage: 'Refund executed.',
        actionId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
      };
    },
    readState: (dbPath) => {
      events.push(`state:${dbPath}`);
      return state;
    },
    removeDatabase: (dbPath) => events.push(`remove:${dbPath}`),
  });

  const expectedPath = '/tmp/refund-demo-hosted-test/trustloopguard-refund-demo/request-123.sqlite';
  assert.deepEqual(events, [
    `seed:${expectedPath}`,
    `agent:Refund order ord_demo_1001 for $25.:${expectedPath}`,
    `state:${expectedPath}`,
    `remove:${expectedPath}`,
  ]);
  assert.equal(response.result.finalMessage, 'Refund executed.');
  assert.equal(response.state, state);
  assert.deepEqual(response.runtime, {
    agent: 'openai',
    guard: 'trustloopguard-rust-api',
    provider: 'stripe-test',
  });
});

test('enforces the shared hosted launch budget before creating external state', async () => {
  const budget = new RefundDemoRequestBudget({ maxRequests: 1, windowMs: 60_000 });
  let seeded = 0;
  const dependencies = {
    budget,
    createClient: () => ({}) as never,
    createRequestId: () => 'budget-test',
    temporaryDirectory: () => '/tmp',
    seedOrder: async () => {
      seeded += 1;
      return 'pi_test';
    },
    runAgent: async (prompt: string) => ({ prompt, traces: [], finalMessage: 'done' }),
    readState: () => ({ orders: [], refunds: [] }),
    removeDatabase: () => undefined,
  };

  await runHostedRefundDemo('first request', dependencies);
  await assert.rejects(
    () => runHostedRefundDemo('second request', dependencies),
    RefundDemoBudgetExceededError,
  );
  assert.equal(seeded, 1);
});

test('reads action status directly from TrustLoopGuard', async () => {
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  const expected = {
    actionId,
    authorizationEffect: 'require_approval' as const,
    executionStatus: 'not_started' as const,
    orderId: 'ord_demo_1001',
    amountMinor: 7_500,
    currency: 'USD' as const,
    updatedAt: '2026-07-13T21:31:00.000Z',
  };
  const client = { name: 'status-client' } as never;

  const result = await readHostedRefundDemoStatus(actionId, {
    createClient: () => client,
    readStatus: async (receivedClient, receivedActionId) => {
      assert.equal(receivedClient, client);
      assert.equal(receivedActionId, actionId);
      return expected;
    },
  });

  assert.equal(result, expected);
});
