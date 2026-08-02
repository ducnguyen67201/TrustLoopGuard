import type { StripeRefundResult } from './types';

export interface StripeFetch {
  (input: string, init: RequestInit): Promise<Response>;
}

export interface CreateRefundInput {
  secretKey: string;
  paymentIntentId: string;
  amountMinor: number;
  reason: string;
  idempotencyKey: string;
  fetchImpl?: StripeFetch;
}

export interface CreatePaymentIntentInput {
  secretKey: string;
  amountMinor: number;
  metadata: Record<string, string>;
  idempotencyKey?: string;
  fetchImpl?: StripeFetch;
}

interface StripeApiObject {
  id?: string;
  status?: string;
  error?: { message?: string };
}

export function stripeTestKeyFromEnv(): string | null {
  const key = process.env.STRIPE_SECRET_KEY?.trim();
  if (key === undefined || key === '') return null;
  return requireStripeTestKey(key);
}

export function requireStripeTestKey(key: string): string {
  if (!key.startsWith('sk_test_')) {
    throw new Error('refusing to run: STRIPE_SECRET_KEY must be a test-mode key (sk_test_...)');
  }
  return key;
}

export async function createStripeRefund(input: CreateRefundInput): Promise<StripeRefundResult> {
  requireStripeTestKey(input.secretKey);
  if (!Number.isInteger(input.amountMinor) || input.amountMinor <= 0) {
    throw new Error('refund amount must be a positive integer minor-unit amount');
  }

  const body = new URLSearchParams({
    payment_intent: input.paymentIntentId,
    amount: String(input.amountMinor),
  });
  body.set('metadata[featherlane_ai_reason]', input.reason);

  const json = await stripePost(
    'https://api.stripe.com/v1/refunds',
    input.secretKey,
    body,
    input.idempotencyKey,
    input.fetchImpl ?? fetch,
  );
  if (json.id === undefined) throw new Error('stripe refund response did not include an id');
  return { id: json.id, status: json.status ?? 'unknown' };
}

export async function createTestPaymentIntent(
  input: CreatePaymentIntentInput,
): Promise<{ id: string; status: string }> {
  requireStripeTestKey(input.secretKey);
  if (!Number.isInteger(input.amountMinor) || input.amountMinor <= 0) {
    throw new Error('payment intent amount must be a positive integer minor-unit amount');
  }

  const body = new URLSearchParams({
    amount: String(input.amountMinor),
    currency: 'usd',
    confirm: 'true',
    payment_method: 'pm_card_visa',
    'automatic_payment_methods[enabled]': 'true',
    'automatic_payment_methods[allow_redirects]': 'never',
    description: 'Featherlane AI Stripe refund agent demo order',
  });
  for (const [key, value] of Object.entries(input.metadata)) {
    body.set(`metadata[${key}]`, value);
  }

  const json = await stripePost(
    'https://api.stripe.com/v1/payment_intents',
    input.secretKey,
    body,
    input.idempotencyKey ?? `featherlane-ai-demo-pi:${input.metadata.order_id ?? 'order'}`,
    input.fetchImpl ?? fetch,
  );
  if (json.id === undefined) throw new Error('stripe payment intent response did not include an id');
  return { id: json.id, status: json.status ?? 'unknown' };
}

async function stripePost(
  url: string,
  secretKey: string,
  body: URLSearchParams,
  idempotencyKey: string,
  fetchImpl: StripeFetch,
): Promise<StripeApiObject> {
  const res = await fetchImpl(url, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${secretKey}`,
      'content-type': 'application/x-www-form-urlencoded',
      'idempotency-key': idempotencyKey,
    },
    body,
  });
  const json = (await res.json().catch(() => ({}))) as StripeApiObject;
  if (!res.ok) {
    throw new Error(`stripe: ${json.error?.message ?? `HTTP ${res.status}`}`);
  }
  return json;
}
