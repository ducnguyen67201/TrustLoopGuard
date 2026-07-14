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

const runtimeSchema = z.object({
  agent: z.literal('openai'),
  guard: z.literal('trustloopguard-rust-api'),
  provider: z.literal('stripe-test'),
});

const responseSchema = z.object({
  result: resultSchema,
  state: stateSchema,
  runtime: runtimeSchema,
});

const actionIdSchema = z.string().uuid('Action ID must be a UUID.');

const statusSchema = z.object({
  actionId: actionIdSchema,
  status: z.enum([
    'proposed',
    'authorized',
    'held',
    'executed',
    'denied',
    'failed',
    'reversed',
    'expired',
  ]),
  orderId: z.string().max(100),
  amountMinor: z.number().int().positive(),
  currency: z.literal('USD'),
  receiptId: z.string().max(200).optional(),
  providerReference: z.string().max(200).optional(),
  updatedAt: z.string().max(100),
});

export type RefundDemoResponse = z.infer<typeof responseSchema>;
export type RefundDemoStatus = z.infer<typeof statusSchema>;

export function parseRefundDemoPrompt(input: unknown): string {
  return promptSchema.parse(input).prompt;
}

export function sanitizeRefundDemoResponse(input: unknown): RefundDemoResponse {
  return responseSchema.parse(input);
}

export function parseRefundDemoActionId(input: unknown): string {
  return actionIdSchema.parse(input);
}

export function sanitizeRefundDemoStatus(input: unknown): RefundDemoStatus {
  return statusSchema.parse(input);
}
