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

const hardenCandidateSchema = z.object({
  policy: policyDocumentSchema,
  substrate: z.string(),
  evidence_seqs: z.array(z.number()),
  source: z.string(),
  verify: verifyResultSchema,
});

const hardenResponseSchema = z.object({
  candidates: z.array(hardenCandidateSchema),
  unreachable: z.array(z.string()),
  generated_at: z.string(),
});

export type HardenCandidate = z.infer<typeof hardenCandidateSchema>;
export type HardenResponse = z.infer<typeof hardenResponseSchema>;

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
