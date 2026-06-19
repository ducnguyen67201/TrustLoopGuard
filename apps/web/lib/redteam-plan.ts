/**
 * Client for the Rust attack-vector planner + static preventive policies.
 *
 * Planning (workflow analysis + LLM grounding) and static policy synthesis are
 * owned by Rust (`POST /v1/agents/{id}/redteam/plan` and `.../static-policies`).
 * This module is the thin typed wrapper the dashboard calls; zod validates every
 * response at the boundary. The vectors come back in the Rust wire shape and are
 * forwarded verbatim into a dispatch as seeds.
 */
import { z } from 'zod';

import { http } from './http';

// Mirrors tl-core WorkflowPath / AttackVector / RedteamPlanResponse.
const workflowPathSchema = z.object({
  source_node: z.string(),
  source_type: z.string(),
  source_category: z.string(),
  sink_node: z.string(),
  sink_type: z.string(),
  sink_category: z.string(),
});

const attackVectorSchema = z.object({
  goal: z.string(),
  technique: z.string(),
  target_operation: z.string(),
  injection_payload: z.string(),
  source_path: workflowPathSchema.optional(),
});

const planResponseSchema = z.object({
  id: z.string(),
  agent_id: z.string(),
  name: z.string(),
  vectors: z.array(attackVectorSchema),
  paths: z.array(workflowPathSchema),
  unmapped_node_types: z.array(z.string()),
  generated_at: z.string(),
});

const planListResponseSchema = z.object({
  plans: z.array(planResponseSchema),
});

const policyDocumentSchema = z.object({
  id: z.string(),
  description: z.string().optional(),
  severity: z.enum(['low', 'medium', 'high', 'critical']),
  enabled: z.boolean(),
  source_yaml: z.string(),
});

const staticPoliciesResponseSchema = z.object({
  generated: z.array(policyDocumentSchema),
});

export type AttackVector = z.infer<typeof attackVectorSchema>;
export type WorkflowPath = z.infer<typeof workflowPathSchema>;
export type RedteamPlan = z.infer<typeof planResponseSchema>;
export type StaticPoliciesResult = z.infer<typeof staticPoliciesResponseSchema>;

/** Derive tailored attack vectors from the agent's definition (prompt + workflow)
 *  and save them as a named plan. */
export async function planAttackVectors(
  agentId: string,
  name?: string,
  signal?: AbortSignal,
): Promise<RedteamPlan> {
  return http.post(
    `/api/agents/${encodeURIComponent(agentId)}/redteam/plan`,
    name !== undefined && name.trim() !== '' ? { name: name.trim() } : {},
    planResponseSchema,
    { signal },
  );
}

/** List an agent's saved attack plans, newest first. */
export async function listPlans(agentId: string, signal?: AbortSignal): Promise<RedteamPlan[]> {
  const result = await http.get(
    `/api/agents/${encodeURIComponent(agentId)}/redteam/plans`,
    planListResponseSchema,
    { signal },
  );
  return result.plans;
}

/** Delete a saved attack plan by id. */
export async function deletePlan(planId: string, signal?: AbortSignal): Promise<void> {
  return http.delete(`/api/redteam/plans/${encodeURIComponent(planId)}`, { signal });
}

/** Synthesize preventive policies from the agent's workflow paths (no execution). */
export async function generateStaticPolicies(
  agentId: string,
  signal?: AbortSignal,
): Promise<StaticPoliciesResult> {
  return http.post(
    `/api/agents/${encodeURIComponent(agentId)}/redteam/static-policies`,
    {},
    staticPoliciesResponseSchema,
    { signal },
  );
}
