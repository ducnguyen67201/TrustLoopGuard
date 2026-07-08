/**
 * Client for the Rust harden endpoint.
 *
 * Synthesis + verification are owned by Rust (`POST /v1/redteam/jobs/{id}/harden`):
 * it classifies each landed attack, synthesizes a guardrail generalized to the
 * leak's class, and verifies it before recommending. This module is the thin
 * typed wrapper the dashboard calls. The previous client-side template synthesis
 * was removed — guardrail business logic must not live in the web app.
 */
import { z } from 'zod';

import { http } from './http';

// Mirrors the Rust wire types (tl-core: VerifyResult / HardenCandidate /
// HardenResponse). Validated at the boundary; `description` is optional (Rust
// omits it when absent and never serializes null).
const verifyResultSchema = z.object({
  blocked_landed: z.number(),
  landed_total: z.number(),
  blocked_variants: z.number(),
  variant_total: z.number(),
  false_blocks: z.number(),
  control_total: z.number(),
  passed: z.boolean(),
});

const policyDocumentSchema = z.object({
  id: z.string(),
  description: z.string().optional(),
  severity: z.enum(['low', 'medium', 'high', 'critical']),
  enabled: z.boolean(),
  source_yaml: z.string(),
});

const hardenCandidateOperationSchema = z.enum(['create', 'tighten']);
const sideEffectClassSchema = z.enum([
  'none',
  'read',
  'external_communication',
  'file_write',
  'shell_exec',
  'network_call',
  'db_mutation',
  'api_mutation',
  'memory_write',
  'publish',
]);
const approvalRuleSchema = z.object({
  required: z.boolean(),
  approver_roles: z.array(z.string()).optional(),
  reason: z.string().optional(),
});
export const toolMetadataSchema = z.object({
  tool: z.string(),
  side_effect: sideEffectClassSchema,
  reversible: z.boolean(),
  params: z.array(z.unknown()).optional(),
  approval: approvalRuleSchema.optional(),
  sandbox_hint: z.unknown().optional(),
});
const eventVerifyResultSchema = z.object({
  escalated_landed: z.number(),
  landed_total: z.number(),
  false_blocks: z.number(),
  control_total: z.number(),
  passed: z.boolean(),
});
const originSchema = z.enum([
  'user',
  'system',
  'tool',
  'memory',
  'file',
  'web',
  'email',
  'api',
  'unknown',
]);
const trustSchema = z.enum(['trusted', 'untrusted', 'unknown']);
const confidentialitySchema = z.enum(['public', 'private', 'secret', 'identity', 'unknown']);
const integritySchema = z.enum(['low', 'medium', 'high', 'unknown']);
const sourceLabelPolicySchema = z.object({
  origin: originSchema,
  trust: trustSchema.optional(),
  confidentiality: confidentialitySchema.optional(),
  integrity: integritySchema.optional(),
});
const regressionCaseSchema = z.object({
  id: z.string(),
  case_key: z.string(),
  environment_id: z.string(),
  agent_id: z.string().optional(),
  source: z.enum(['harden', 'manual']),
  source_job_id: z.string().optional(),
  source_session_seqs: z.array(z.number()),
  substrate: z.string(),
  artifact_id: z.string(),
  expected_outcome: z.enum(['block', 'escalate', 'stop']),
  attack: z.string(),
  goal: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
});
const hardenRejectionReasonSchema = z.enum([
  'no_target_reply',
  'synthesis_invalid',
  'missed_landed',
  'missed_variant',
  'false_blocked_control',
  'semantic_judge_unavailable',
  'unreachable_substrate',
]);

const hardenCandidateSchema = z.object({
  policy: policyDocumentSchema,
  operation: hardenCandidateOperationSchema,
  existing_policy_id: z.string().optional(),
  substrate: z.string(),
  evidence_seqs: z.array(z.number()),
  source: z.string(),
  verify: verifyResultSchema,
});

const hardenEventCandidateSchema = z.object({
  tool_metadata: toolMetadataSchema,
  operation: hardenCandidateOperationSchema,
  existing_tool_id: z.string().optional(),
  substrate: z.string(),
  evidence_seqs: z.array(z.number()),
  source: z.string(),
  verify: eventVerifyResultSchema,
});

const hardenLabelPolicyCandidateSchema = z.object({
  label_policy: sourceLabelPolicySchema,
  operation: hardenCandidateOperationSchema,
  existing_origin: originSchema.optional(),
  substrate: z.string(),
  evidence_seqs: z.array(z.number()),
  source: z.string(),
  verify: eventVerifyResultSchema,
});

const toolMetadataEntrySchema = z.object({
  metadata: toolMetadataSchema,
  enabled: z.boolean(),
});

const hardenRejectionSchema = z.object({
  reason: hardenRejectionReasonSchema,
  substrate: z.string(),
  evidence_seqs: z.array(z.number()),
  verify: verifyResultSchema.optional(),
  message: z.string(),
});

const hardenResponseSchema = z.object({
  candidates: z.array(hardenCandidateSchema),
  event_candidates: z.array(hardenEventCandidateSchema).optional(),
  label_policy_candidates: z.array(hardenLabelPolicyCandidateSchema).optional(),
  rejections: z.array(hardenRejectionSchema),
  unreachable: z.array(z.string()),
  regression_cases: z.array(regressionCaseSchema).optional(),
  generated_at: z.string(),
});

export type HardenCandidate = z.infer<typeof hardenCandidateSchema>;
export type HardenEventCandidate = z.infer<typeof hardenEventCandidateSchema>;
export type HardenLabelPolicyCandidate = z.infer<typeof hardenLabelPolicyCandidateSchema>;
export type HardenRejection = z.infer<typeof hardenRejectionSchema>;
export type HardenResponse = z.infer<typeof hardenResponseSchema>;
export type SourceLabelPolicy = z.infer<typeof sourceLabelPolicySchema>;
export type ToolMetadata = z.infer<typeof toolMetadataSchema>;

/**
 * Synthesize + verify guardrail candidates for a job. `persist` saves the
 * survivors `enabled=false` (the operator opts in separately); `false` previews.
 */
export async function hardenJob(
  jobId: string,
  persist: boolean,
  signal?: AbortSignal,
): Promise<HardenResponse> {
  return http.post(
    `/api/redteam/jobs/${encodeURIComponent(jobId)}/harden`,
    { persist },
    hardenResponseSchema,
    { signal },
  );
}

export async function upsertToolMetadata(
  metadata: ToolMetadata,
  enabled: boolean,
  signal?: AbortSignal,
): Promise<void> {
  await http.post(
    '/api/tool-metadata',
    { ...metadata, enabled },
    toolMetadataEntrySchema,
    { signal },
  );
}

export async function upsertLabelPolicy(
  policy: SourceLabelPolicy,
  enabled: boolean,
  signal?: AbortSignal,
): Promise<void> {
  await http.post('/api/label-policies', { policy, enabled }, z.unknown(), { signal });
}
