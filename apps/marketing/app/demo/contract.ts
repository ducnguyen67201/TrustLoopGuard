import { z } from 'zod';

const promptSchema = z.object({
  prompt: z.string().trim().min(1, 'Prompt is required.').max(500, 'Prompt must be 500 characters or fewer.'),
});

const traceSchema = z.object({
  tool: z.enum(['search_order', 'prepare_refund', 'execute_refund']),
  summary: z.string().max(1_000),
});

const resultSchema = z.object({
  prompt: z.string().max(500),
  traces: z.array(traceSchema).max(12),
  finalMessage: z.string().max(2_000),
  actionId: z.string().max(200).optional(),
  receiptId: z.string().max(200).optional(),
});

const orderSchema = z.object({
  id: z.string().max(100),
  customerName: z.string().max(100),
  paymentMethodLast4: z.string().regex(/^\d{4}$/),
  amountPaidMinor: z.number().int().nonnegative(),
  refundableBalanceMinor: z.number().int().nonnegative(),
  currency: z.literal('USD'),
  captured: z.boolean(),
  refundWindowOpen: z.boolean(),
  refundCount: z.number().int().nonnegative(),
});

const refundSchema = z.object({
  orderId: z.string().max(100),
  financialActionId: z.string().max(200),
  amountMinor: z.number().int().positive(),
  providerReference: z.string().max(200).optional(),
  status: z.string().max(50),
  reason: z.string().max(200),
  createdAt: z.string().max(100),
});

const stateSchema = z.object({
  orders: z.array(orderSchema).max(5),
  refunds: z.array(refundSchema).max(20),
});

const logSchema = z.object({
  step: z.string().max(80),
  message: z.string().max(1_000),
});

const runtimeSchema = z.object({
  agent: z.literal('openai'),
  guard: z.literal('trustloopguard-rust-api'),
  provider: z.literal('stripe-test'),
});

const responseSchema = z.object({
  result: resultSchema,
  state: stateSchema,
  logs: z.array(logSchema).max(30),
  runtime: runtimeSchema,
});

export type RefundDemoResponse = z.infer<typeof responseSchema>;

export function parseRefundDemoPrompt(input: unknown): string {
  return promptSchema.parse(input).prompt;
}

export function sanitizeRefundDemoResponse(input: unknown): RefundDemoResponse {
  return responseSchema.parse(input);
}

export function refundDemoServiceUrl(raw = process.env['REFUND_DEMO_SERVICE_URL']): string {
  const url = new URL(raw?.trim() || 'http://127.0.0.1:9310');
  const isLoopback = ['127.0.0.1', 'localhost', '::1', '[::1]'].includes(url.hostname);
  if (url.protocol !== 'https:' && !(url.protocol === 'http:' && isLoopback)) {
    throw new Error('REFUND_DEMO_SERVICE_URL must use HTTPS or loopback HTTP');
  }
  if (url.username !== '' || url.password !== '' || url.search !== '' || url.hash !== '') {
    throw new Error('REFUND_DEMO_SERVICE_URL must be a plain service origin');
  }
  return url.toString().replace(/\/$/, '');
}
