import type {
  PolicyDocument,
  PolicyListResponse,
  PolicySetEnabledRequest,
  PolicySummary,
  PolicyValidateResponse,
  PolicyValidationIssue,
  Severity,
} from '@trustloopguard/sdk';
import { z } from 'zod';
import { http } from './http';
import { policyDraftSchema, type PolicyDraft } from './policy-draft';

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

const policyValidationIssueSchema = z.object({
  path: z.string(),
  message: z.string(),
}) satisfies z.ZodType<PolicyValidationIssue>;

export const policySummarySchema: z.ZodType<PolicySummary> =
  policySummaryWireSchema.transform(toPolicySummary);

const policyDocumentSchema: z.ZodType<PolicyDocument> =
  policyDocumentWireSchema.transform(toPolicyDocument);

const policyValidateResponseSchema: z.ZodType<PolicyValidateResponse> = z
  .object({
    valid: z.boolean(),
    policy_id: z.string().nullable().optional(),
    errors: z.array(policyValidationIssueSchema),
  })
  .transform(
    (result): PolicyValidateResponse => ({
      valid: result.valid,
      errors: result.errors,
      ...(result.policy_id !== undefined && result.policy_id !== null
        ? { policy_id: result.policy_id }
        : {}),
    }),
  );

const policyListResponseSchema: z.ZodType<PolicyListResponse> = z.object({
  policies: z.array(policySummarySchema),
});

const generatePolicyDraftResponseSchema = z.object({
  draft: policyDraftSchema,
});

export async function listPolicies(signal?: AbortSignal): Promise<PolicyListResponse> {
  return http.get('/api/policies', policyListResponseSchema, { signal });
}

export async function listPoliciesForAgent(
  agentId: string,
  signal?: AbortSignal,
): Promise<PolicyListResponse> {
  return http.get(
    `/api/policies?agentid=${encodeURIComponent(agentId)}`,
    policyListResponseSchema,
    { signal },
  );
}

export async function getPolicy(policyId: string, signal?: AbortSignal): Promise<PolicyDocument> {
  return http.get(`/api/policies/${encodeURIComponent(policyId)}`, policyDocumentSchema, {
    signal,
  });
}

export async function validatePolicy(
  sourceYaml: string,
  signal?: AbortSignal,
): Promise<PolicyValidateResponse> {
  return http.post('/api/policies/validate', sourceYaml, policyValidateResponseSchema, {
    contentType: 'application/yaml',
    signal,
  });
}

export async function upsertPolicy(
  sourceYaml: string,
  signal?: AbortSignal,
): Promise<PolicyDocument> {
  return http.post('/api/policies', sourceYaml, policyDocumentSchema, {
    contentType: 'application/yaml',
    signal,
  });
}

export async function setPolicyEnabled(
  policyId: string,
  enabled: boolean,
  signal?: AbortSignal,
): Promise<PolicyDocument> {
  const body = { enabled } satisfies PolicySetEnabledRequest;
  return http.patch(
    `/api/policies/${encodeURIComponent(policyId)}/enabled`,
    body,
    policyDocumentSchema,
    { signal },
  );
}

export async function deletePolicy(policyId: string, signal?: AbortSignal): Promise<void> {
  await http.delete(`/api/policies/${encodeURIComponent(policyId)}`, { signal });
}

export async function generatePolicyDraft(
  prompt: string,
  signal?: AbortSignal,
): Promise<PolicyDraft> {
  const result = await http.post(
    '/api/policies/generate',
    { prompt },
    generatePolicyDraftResponseSchema,
    { signal },
  );
  return result.draft;
}

export type PolicyValidationResult = z.infer<typeof policyValidateResponseSchema>;

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
