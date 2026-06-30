// Pluggable payment executor for the money-agent demo.
//
// Called ONLY when the guard returns `allow`. With no STRIPE_SECRET_KEY it
// records to an in-memory ledger (zero setup, offline/CI friendly). With a
// test-mode key it makes one real Stripe test-mode call so "the money moved"
// is visible in the Stripe test dashboard. We hit the Stripe REST API with
// `fetch` (the idiom this demo already uses) rather than add an SDK dependency.

export interface PaymentRequest {
  kind: 'refund' | 'wire';
  /** Integer minor units (cents). */
  amountCents: number;
  destination: string;
}

export interface PaymentResult {
  /** A reference for the executed payment (Stripe id, or a sim marker). */
  ref: string;
  mode: 'simulated' | 'stripe-test';
}

interface StripePaymentIntent {
  id?: string;
  error?: { message?: string };
}

const simulatedLedger: PaymentRequest[] = [];

/** Payments recorded on the simulated path (for the scenario summary/tests). */
export function simulatedPayments(): readonly PaymentRequest[] {
  return simulatedLedger;
}

export async function executePayment(req: PaymentRequest): Promise<PaymentResult> {
  const key = process.env.STRIPE_SECRET_KEY?.trim();
  if (key === undefined || key === '') {
    simulatedLedger.push(req);
    return { ref: '[simulated]', mode: 'simulated' };
  }
  // Money-safety: never run this demo against a live key.
  if (!key.startsWith('sk_test_')) {
    throw new Error('refusing to run: STRIPE_SECRET_KEY must be a test-mode key (sk_test_…)');
  }

  const body = new URLSearchParams({
    amount: String(req.amountCents),
    currency: 'usd',
    confirm: 'true',
    payment_method: 'pm_card_visa',
    'automatic_payment_methods[enabled]': 'true',
    'automatic_payment_methods[allow_redirects]': 'never',
    description: `TrustLoopGuard demo ${req.kind} to ${req.destination}`,
  });
  const res = await fetch('https://api.stripe.com/v1/payment_intents', {
    method: 'POST',
    headers: {
      authorization: `Bearer ${key}`,
      'content-type': 'application/x-www-form-urlencoded',
    },
    body,
  });
  const json = (await res.json().catch(() => ({}))) as StripePaymentIntent;
  if (!res.ok) {
    throw new Error(`stripe: ${json.error?.message ?? `HTTP ${res.status}`}`);
  }
  return { ref: json.id ?? '(unknown)', mode: 'stripe-test' };
}
