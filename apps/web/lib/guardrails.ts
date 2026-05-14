import type {
  GuardrailGenerateResponse,
  GuardrailListResponse,
  PolicyDocument,
  PolicySummary,
  Severity,
} from '@trustloopguard/sdk';
import { z } from 'zod';
import { http } from './http';

// Same severity shape as lib/policies.ts; duplicated here to keep the
// modules independent. If it grows a third copy, lift to a shared file.
const severitySchema = z.enum(['low', 'medium', 'high', 'critical']) satisfies z.ZodType<Severity>;

const policySummaryWireSchema = z.object({
  id: z.string(),
  description: z.string().nullable().optional(),
  severity: severitySchema,
  enabled: z.boolean(),
});

const policyDocumentWireSchema = policySummaryWireSchema.extend({
  source_yaml: z.string(),
});

const guardrailListResponseSchema: z.ZodType<GuardrailListResponse> = z.object({
  policies: z.array(policySummaryWireSchema.transform(toPolicySummary)),
});

const guardrailGenerateResponseSchema: z.ZodType<GuardrailGenerateResponse> = z.object({
  generated: z.array(policyDocumentWireSchema.transform(toPolicyDocument)),
});

export async function listGuardrails(
  agentId: string,
  signal?: AbortSignal,
): Promise<GuardrailListResponse> {
  return http.get(
    `/api/agents/${encodeURIComponent(agentId)}/guardrails`,
    guardrailListResponseSchema,
    { signal },
  );
}

export async function generateGuardrails(
  agentId: string,
  signal?: AbortSignal,
): Promise<GuardrailGenerateResponse> {
  return http.post(
    `/api/agents/${encodeURIComponent(agentId)}/guardrails/generate`,
    null,
    guardrailGenerateResponseSchema,
    { signal },
  );
}

type ParsedPolicySummary = z.infer<typeof policySummaryWireSchema>;
type ParsedPolicyDocument = z.infer<typeof policyDocumentWireSchema>;

function toPolicySummary(policy: ParsedPolicySummary): PolicySummary {
  return {
    id: policy.id,
    ...(typeof policy.description === 'string' ? { description: policy.description } : {}),
    severity: policy.severity,
    enabled: policy.enabled,
  };
}

function toPolicyDocument(policy: ParsedPolicyDocument): PolicyDocument {
  return {
    ...toPolicySummary(policy),
    source_yaml: policy.source_yaml,
  };
}
