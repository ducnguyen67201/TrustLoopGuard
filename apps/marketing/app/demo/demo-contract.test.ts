import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  parseRefundDemoActionId,
  parseRefundDemoPrompt,
  refundDemoServiceUrl,
  sanitizeRefundDemoStatus,
  sanitizeRefundDemoResponse,
} from './contract';
import { refundDemoReviewUrl } from './review-url';

test('accepts a concrete refund request and trims whitespace', () => {
  assert.equal(
    parseRefundDemoPrompt({
      prompt: '  Refund order ord_demo_1001 for $75 because it arrived damaged.  ',
    }),
    'Refund order ord_demo_1001 for $75 because it arrived damaged.',
  );
});

test('rejects empty and oversized chat input at the public boundary', () => {
  assert.throws(() => parseRefundDemoPrompt({ prompt: '   ' }), /prompt/i);
  assert.throws(() => parseRefundDemoPrompt({ prompt: 'x'.repeat(501) }), /500/);
});

test('only permits loopback HTTP or HTTPS demo-service URLs', () => {
  assert.equal(refundDemoServiceUrl('http://127.0.0.1:9310'), 'http://127.0.0.1:9310');
  assert.equal(refundDemoServiceUrl('https://demo-api.gettrustloop.app/'), 'https://demo-api.gettrustloop.app');
  assert.throws(() => refundDemoServiceUrl('http://example.com'), /https/i);
  assert.throws(() => refundDemoServiceUrl('file:///tmp/demo'), /https/i);
});

test('exposes only the public agent trace, order, refund, and decision fields', () => {
  const response = sanitizeRefundDemoResponse({
    result: {
      prompt: 'Refund order ord_demo_1001 for $75.',
      traces: [{ tool: 'prepare_refund', summary: 'held: approval required', secret: 'hidden' }],
      finalMessage: 'Held for approval.',
      actionId: 'financial_action_123',
      receiptId: undefined,
      internalPrompt: 'hidden',
    },
    state: {
      orders: [
        {
          id: 'ord_demo_1001',
          customerName: 'Jamie Demo',
          customerEmail: 'jamie@example.com',
          paymentMethodLast4: '4242',
          amountPaidMinor: 10_000,
          refundableBalanceMinor: 10_000,
          currency: 'USD',
          captured: true,
          refundWindowOpen: true,
          refundCount: 0,
          paymentIntentId: 'pi_secret_internal',
        },
      ],
      refunds: [],
    },
    logs: [{ step: 'prepare_refund', message: 'held: financial_action_123' }],
    runtime: {
      agent: 'openai',
      guard: 'trustloopguard-rust-api',
      provider: 'stripe-test',
    },
    providerApiKey: 'must-not-leak',
  });

  assert.equal(response.result.traces[0]?.tool, 'prepare_refund');
  assert.equal(response.state.orders[0]?.paymentMethodLast4, '4242');
  assert.equal('customerEmail' in response.state.orders[0]!, false);
  assert.equal('paymentIntentId' in response.state.orders[0]!, false);
  assert.equal('providerApiKey' in response, false);
  assert.equal('logs' in response, false);
});

test('validates and redacts a public refund action status', () => {
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  assert.equal(parseRefundDemoActionId(actionId), actionId);
  assert.throws(() => parseRefundDemoActionId('not-an-action-id'), /action/i);

  const status = sanitizeRefundDemoStatus({
    actionId,
    status: 'executed',
    orderId: 'ord_demo_1001',
    amountMinor: 7_500,
    currency: 'USD',
    receiptId: actionId,
    providerReference: 're_test_status_123',
    updatedAt: '2026-07-13T21:31:00.000Z',
    paymentIntentId: 'pi_private',
  });
  assert.equal(status.providerReference, 're_test_status_123');
  assert.equal('paymentIntentId' in status, false);
});

test('the Product Hunt route shows a live chat, the control boundary, and Stripe outcome', () => {
  const page = readFileSync(new URL('./page.tsx', import.meta.url), 'utf8');
  const demo = readFileSync(new URL('./refund-demo.tsx', import.meta.url), 'utf8');
  const source = `${page}\n${demo}`;

  assert.match(source, /Ask the refund agent/i);
  assert.match(source, /TrustLoopGuard/i);
  assert.match(source, /Stripe test mode/i);
  assert.match(source, /not a scripted animation/i);
  assert.match(source, /api\/demo\/refund\?actionId=/i);
  assert.match(source, /Review this exact action/i);
});

test('links a held refund to the exact dashboard financial action', () => {
  assert.equal(
    refundDemoReviewUrl(
      '019f5d6a-f57d-7c23-ada2-acc821b332ea',
      'http://localhost:3000/',
    ),
    'http://localhost:3000/financial?workspace=default&environment=production&actionId=019f5d6a-f57d-7c23-ada2-acc821b332ea',
  );
});
