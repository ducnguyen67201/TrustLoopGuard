import { randomUUID } from 'node:crypto';

import { resetOrderDatabase } from './order-db';
import { createTestPaymentIntent, stripeTestKeyFromEnv } from './stripe';
import { DEMO_ORDER_ID } from './types';

const DEMO_ORDER_AMOUNT_MINOR = 10_000;

/** Creates a fresh captured Stripe test payment for each public demo run. */
export async function seedLiveRefundOrder(): Promise<string> {
  const secretKey = stripeTestKeyFromEnv();
  if (secretKey === null) {
    throw new Error('live refund demo requires STRIPE_SECRET_KEY in Stripe test mode');
  }

  const runId = randomUUID();
  const paymentIntent = await createTestPaymentIntent({
    secretKey,
    amountMinor: DEMO_ORDER_AMOUNT_MINOR,
    idempotencyKey: `tlg-public-refund-demo:${runId}`,
    metadata: {
      order_id: DEMO_ORDER_ID,
      demo_run_id: runId,
      source: 'trustloopguard_product_hunt_demo',
    },
  });
  if (paymentIntent.status !== 'succeeded') {
    throw new Error(`Stripe test payment did not succeed (${paymentIntent.status})`);
  }

  process.env.STRIPE_PAYMENT_INTENT_ID = paymentIntent.id;
  resetOrderDatabase();
  return paymentIntent.id;
}
