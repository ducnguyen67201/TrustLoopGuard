import assert from 'node:assert/strict';
import test from 'node:test';

import type { FinancialActionRecord, FinancialReceipt } from '@trustloopguard/sdk';
import { readRefundDemoActionStatus, type RefundDemoStatusClient } from './status';

test('reads linked authorization and execution state for a completed refund', async () => {
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  const client: RefundDemoStatusClient = {
    async getFinancialAction() { return demoAction(actionId); },
    async getReceipt(): Promise<FinancialReceipt> {
      return {
        id: actionId, action_id: actionId, authorization_receipt_id: `authorization_${actionId}`,
        ledger_event_ids: [], proof: { provider: { reference: 're_test_approved_123', status: 'succeeded' } },
        created_at: '2026-07-13T21:31:00.000Z',
      };
    },
  };

  assert.deepEqual(await readRefundDemoActionStatus(client, actionId), {
    actionId, authorizationEffect: 'permit', executionStatus: 'succeeded',
    orderId: 'ord_demo_1001', amountMinor: 7_500, currency: 'USD', receiptId: actionId,
    providerReference: 're_test_approved_123', updatedAt: '2026-07-13T21:31:00.000Z',
  });
});

test('refuses to expose a non-demo financial action', async () => {
  const action = demoAction('019f5d63-f8ca-77c3-ae7f-07b122daa7b3');
  action.action.principal_id = 'another-agent';
  const client: RefundDemoStatusClient = {
    async getFinancialAction() { return action; },
    async getReceipt() { throw new Error('not expected'); },
  };
  await assert.rejects(readRefundDemoActionStatus(client, action.id), /not found/i);
});

function demoAction(id: string): FinancialActionRecord {
  return {
    id, workspace_id: 'default', environment_id: 'production',
    authorization_intent_id: `intent_${id}`, authorization_receipt_id: `authorization_${id}`,
    authorization_effect: 'permit', authorization_status: 'authorized', execution_status: 'succeeded',
    state: 'executed',
    action: {
      id, kind: 'refund', operation: 'issue_refund', principal_id: 'refund-bot',
      amount: { amount_minor: 7_500n, currency: 'USD' },
      counterparty: { id: 'cust_demo_1001', kind: 'customer', metadata: null },
      rail: 'payment_http', memo: 'Refund ord_demo_1001: item_arrived_damaged',
      metadata: { demo_request_id: 'demo-request-123', order_id: 'ord_demo_1001', reason: 'item_arrived_damaged' },
    },
    evidence: [], created_at: '2026-07-13T21:30:00.000Z', updated_at: '2026-07-13T21:31:00.000Z',
  };
}
