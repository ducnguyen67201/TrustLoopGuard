import { z } from 'zod';

export const PROCUREMENT_POLICY_IDS = [
  'procurement-approved-suppliers',
  'procurement-high-value-review',
  'procurement-restricted-categories',
] as const;

const policyIdSchema = z.enum(PROCUREMENT_POLICY_IDS);

const requestSchema = z
  .object({
    prompt: z
      .string()
      .trim()
      .min(1, 'Prompt is required.')
      .max(500, 'Prompt must be 500 characters or fewer.'),
    activePolicyIds: z
      .array(policyIdSchema)
      .max(3)
      .default([...PROCUREMENT_POLICY_IDS]),
  })
  .strict()
  .transform((request) => {
    const selected = new Set(request.activePolicyIds);
    return {
      prompt: request.prompt,
      activePolicyIds: PROCUREMENT_POLICY_IDS.filter((policyId) => selected.has(policyId)),
    };
  });

const traceSchema = z.object({
  tool: z.enum(['search_catalog', 'submit_purchase_order']),
  summary: z.string().max(1_000),
});

const findingSchema = z.object({
  id: z.string().max(200),
  effect: z.enum(['permit', 'deny', 'transform', 'require_approval', 'defer']),
  reason: z.string().max(1_000),
  severity: z.enum(['low', 'medium', 'high', 'critical']),
  policyId: z.string().max(200).optional(),
});

const decisionSchema = z.object({
  traceId: z.string().max(200),
  effect: z.enum(['permit', 'deny', 'transform', 'require_approval', 'defer']),
  reason: z.string().max(1_000),
  latencyMs: z.number().int().nonnegative(),
  findings: z.array(findingSchema).max(5),
  approvalId: z.string().max(200).optional(),
});

const purchaseOrderSchema = z.object({
  id: z.string().max(200),
  quoteId: z.enum([
    'quote-approved-chairs',
    'quote-high-value-laptops',
    'quote-unapproved-supplies',
    'quote-restricted-gift-cards',
  ]),
  supplierName: z.string().max(200),
  itemName: z.string().max(200),
  quantity: z.number().int().positive(),
  totalMinor: z.number().int().nonnegative(),
  currency: z.literal('USD'),
  status: z.literal('submitted'),
});

const publicPolicySchema = z.object({
  id: policyIdSchema,
  title: z.string().max(100),
  description: z.string().max(300),
  effect: z.enum(['deny', 'require_approval']),
  enabled: z.boolean(),
});

const responseSchema = z.object({
  result: z.object({
    finalMessage: z.string().max(2_000),
    traces: z.array(traceSchema).max(12),
    decision: decisionSchema.optional(),
  }),
  state: z.object({
    purchaseOrders: z.array(purchaseOrderSchema).max(1),
  }),
  activePolicies: z.array(publicPolicySchema).length(3),
  runtime: z.object({
    agent: z.literal('openai-agents-js'),
    guard: z.literal('trustloopguard-rust-api'),
    provider: z.literal('simulated-procurement-api'),
  }),
});

export type JsonValue = null | boolean | number | string | JsonValue[] | JsonObject;
export interface JsonObject {
  [key: string]: JsonValue;
}

export type ProcurementPolicyId = z.infer<typeof policyIdSchema>;
export type ProcurementDemoRequest = z.infer<typeof requestSchema>;
export type ProcurementDemoResponse = z.infer<typeof responseSchema>;

export function parseProcurementDemoRequest(input: JsonValue): ProcurementDemoRequest {
  return requestSchema.parse(input);
}

export function sanitizeProcurementDemoResponse(input: JsonValue): ProcurementDemoResponse {
  return responseSchema.parse(input);
}
