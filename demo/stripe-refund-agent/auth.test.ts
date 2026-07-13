import assert from 'node:assert/strict';
import test from 'node:test';

import {
  RefundDemoRequestBudget,
  isValidRefundDemoAuthorization,
  requireRefundDemoProxySecret,
} from './auth';

const VALID_SECRET = 'refund-demo-proxy-secret-32-bytes-minimum';

test('requires a strong proxy secret', () => {
  assert.throws(() => requireRefundDemoProxySecret(undefined), /required/i);
  assert.throws(() => requireRefundDemoProxySecret('too-short'), /32/);
  assert.equal(requireRefundDemoProxySecret(VALID_SECRET), VALID_SECRET);
});

test('accepts only the exact bearer credential', () => {
  assert.equal(isValidRefundDemoAuthorization(undefined, VALID_SECRET), false);
  assert.equal(isValidRefundDemoAuthorization('Bearer wrong', VALID_SECRET), false);
  assert.equal(
    isValidRefundDemoAuthorization(`Bearer ${VALID_SECRET}`, VALID_SECRET),
    true,
  );
});

test('enforces a central fixed-window budget for expensive live runs', () => {
  const budget = new RefundDemoRequestBudget({ maxRequests: 2, windowMs: 1_000 });

  assert.equal(budget.tryAcquire(100), true);
  assert.equal(budget.tryAcquire(200), true);
  assert.equal(budget.tryAcquire(300), false);
  assert.equal(budget.tryAcquire(1_101), true);
});
