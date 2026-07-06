import type {
  PolicyBatchSetEnabledRequest,
  PolicyBatchSetEnabledResponse,
  PolicyDocument,
  PolicyFamily,
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
const policyFamilySchema = z.enum([
  'content',
  'flow',
  'parameter_source',
  'approval',
  'memory',
  'financial',
  'source_label',
]) satisfies z.ZodType<PolicyFamily>;

const policySummaryWireSchema = z.object({
  id: z.string(),
  family: policyFamilySchema.default('content'),
  description: z.string().nullable().optional(),
  severity: severitySchema,
  action: z.string().optional(),
  enabled: z.boolean(),
  owner_agent_id: z.string().nullable().optional(),
});

const policyDocumentWireSchema = policySummaryWireSchema.extend({
  source_yaml: z.string(),
});

const policyValidationIssueSchema = z.object({
  path: z.string(),
  message: z.string(),
}) satisfies z.ZodType<PolicyValidationIssue>;

const policySummarySchema: z.ZodType<PolicySummary> =
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

const policyBatchSetEnabledResponseSchema: z.ZodType<PolicyBatchSetEnabledResponse> = z.object({
  policies: z.array(policySummarySchema),
});

const generatePolicyDraftResponseSchema = z.object({
  draft: policyDraftSchema,
});

const policyVersionSummarySchema = z.object({
  version: z.number(),
  created_at: z.string(),
});

const policyVersionListResponseSchema = z.object({
  versions: z.array(policyVersionSummarySchema),
});

const policyVersionDetailSchema = policyVersionSummarySchema.extend({
  content: z.string(),
});

const aiEditResponseSchema = z.object({
  yaml: z.string(),
});

export async function getPolicy(policyId: string, signal?: AbortSignal): Promise<PolicyDocument> {
  return http.get(
    `/api/policies/${encodeURIComponent(policyId)}`,
    policyDocumentSchema,
    { signal },
  );
}

export async function validatePolicy(
  sourceYaml: string,
  signal?: AbortSignal,
): Promise<PolicyValidateResponse> {
  return http.post(
    '/api/policies/validate',
    sourceYaml,
    policyValidateResponseSchema,
    {
      contentType: 'application/yaml',
      signal,
    },
  );
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

export async function setPoliciesEnabled(
  policyIds: string[],
  enabled: boolean,
  signal?: AbortSignal,
): Promise<PolicyBatchSetEnabledResponse> {
  const body = { ids: policyIds, enabled } satisfies PolicyBatchSetEnabledRequest;
  return http.patch(
    '/api/policies/batch/enabled',
    body,
    policyBatchSetEnabledResponseSchema,
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

interface PolicyVersionSummary {
  version: number;
  created_at: string;
}

interface PolicyVersionListResponse {
  versions: PolicyVersionSummary[];
}

interface PolicyVersionDetail {
  version: number;
  content: string;
  created_at: string;
}

export async function listPolicyVersions(
  policyId: string,
  signal?: AbortSignal,
): Promise<PolicyVersionListResponse> {
  return http.get(
    `/api/policies/${encodeURIComponent(policyId)}/versions`,
    policyVersionListResponseSchema,
    { signal },
  );
}

export async function getPolicyVersion(
  policyId: string,
  version: number,
  signal?: AbortSignal,
): Promise<PolicyVersionDetail> {
  return http.get(
    `/api/policies/${encodeURIComponent(policyId)}/versions/${version}`,
    policyVersionDetailSchema,
    { signal },
  );
}

export async function aiEditPolicy(
  yaml: string,
  instruction: string,
  signal?: AbortSignal,
): Promise<string> {
  const data = await http.post(
    '/api/policies/ai-edit',
    { yaml, instruction },
    aiEditResponseSchema,
    { signal },
  );
  return data.yaml;
}


type ParsedPolicySummary = z.infer<typeof policySummaryWireSchema>;
type ParsedPolicyDocument = z.infer<typeof policyDocumentWireSchema>;

function toPolicySummary(policy: ParsedPolicySummary): PolicySummary {
  return {
    id: policy.id,
    family: policy.family,
    ...(typeof policy.description === 'string' ? { description: policy.description } : {}),
    severity: policy.severity,
    action: policy.action ?? 'block',
    enabled: policy.enabled,
    ...(policy.owner_agent_id !== undefined && policy.owner_agent_id !== null
      ? { owner_agent_id: policy.owner_agent_id }
      : {}),
  };
}

function toPolicyDocument(policy: ParsedPolicyDocument): PolicyDocument {
  return {
    ...toPolicySummary(policy),
    source_yaml: policy.source_yaml,
  };
}
