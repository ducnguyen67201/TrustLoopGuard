import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';

import { runRefundAgent } from './agent';
import type { RefundAgentClient } from './core';
import { searchOrder } from './orders';
import { seedLiveRefundOrder } from './seed';
import { DEMO_ORDER_ID } from './types';

test('live demo mode never falls back to the scripted agent', async () => {
  const client = {} as RefundAgentClient;
  const options = { useOpenAI: false, requireLiveAgent: true };

  await assert.rejects(
    runRefundAgent('Refund order ord_missing for $75 because it arrived damaged.', client, options),
    /live agent/i,
  );
});

test('concurrent live seeds keep each request payment intent isolated', async () => {
  const originalStripeKey = process.env.STRIPE_SECRET_KEY;
  const originalPaymentIntent = process.env.STRIPE_PAYMENT_INTENT_ID;
  const root = mkdtempSync(join(tmpdir(), 'featherlane-ai-live-seed-isolation-'));
  const firstDb = join(root, 'first.sqlite');
  const secondDb = join(root, 'second.sqlite');
  let sequence = 0;

  process.env.STRIPE_SECRET_KEY = 'sk_test_demo';
  process.env.STRIPE_PAYMENT_INTENT_ID = 'pi_do_not_mutate';
  const stripeFetch = async () => {
    sequence += 1;
    return Response.json({ id: `pi_isolated_${sequence}`, status: 'succeeded' });
  };

  try {
    const [firstPaymentIntent, secondPaymentIntent] = await Promise.all([
      seedLiveRefundOrder({ dbPath: firstDb, fetchImpl: stripeFetch }),
      seedLiveRefundOrder({ dbPath: secondDb, fetchImpl: stripeFetch }),
    ]);

    assert.notEqual(firstPaymentIntent, secondPaymentIntent);
    assert.equal(
      searchOrder({ orderId: DEMO_ORDER_ID }, firstDb).order?.paymentIntentId,
      firstPaymentIntent,
    );
    assert.equal(
      searchOrder({ orderId: DEMO_ORDER_ID }, secondDb).order?.paymentIntentId,
      secondPaymentIntent,
    );
    assert.equal(process.env.STRIPE_PAYMENT_INTENT_ID, 'pi_do_not_mutate');
  } finally {
    restoreEnv('STRIPE_SECRET_KEY', originalStripeKey);
    restoreEnv('STRIPE_PAYMENT_INTENT_ID', originalPaymentIntent);
  }
});

function restoreEnv(name: string, value: string | undefined): void {
  if (value === undefined) delete process.env[name];
  else process.env[name] = value;
}
