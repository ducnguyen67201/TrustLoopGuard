import type {
  PolicyDocument,
  PolicyListResponse,
  PolicySummary,
  PolicyValidateResponse,
  PolicyValidationIssue,
  Severity,
} from '@trustloopguard/sdk';
import { z } from 'zod';

const severitySchema = z.enum(['low', 'medium', 'high', 'critical']) satisfies z.ZodType<Severity>;

const policySummarySchema = z.object({
  id: z.string(),
  description: z.string().optional(),
  severity: severitySchema,
  enabled: z.boolean(),
});

const policyDocumentSchema = policySummarySchema.extend({
  source_yaml: z.string(),
});

const policyValidationIssueSchema = z.object({
  path: z.string(),
  message: z.string(),
}) satisfies z.ZodType<PolicyValidationIssue>;

const policyValidateResponseSchema = z.object({
  valid: z.boolean(),
  errors: z.array(policyValidationIssueSchema),
}) satisfies z.ZodType<PolicyValidateResponse>;

const policyListResponseSchema = z.object({
  policies: z.array(policySummarySchema),
});

function buildInit(init: Omit<RequestInit, 'signal'>, signal?: AbortSignal): RequestInit {
  return signal !== undefined ? { ...init, signal } : init;
}

async function callJson<T>(
  schema: z.ZodType<T>,
  input: string,
  init: RequestInit,
): Promise<T> {
  const res = await fetch(input, init);
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    let message = body;
    try {
      const parsed = JSON.parse(body) as { error?: string };
      if (typeof parsed.error === 'string') message = parsed.error;
    } catch {
      /* not JSON */
    }
    throw new Error(message || `${res.status} ${res.statusText}`);
  }
  return schema.parse(await res.json());
}

export async function listPolicies(signal?: AbortSignal): Promise<PolicyListResponse> {
  const parsed = await callJson(policyListResponseSchema, '/api/policies', buildInit({}, signal));
  return { policies: parsed.policies.map(toPolicySummary) };
}

export async function getPolicy(policyId: string, signal?: AbortSignal): Promise<PolicyDocument> {
  return toPolicyDocument(
    await callJson(
      policyDocumentSchema,
      `/api/policies/${encodeURIComponent(policyId)}`,
      buildInit({}, signal),
    ),
  );
}

export async function validatePolicy(
  sourceYaml: string,
  signal?: AbortSignal,
): Promise<PolicyValidateResponse> {
  return callJson(
    policyValidateResponseSchema,
    '/api/policies/validate',
    buildInit(
      {
        method: 'POST',
        headers: { 'content-type': 'application/yaml' },
        body: sourceYaml,
      },
      signal,
    ),
  );
}

export async function upsertPolicy(
  sourceYaml: string,
  signal?: AbortSignal,
): Promise<PolicyDocument> {
  return toPolicyDocument(
    await callJson(
      policyDocumentSchema,
      '/api/policies',
      buildInit(
        {
          method: 'POST',
          headers: { 'content-type': 'application/yaml' },
          body: sourceYaml,
        },
        signal,
      ),
    ),
  );
}

export async function setPolicyEnabled(
  policyId: string,
  enabled: boolean,
  signal?: AbortSignal,
): Promise<PolicyDocument> {
  return toPolicyDocument(
    await callJson(
      policyDocumentSchema,
      `/api/policies/${encodeURIComponent(policyId)}/enabled`,
      buildInit(
        {
          method: 'PATCH',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ enabled }),
        },
        signal,
      ),
    ),
  );
}

export async function deletePolicy(policyId: string, signal?: AbortSignal): Promise<void> {
  const res = await fetch(
    `/api/policies/${encodeURIComponent(policyId)}`,
    buildInit({ method: 'DELETE' }, signal),
  );
  if (!res.ok && res.status !== 204) {
    const body = await res.text().catch(() => '');
    throw new Error(body || `${res.status} ${res.statusText}`);
  }
}

export type PolicyValidationResult = z.infer<typeof policyValidateResponseSchema>;

type ParsedPolicySummary = z.infer<typeof policySummarySchema>;
type ParsedPolicyDocument = z.infer<typeof policyDocumentSchema>;

function toPolicySummary(policy: ParsedPolicySummary): PolicySummary {
  return {
    id: policy.id,
    ...(policy.description !== undefined ? { description: policy.description } : {}),
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
