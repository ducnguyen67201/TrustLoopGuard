import { Client } from '@trustloopguard/sdk';
import type {
  PolicyDocument,
  PolicyListResponse,
  PolicySummary,
  PolicyValidateResponse,
  PolicyValidationIssue,
  Severity,
} from '@trustloopguard/sdk';
import { z } from 'zod';
import { getServerUrl } from './server-url';

let cachedClient: Client | null = null;

function getClient(): Client {
  if (cachedClient !== null) return cachedClient;
  cachedClient = new Client({ baseUrl: getServerUrl() });
  return cachedClient;
}

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

export async function listPolicies(signal?: AbortSignal): Promise<PolicyListResponse> {
  const parsed = policyListResponseSchema.parse(await getClient().listPolicies(signal));
  return {
    policies: parsed.policies.map(toPolicySummary),
  };
}

export async function getPolicy(policyId: string, signal?: AbortSignal): Promise<PolicyDocument> {
  return toPolicyDocument(
    policyDocumentSchema.parse(await getClient().getPolicy(policyId, signal)),
  );
}

export async function validatePolicy(
  sourceYaml: string,
  signal?: AbortSignal,
): Promise<PolicyValidateResponse> {
  return policyValidateResponseSchema.parse(await getClient().validatePolicy(sourceYaml, signal));
}

export async function upsertPolicy(
  sourceYaml: string,
  signal?: AbortSignal,
): Promise<PolicyDocument> {
  return toPolicyDocument(
    policyDocumentSchema.parse(await getClient().upsertPolicy(sourceYaml, signal)),
  );
}

export async function setPolicyEnabled(
  policyId: string,
  enabled: boolean,
  signal?: AbortSignal,
): Promise<PolicyDocument> {
  return toPolicyDocument(
    policyDocumentSchema.parse(await getClient().setPolicyEnabled(policyId, enabled, signal)),
  );
}

export async function deletePolicy(policyId: string, signal?: AbortSignal): Promise<void> {
  await getClient().deletePolicy(policyId, signal);
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
