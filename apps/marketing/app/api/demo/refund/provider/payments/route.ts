import {
  handleProviderPayment,
  isValidProviderAuthorization,
  type ProviderReply,
} from '@featherlane-ai/demo/stripe-refund-agent/provider-adapter';
import type { StripeRefundProviderRequest } from '@featherlane-ai/demo/stripe-refund-agent/types';
import { NextResponse } from 'next/server';
import { z } from 'zod';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

const providerRequestSchema = z.object({
  action_id: z.string().trim().min(1).max(200),
  kind: z.string().trim().min(1).max(50),
  amount: z.number().int().positive().optional(),
  amount_minor: z.number().int().positive().optional(),
  currency: z.string().trim().min(1).max(10),
  memo: z.string().max(1_000).optional(),
  metadata: z
    .object({
      payment_intent_id: z.string().trim().min(1).max(200).optional(),
      order_id: z.string().trim().min(1).max(100).optional(),
      reason: z.string().trim().min(1).max(200).optional(),
    })
    .optional(),
});

interface ProviderPaymentsDependencies {
  authorize?: (authorization: string | undefined) => boolean;
  handlePayment: (
    authorization: string | undefined,
    request: StripeRefundProviderRequest,
  ) => Promise<ProviderReply>;
}

export function createProviderPaymentsHandler(
  dependencies: ProviderPaymentsDependencies = {
    authorize: isValidProviderAuthorization,
    handlePayment: handleProviderPayment,
  },
) {
  const authorize = dependencies.authorize ?? isValidProviderAuthorization;

  return async function POST(request: Request) {
    const authorization = request.headers.get('authorization') ?? undefined;
    try {
      if (!authorize(authorization)) {
        return NextResponse.json({ error: 'unauthorized' }, { status: 401 });
      }
    } catch (error) {
      console.error('refund provider authentication failed', safeErrorForLog(error));
      return NextResponse.json({ error: 'provider unavailable' }, { status: 503 });
    }

    let body: unknown;
    try {
      body = await request.json();
    } catch {
      return NextResponse.json({ error: 'invalid provider request' }, { status: 400 });
    }
    const parsed = providerRequestSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: 'invalid provider request' }, { status: 400 });
    }

    try {
      const reply = await dependencies.handlePayment(
        authorization,
        parsed.data as StripeRefundProviderRequest,
      );
      return NextResponse.json(reply.body, { status: reply.statusCode });
    } catch (error) {
      console.error('refund provider callback failed', safeErrorForLog(error));
      return NextResponse.json({ error: 'provider request failed' }, { status: 500 });
    }
  };
}

export const POST = createProviderPaymentsHandler();

function safeErrorForLog(error: unknown): string {
  if (!(error instanceof Error)) return 'unknown error';
  return `${error.name}: ${error.message}`.slice(0, 500);
}
