import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  parseRefundDemoPrompt,
  refundDemoServiceUrl,
  sanitizeRefundDemoResponse,
} from './contract';

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
    providerApiKey: 'must-not-leak',
  });

  assert.equal(response.result.traces[0]?.tool, 'prepare_refund');
  assert.equal(response.state.orders[0]?.paymentMethodLast4, '4242');
  assert.equal('customerEmail' in response.state.orders[0]!, false);
  assert.equal('paymentIntentId' in response.state.orders[0]!, false);
  assert.equal('providerApiKey' in response, false);
});

test('the Product Hunt route shows a live chat, the control boundary, and Stripe outcome', () => {
  const page = readFileSync(new URL('./page.tsx', import.meta.url), 'utf8');

  assert.match(page, /Ask the refund agent/i);
  assert.match(page, /TrustLoopGuard/i);
  assert.match(page, /Stripe test mode/i);
  assert.match(page, /not a scripted animation/i);
});
