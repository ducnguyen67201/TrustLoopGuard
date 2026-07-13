import assert from 'node:assert/strict';
import test from 'node:test';

import type { FinancialActionRecord, FinancialReceipt } from '@trustloopguard/sdk';
import { readRefundDemoActionStatus, type RefundDemoStatusClient } from './status';

test('reads the Stripe receipt after an externally approved demo refund executes', async () => {
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  const client: RefundDemoStatusClient = {
    async getFinancialAction() {
      return demoAction(actionId, 'executed');
    },
    async getFinancialDecisionReceipt() {
      return {
        execution: { status: 'executed', receipt_id: actionId, ledger_event_ids: [] },
      } as Awaited<ReturnType<RefundDemoStatusClient['getFinancialDecisionReceipt']>>;
    },
    async getReceipt(): Promise<FinancialReceipt> {
      return {
        id: actionId,
        action_id: actionId,
        ledger_event_ids: [],
        proof: { provider: { reference: 're_test_approved_123', status: 'succeeded' } },
        created_at: '2026-07-13T21:31:00.000Z',
      };
    },
  };

  assert.deepEqual(await readRefundDemoActionStatus(client, actionId), {
    actionId,
    status: 'executed',
    orderId: 'ord_demo_1001',
    amountMinor: 7_500,
    currency: 'USD',
    receiptId: actionId,
    providerReference: 're_test_approved_123',
    updatedAt: '2026-07-13T21:31:00.000Z',
  });
});

test('refuses to expose a non-demo financial action', async () => {
  const action = demoAction('019f5d63-f8ca-77c3-ae7f-07b122daa7b3', 'held');
  action.action.principal_id = 'another-agent';
  const client = {
    async getFinancialAction() {
      return action;
    },
    async getFinancialDecisionReceipt() {
      throw new Error('not expected');
    },
    async getReceipt() {
      throw new Error('not expected');
    },
  };

  await assert.rejects(readRefundDemoActionStatus(client, action.id), /not found/i);
});

function demoAction(
  id: string,
  status: FinancialActionRecord['status'],
): FinancialActionRecord {
  return {
    id,
    workspace_id: 'default',
    status,
    action: {
      id,
      kind: 'refund',
      operation: 'issue_refund',
      principal_id: 'refund-bot',
      amount: { amount_minor: 7_500n, currency: 'USD' },
      counterparty: { id: 'cust_demo_1001', kind: 'customer', metadata: null },
      rail: 'payment_http',
      mandate: { id: 'mandate_stripe_refund_demo_v1', version: 1 },
      memo: 'Refund ord_demo_1001: item_arrived_damaged',
      metadata: {
        demo_request_id: 'demo-request-123',
        order_id: 'ord_demo_1001',
        reason: 'item_arrived_damaged',
      },
    },
    evidence: [],
    created_at: '2026-07-13T21:30:00.000Z',
    updated_at: '2026-07-13T21:31:00.000Z',
  };
}
