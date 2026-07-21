import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  parseRefundDemoActionId,
  parseRefundDemoPrompt,
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
    authorizationEffect: 'permit',
    executionStatus: 'succeeded',
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
  const pageContent = readFileSync(new URL('./refund-page.tsx', import.meta.url), 'utf8');
  const demo = readFileSync(new URL('./refund-demo.tsx', import.meta.url), 'utf8');
  const content = readFileSync(new URL('./refund-content.ts', import.meta.url), 'utf8');
  const source = `${page}\n${pageContent}\n${demo}\n${content}`;

  assert.match(source, /Ask the refund agent/i);
  assert.match(source, /TrustLoopGuard/i);
  assert.match(source, /Stripe test mode/i);
  assert.match(source, /not a scripted animation/i);
  assert.match(source, /api\/demo\/refund\?actionId=/i);
  assert.match(source, /Review this exact action/i);
});

test('the Vietnamese refund route localizes the page and reuses the live workflow', () => {
  const page = readFileSync(new URL('../vi/demo/page.tsx', import.meta.url), 'utf8');
  const pageContent = readFileSync(new URL('./refund-page.tsx', import.meta.url), 'utf8');
  const demo = readFileSync(new URL('./refund-demo.tsx', import.meta.url), 'utf8');
  const content = readFileSync(new URL('./refund-content.ts', import.meta.url), 'utf8');
  const sitemap = readFileSync(new URL('../sitemap.ts', import.meta.url), 'utf8');
  const source = `${page}\n${pageContent}\n${demo}\n${content}`;

  assert.match(page, /<RefundDemoPageContent locale="vi"/);
  assert.match(page, /canonical: '\/vi\/demo'/);
  assert.match(page, /locale: 'vi_VN'/);
  assert.match(source, /Yêu cầu tác nhân hoàn tiền/);
  assert.match(source, /Không dùng tiền thật/i);
  assert.match(source, /<RefundDemo locale=\{locale\}/);
  assert.match(demo, /locale === 'vi'/);
  assert.match(sitemap, /url: absoluteUrl\('\/vi\/demo'\)/);
  assert.doesNotMatch(page, /runHostedRefundDemo|withAuthorizedAction/);
});

test('tracks demo activation without sending the customer prompt to analytics', () => {
  const demo = readFileSync(new URL('./refund-demo.tsx', import.meta.url), 'utf8');

  assert.match(demo, /trackMarketingEvent\('demo_started'/);
  assert.match(demo, /trackMarketingEvent\('demo_decision_shown'/);
  assert.match(demo, /scenario,/);

  const analyticsCalls = demo.match(/trackMarketingEvent\([\s\S]*?\n\s*\}\);/g) ?? [];
  assert.equal(analyticsCalls.length, 3);
  for (const analyticsCall of analyticsCalls) {
    assert.doesNotMatch(analyticsCall, /prompt\s*:/);
    assert.doesNotMatch(analyticsCall, /actionId|receiptId/);
  }
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

test('keeps a tracked app entry action visible across every demo top bar', () => {
  const appLink = readFileSync(new URL('./demo-app-link.tsx', import.meta.url), 'utf8');
  const demoPages = [
    './refund-page.tsx',
    './healthcare/healthcare-page.tsx',
    './procurement/procurement-page.tsx',
    './personalized-contextual-page.tsx',
  ].map((path) => readFileSync(new URL(path, import.meta.url), 'utf8'));

  assert.match(appLink, /href=\{APP_URL\}/);
  assert.match(appLink, /event="app_click"/);
  assert.match(appLink, /Go to the app/);
  for (const demoPage of demoPages) {
    assert.match(demoPage, /<DemoAppLink locale=\{locale\} \/>/);
  }
});
