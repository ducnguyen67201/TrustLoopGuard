import assert from 'node:assert/strict';
import test from 'node:test';

import type { RefundDemoResponse, RefundDemoStatus } from './contract';
import { mergeRefundDemoStatus } from './status-model';

test('merges an externally approved Stripe execution into the held demo response', () => {
  const response = heldResponse();
  const status: RefundDemoStatus = {
    actionId: response.result.actionId!,
    authorizationEffect: 'permit',
    executionStatus: 'succeeded',
    orderId: 'ord_demo_1001',
    amountMinor: 7_500,
    currency: 'USD',
    receiptId: response.result.actionId!,
    providerReference: 're_test_approved_123',
    updatedAt: '2026-07-13T21:31:00.000Z',
  };

  const merged = mergeRefundDemoStatus(response, status);
  assert.equal(merged.result.receiptId, status.receiptId);
  assert.match(merged.result.finalMessage, /approved/i);
  assert.equal(merged.result.traces.at(-1)?.tool, 'execute_refund');
  assert.equal(merged.state.orders[0]?.refundableBalanceMinor, 2_500);
  assert.equal(merged.state.orders[0]?.refundCount, 1);
  assert.equal(merged.state.refunds[0]?.providerReference, 're_test_approved_123');
});

test('does not merge a status belonging to another action', () => {
  const response = heldResponse();
  const status: RefundDemoStatus = {
    actionId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b4',
    authorizationEffect: 'permit',
    executionStatus: 'succeeded',
    orderId: 'ord_demo_1001',
    amountMinor: 7_500,
    currency: 'USD',
    receiptId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b4',
    updatedAt: '2026-07-13T21:31:00.000Z',
  };
  assert.equal(mergeRefundDemoStatus(response, status), response);
});

function heldResponse(): RefundDemoResponse {
  return {
    result: {
      prompt: 'Refund order ord_demo_1001 for $75.',
      traces: [
        { tool: 'search_order', summary: 'found ord_demo_1001' },
        {
          tool: 'prepare_refund',
          summary: 'require_approval: refund 019f5d63-f8ca-77c3-ae7f-07b122daa7b3 requires approval',
        },
      ],
      finalMessage: 'The refund is held for approval.',
      actionId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
    },
    state: {
      orders: [
        {
          id: 'ord_demo_1001',
          customerName: 'Jamie Demo',
          paymentMethodLast4: '4242',
          amountPaidMinor: 10_000,
          refundableBalanceMinor: 10_000,
          currency: 'USD',
          captured: true,
          refundWindowOpen: true,
          refundCount: 0,
        },
      ],
      refunds: [],
    },
    runtime: {
      agent: 'openai',
      guard: 'trustloopguard-rust-api',
      provider: 'stripe-test',
    },
  };
}
