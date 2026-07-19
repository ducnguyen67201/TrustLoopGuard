import { z } from 'zod';

const effectSchema = z.enum(['permit', 'transform', 'deny', 'require_approval', 'defer']);
const severitySchema = z.enum(['low', 'medium', 'high', 'critical']);

const historyItemSchema = z.object({
  role: z.enum(['user', 'assistant']),
  content: z
    .string()
    .trim()
    .min(1, 'History messages cannot be empty.')
    .max(1_000, 'History messages must be 1,000 characters or fewer.'),
});

const healthcareDemoRequestSchema = z
  .object({
    locale: z.enum(['en', 'vi']).default('en'),
    sessionId: z.string().uuid('Session ID must be a UUID.'),
    message: z
      .string()
      .trim()
      .min(1, 'Message is required.')
      .max(500, 'Message must be 500 characters or fewer.'),
    history: z.array(historyItemSchema).max(8, 'History may contain at most eight messages.'),
  })
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

const findingSchema = z.object({
  policyId: z.string().max(200).optional(),
  effect: effectSchema,
  severity: severitySchema,
  reason: z.string().max(500),
});

const policyPhaseSchema = z.enum(['input', 'output']);

const checkFields = {
  status: z.enum(['checked', 'skipped', 'unavailable']),
  effect: effectSchema.optional(),
  reason: z.string().max(500).optional(),
  traceId: z.string().max(200).optional(),
  latencyMs: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER).optional(),
  findings: z.array(findingSchema).max(12),
};

const inputCheckSchema = z.object({
  phase: z.literal('input'),
  ...checkFields,
});

const outputCheckSchema = z.object({
  phase: z.literal('output'),
  ...checkFields,
});

const policyFields = {
  id: z.string().max(200),
  description: z.string().max(300).optional(),
  severity: severitySchema,
  action: z.string().max(100).optional(),
  phase: policyPhaseSchema.optional(),
};

const activePolicySchema = z.object({
  ...policyFields,
  enabled: z.literal(true),
});

const previewPolicySchema = z.object({
  ...policyFields,
  phase: policyPhaseSchema,
  enabled: z.literal(false),
});

const runtimeSchema = z.object({
  agent: z.literal('openai-responses'),
  guard: z.literal('trustloopguard-rust-api'),
  data: z.literal('synthetic-only'),
});

const healthcareDemoResponseSchema = z.object({
  reply: z.string().min(1).max(2_000),
  modelCalled: z.boolean(),
  checks: z.tuple([inputCheckSchema, outputCheckSchema]),
  policies: z.array(activePolicySchema).max(20),
  runtime: runtimeSchema,
});

const healthcarePolicyInventorySchema = z.discriminatedUnion('source', [
  z.object({
    policies: z.array(activePolicySchema).max(20),
    source: z.literal('rust'),
    runtime: runtimeSchema,
  }),
  z.object({
    policies: z.array(previewPolicySchema).max(20),
    source: z.literal('demo_template'),
    runtime: runtimeSchema,
  }),
]);

export type HealthcareDemoRequest = z.infer<typeof healthcareDemoRequestSchema>;
export type HealthcareDemoResponse = z.infer<typeof healthcareDemoResponseSchema>;
export type HealthcareCheck = HealthcareDemoResponse['checks'][number];
export type HealthcarePolicyInventory = z.infer<typeof healthcarePolicyInventorySchema>;
export type HealthcarePolicy = HealthcarePolicyInventory['policies'][number];

export function parseHealthcareDemoRequest(
  input: z.input<typeof healthcareDemoRequestSchema>,
): HealthcareDemoRequest {
  return healthcareDemoRequestSchema.parse(input);
}

export function sanitizeHealthcareDemoResponse(
  input: z.input<typeof healthcareDemoResponseSchema>,
): HealthcareDemoResponse {
  return healthcareDemoResponseSchema.parse(input);
}

export function sanitizeHealthcarePolicyInventory(
  input: z.input<typeof healthcarePolicyInventorySchema>,
): HealthcarePolicyInventory {
  return healthcarePolicyInventorySchema.parse(input);
}
