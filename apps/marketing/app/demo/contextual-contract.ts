import { z } from 'zod';

const effectSchema = z.enum(['permit', 'transform', 'deny', 'require_approval', 'defer']);
const severitySchema = z.enum(['low', 'medium', 'high', 'critical']);

const historyItemSchema = z
  .object({
    role: z.enum(['user', 'assistant']),
    content: z.string().trim().min(1).max(1_000),
  })
  .strict();

const contextualDemoRequestSchema = z
  .object({
    locale: z.enum(['en', 'vi']).default('en'),
    sessionId: z.string().uuid('Session ID must be a UUID.'),
    message: z.string().trim().min(1, 'Message is required.').max(500),
    history: z.array(historyItemSchema).max(8),
  })
  .strict()
  .superRefine((request, context) => {
    const totalCharacters = request.history.reduce(
      (total, item) => total + item.content.length,
      0,
    );
    if (totalCharacters > 4_000) {
      context.addIssue({
        code: 'custom',
        path: ['history'],
        message: 'History must contain 4,000 characters or fewer in total.',
      });
    }
  });

const findingSchema = z
  .object({
    policyId: z.string().max(200).optional(),
    effect: effectSchema,
    severity: severitySchema,
    reason: z.string().max(500),
  })
  .strict();

const checkFields = {
  status: z.enum(['checked', 'skipped', 'unavailable']),
  effect: effectSchema.optional(),
  reason: z.string().max(500).optional(),
  traceId: z.string().max(200).optional(),
  latencyMs: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER).optional(),
  findings: z.array(findingSchema).max(12),
};

const inputCheckSchema = z.object({ phase: z.literal('input'), ...checkFields }).strict();
const outputCheckSchema = z.object({ phase: z.literal('output'), ...checkFields }).strict();

const policySchema = z
  .object({
    id: z.string().max(200),
    description: z.string().max(300).optional(),
    severity: severitySchema,
    action: z.string().max(100).optional(),
    phase: z.enum(['input', 'output']),
    enabled: z.literal(true),
  })
  .strict();

const runtimeSchema = z
  .object({
    agent: z.literal('openai-responses'),
    guard: z.literal('featherlane-ai-rust-api'),
    workspace: z.literal('shared-contextual-demo'),
    data: z.literal('synthetic-only'),
  })
  .strict();

const contextualDemoResponseSchema = z
  .object({
    reply: z.string().min(1).max(2_000),
    modelCalled: z.boolean(),
    checks: z.tuple([inputCheckSchema, outputCheckSchema]),
    policies: z.array(policySchema).max(20),
    runtime: runtimeSchema,
  })
  .strict();

const contextualPolicyInventorySchema = z
  .object({
    policies: z.array(policySchema).max(20),
    source: z.literal('rust'),
    runtime: runtimeSchema,
  })
  .strict();

export type ContextualDemoRequest = z.infer<typeof contextualDemoRequestSchema>;
export type ContextualDemoResponse = z.infer<typeof contextualDemoResponseSchema>;
export type ContextualPolicy = z.infer<typeof policySchema>;

export function parseContextualDemoRequest(input: z.input<typeof contextualDemoRequestSchema>) {
  return contextualDemoRequestSchema.parse(input);
}

export function sanitizeContextualDemoResponse(
  input: z.input<typeof contextualDemoResponseSchema>,
): ContextualDemoResponse {
  return contextualDemoResponseSchema.parse(input);
}

export function sanitizeContextualPolicyInventory(
  input: z.input<typeof contextualPolicyInventorySchema>,
) {
  return contextualPolicyInventorySchema.parse(input);
}
