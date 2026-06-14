/**
 * Same-origin client for durable TrustLoopGuardBench runs.
 *
 * Browser code calls `/api/bench/*`, which only validates/proxies to Rust
 * `/v1/bench/*`. Rust owns parent runs, arm mappings, child red-team jobs, and
 * report aggregation; this client only translates UI input and validates the
 * Rust response shape at the web boundary.
 */
import type {
  BenchArm,
  BenchArmMetrics,
  BenchComparedCase,
  BenchReportDelta,
  BenchReportPayload,
  BenchRunArmSummary,
  BenchRunDetail,
  BenchRunStatus,
  BenchRunSummary,
  BenchTrackMetrics,
  RedteamGenerator,
} from '@trustloopguard/sdk';
import { z } from 'zod';

import {
  redteamGeneratorSchema,
  redteamJobProfileSchema,
  redteamJobSummarySchema,
  type RedteamJobProfile,
} from './redteam-jobs';

export type {
  BenchArm,
  BenchArmMetrics,
  BenchComparedCase,
  BenchReportDelta,
  BenchReportPayload,
  BenchRunArmSummary,
  BenchRunDetail,
  BenchRunStatus,
  BenchRunSummary,
  BenchTrackMetrics,
};

export const benchArmSchema = z.enum(['raw', 'guarded']);
export const benchRunStatusSchema = z.enum(['queued', 'running', 'complete', 'error', 'cancelled']);
export const comparedAttackStatusSchema = z.enum([
  'fixed',
  'still_vulnerable',
  'regressed',
  'unchanged',
]);

export const benchRunSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  environment_id: z.string(),
  status: benchRunStatusSchema,
  profile: redteamJobProfileSchema,
  generator: redteamGeneratorSchema,
  agent_id: z.string().nullable(),
  seed: z.string().nullable(),
  error: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const benchRunArmSummarySchema = z.object({
  run_id: z.string(),
  arm: benchArmSchema,
  label: z.string(),
  target: z.string(),
  redteam_job_id: z.string().nullable(),
  checker_config: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const benchRunDetailSchema = z.object({
  run: benchRunSummarySchema,
  arms: z.array(benchRunArmSummarySchema),
  raw_job: redteamJobSummarySchema.nullable(),
  guarded_job: redteamJobSummarySchema.nullable(),
});

export const benchRunListResponseSchema = z.object({
  runs: z.array(benchRunSummarySchema),
});

export const benchArmMetricsSchema = z.object({
  arm: benchArmSchema,
  attacks: z.number(),
  landed: z.number(),
  blocked: z.number(),
  clean: z.number(),
  errored: z.number(),
  attack_success_rate: z.number(),
  benign_utility_rate: z.number(),
  utility_under_attack_rate: z.number(),
  false_block_rate: z.number(),
});

export const benchTrackMetricsSchema = z.object({
  track: z.string(),
  raw: benchArmMetricsSchema,
  guarded: benchArmMetricsSchema,
});

export const benchReportDeltaSchema = z.object({
  attack_success_rate_reduction: z.number(),
  benign_utility_delta: z.number(),
  utility_under_attack_delta: z.number(),
  false_block_delta: z.number(),
});

export const benchComparedCaseSchema = z.object({
  case_id: z.string().nullable(),
  attack: z.string(),
  goal: z.string(),
  track: z.string().nullable(),
  kind: z.string().nullable(),
  raw_outcome: z.string(),
  guarded_outcome: z.string(),
  status: comparedAttackStatusSchema,
});

export const benchReportPayloadSchema = z.object({
  run: benchRunSummarySchema,
  arms: z.array(benchRunArmSummarySchema),
  raw: benchArmMetricsSchema,
  guarded: benchArmMetricsSchema,
  delta: benchReportDeltaSchema,
  tracks: z.array(benchTrackMetricsSchema),
  cases: z.array(benchComparedCaseSchema),
  generated_at: z.string(),
});

export interface CreateBenchRunInput {
  rawTargetUrl: string;
  guardedTargetUrl: string;
  profile: RedteamJobProfile;
  generator?: RedteamGenerator;
  agentId?: string;
  seed?: string;
}

export interface ListBenchRunsParams {
  limit?: number;
}

const errorEnvelopeSchema = z.object({ error: z.string() });

function messageFromBody(body: unknown, status: number): string {
  const parsed = errorEnvelopeSchema.safeParse(body);
  if (parsed.success) return parsed.data.error;
  return `benchmark request failed (HTTP ${status})`;
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text();
  if (text === '') return {};
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return {};
  }
}

async function request<S extends z.ZodTypeAny>(
  path: string,
  schema: S,
  init?: RequestInit,
): Promise<z.infer<S>> {
  const response = await fetch(`/api/bench${path}`, init);
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return schema.parse(json);
}

function createRunBody(input: CreateBenchRunInput): {
  raw_target_url: string;
  guarded_target_url: string;
  profile: RedteamJobProfile;
  generator?: RedteamGenerator;
  agent_id?: string;
  seed?: string;
} {
  const body: {
    raw_target_url: string;
    guarded_target_url: string;
    profile: RedteamJobProfile;
    generator?: RedteamGenerator;
    agent_id?: string;
    seed?: string;
  } = {
    raw_target_url: input.rawTargetUrl,
    guarded_target_url: input.guardedTargetUrl,
    profile: input.profile,
  };
  if (input.generator !== undefined) body.generator = input.generator;
  if (input.agentId !== undefined && input.agentId !== '') body.agent_id = input.agentId;
  if (input.seed !== undefined && input.seed !== '') body.seed = input.seed;
  return body;
}

function runsQuery(params?: ListBenchRunsParams): string {
  const query = new URLSearchParams();
  if (params?.limit !== undefined) query.set('limit', String(params.limit));
  const serialized = query.toString();
  return serialized === '' ? '' : `?${serialized}`;
}

export const bench = {
  createRun(input: CreateBenchRunInput): Promise<BenchRunDetail> {
    return request('/runs', benchRunDetailSchema, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(createRunBody(input)),
    });
  },

  getRun(id: string): Promise<BenchRunDetail> {
    return request(`/runs/${encodeURIComponent(id)}`, benchRunDetailSchema);
  },

  async listRuns(params?: ListBenchRunsParams): Promise<BenchRunSummary[]> {
    return (await request(`/runs${runsQuery(params)}`, benchRunListResponseSchema)).runs;
  },

  getReport(id: string): Promise<BenchReportPayload> {
    return request(`/runs/${encodeURIComponent(id)}/report`, benchReportPayloadSchema);
  },

  cancel(id: string): Promise<BenchRunSummary> {
    return request(`/runs/${encodeURIComponent(id)}/cancel`, benchRunSummarySchema, {
      method: 'POST',
    });
  },
};
