import { timingSafeEqual } from 'node:crypto';

import { createStripeRefund, stripeTestKeyFromEnv, type StripeFetch } from './stripe';
import {
  DEFAULT_PROVIDER_API_KEY,
  type StripeRefundProviderRequest,
  type StripeRefundProviderResponse,
} from './types';

export interface ProviderReply {
  statusCode: number;
  body: StripeRefundProviderResponse | { error: string };
}

export async function handleProviderPayment(
  authorization: string | undefined,
  request: StripeRefundProviderRequest,
  fetchImpl?: StripeFetch,
): Promise<ProviderReply> {
  if (!isValidProviderAuthorization(authorization)) {
    return { statusCode: 401, body: { error: 'invalid provider bearer token' } };
  }
  if (request.kind !== 'refund') {
    return { statusCode: 400, body: { error: 'provider only supports refund actions' } };
  }
  if (request.currency !== 'USD') {
    return { statusCode: 400, body: { error: 'provider only supports USD refunds' } };
  }

  const amountMinor = request.amount_minor ?? request.amount;
  if (!Number.isInteger(amountMinor) || amountMinor === undefined || amountMinor <= 0) {
    return { statusCode: 400, body: { error: 'amount_minor must be a positive integer' } };
  }

  const paymentIntentId = request.metadata?.payment_intent_id;
  if (paymentIntentId === undefined || paymentIntentId.trim() === '') {
    return { statusCode: 400, body: { error: 'metadata.payment_intent_id is required' } };
  }

  const stripeKey = stripeTestKeyFromEnv();
  if (stripeKey === null) {
    return {
      statusCode: 200,
      body: providerSuccess({
        providerReference: `simulated_re_${request.action_id}`,
        providerStatus: 'succeeded',
        mode: 'simulated',
      }),
    };
  }

  const refund = await createStripeRefund({
    secretKey: stripeKey,
    paymentIntentId,
    amountMinor,
    reason: request.metadata?.reason ?? 'customer_request',
    idempotencyKey: request.action_id,
    fetchImpl,
  });
  return {
    statusCode: 200,
    body: providerSuccess({
      providerReference: refund.id,
      providerStatus: refund.status,
      mode: 'stripe-test',
      stripeRefundId: refund.id,
    }),
  };
}

export function providerApiKey(
  raw = process.env.STRIPE_REFUND_PROVIDER_API_KEY,
  nodeEnv = process.env.NODE_ENV,
): string {
  const key = raw?.trim() || DEFAULT_PROVIDER_API_KEY;
  if (nodeEnv === 'production' && key.length < 32) {
    throw new Error('STRIPE_REFUND_PROVIDER_API_KEY must contain at least 32 characters');
  }
  return key;
}

export function isValidProviderAuthorization(
  authorization: string | undefined,
  secret = providerApiKey(),
): boolean {
  if (authorization === undefined) return false;
  const actual = Buffer.from(authorization);
  const expected = Buffer.from(`Bearer ${secret}`);
  return actual.length === expected.length && timingSafeEqual(actual, expected);
}

function providerSuccess(input: {
  providerReference: string;
  providerStatus: string;
  mode: 'simulated' | 'stripe-test';
  stripeRefundId?: string;
}): StripeRefundProviderResponse {
  return {
    status: 'succeeded',
    provider_status: input.providerStatus,
    provider_reference: input.providerReference,
    reversal_capability: 'manual_recovery',
    recovery_status: 'manual_required',
    mode: input.mode,
    stripe_refund_id: input.stripeRefundId,
  };
}
