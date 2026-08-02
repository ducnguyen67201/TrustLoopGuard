import { randomUUID } from 'node:crypto';

import { resetOrderDatabase } from './order-db';
import {
  createTestPaymentIntent,
  stripeTestKeyFromEnv,
  type StripeFetch,
} from './stripe';
import { DEMO_ORDER_ID } from './types';

const DEMO_ORDER_AMOUNT_MINOR = 10_000;

/** Creates a fresh captured Stripe test payment for each public demo run. */
export async function seedLiveRefundOrder(
  options: { dbPath?: string; fetchImpl?: StripeFetch } = {},
): Promise<string> {
  const secretKey = stripeTestKeyFromEnv();
  if (secretKey === null) {
    throw new Error('live refund demo requires STRIPE_SECRET_KEY in Stripe test mode');
  }

  const runId = randomUUID();
  const paymentIntent = await createTestPaymentIntent({
    secretKey,
    amountMinor: DEMO_ORDER_AMOUNT_MINOR,
    idempotencyKey: `featherlane-ai-public-refund-demo:${runId}`,
    metadata: {
      order_id: DEMO_ORDER_ID,
      demo_run_id: runId,
      source: 'featherlane_ai_product_hunt_demo',
    },
    fetchImpl: options.fetchImpl,
  });
  if (paymentIntent.status !== 'succeeded') {
    throw new Error(`Stripe test payment did not succeed (${paymentIntent.status})`);
  }

  resetOrderDatabase(options.dbPath, paymentIntent.id);
  return paymentIntent.id;
}
