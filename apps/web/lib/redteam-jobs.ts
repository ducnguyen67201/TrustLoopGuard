/**
 * Same-origin client for the durable red-team job API.
 *
 * The browser calls the authed routes under `/api/redteam/*`, which proxy to the
 * Rust orchestrator (`crates/tl-server/src/redteam`); Rust owns the durable job +
 * per-attack results. The dashboard only dispatches, polls, lists, and cancels.
 *
 * The dashboard cannot use `@trustloopguard/sdk`'s `Client` here: that targets
 * Rust `/v1/*` directly with a bearer key (customer runtime), whereas the browser
 * authenticates by session through this same-origin proxy. So this is a thin
 * client over `/api/redteam/*` — but its TYPES are single-sourced from Rust via
 * the generated SDK bindings, and zod validates every response at the boundary.
 */
import type {
  JobStatus,
  RedteamGenerator,
  RedteamJobDetail,
  RedteamJobResult,
  RedteamJobSummary,
  RedteamReportShare,
} from '@trustloopguard/sdk';
import { z } from 'zod';

export type {
  JobStatus,
  RedteamGenerator,
  RedteamJobDetail,
  RedteamJobResult,
  RedteamJobSummary,
  RedteamReportShare,
};

// Web-only: the dashboard offers exactly these profiles. `profile` is a free
// String on the wire, so this enum is a UI constraint, not a generated wire type.
export const REDTEAM_JOB_PROFILES = ['fast', 'full', 'max'] as const;
export const redteamJobProfileSchema = z.enum(REDTEAM_JOB_PROFILES);
export type RedteamJobProfile = z.infer<typeof redteamJobProfileSchema>;

/** Terminal states stop polling. */
export function isTerminalStatus(status: JobStatus): boolean {
  return status === 'complete' || status === 'error' || status === 'cancelled';
}

/** Share of non-control attacks that landed, as a 0–100 integer percentage. */
export function landedPercent(job: RedteamJobSummary): number {
  return job.attacks > 0 ? Math.round((job.landed / job.attacks) * 100) : 0;
}

const jobStatusSchema = z.enum(['queued', 'running', 'complete', 'error', 'cancelled']);
export const redteamGeneratorSchema = z.enum(['deterministic', 'hackagent']);

export const redteamJobSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  environment_id: z.string(),
  status: jobStatusSchema,
  target: z.string(),
  // Dispatch validates profile ∈ {fast,full,max} server-side; constrain here too
  // to catch backend drift (narrower than the wire's String, still assignable).
  profile: redteamJobProfileSchema,
  generator: redteamGeneratorSchema,
  // Wire sends `null` (not an omitted key) for absent optionals — see redteam.rs.
  agent_id: z.string().nullable(),
  attacks: z.number(),
  landed: z.number(),
  blocked: z.number(),
  error: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

const redteamJobResultSchema = z.object({
  seq: z.number(),
  attack: z.string(),
  goal: z.string(),
  outcome: z.string(),
  landed: z.boolean(),
  prompt: z.string().nullable(),
  reply: z.string(),
  trace_id: z.string().nullable(),
});

export const redteamJobDetailSchema = z.object({
  job: redteamJobSummarySchema,
  results: z.array(redteamJobResultSchema),
});

const redteamJobListResponseSchema = z.object({
  jobs: z.array(redteamJobSummarySchema),
});

const redteamReportShareSchema = z.object({
  token: z.string(),
  path: z.string(),
  job_id: z.string(),
  compare_job_id: z.string().nullable(),
  created_at: z.string(),
  expires_at: z.string().nullable(),
});

export interface CreateReportInput {
  jobId: string;
  compareJobId?: string;
  ttlDays?: number;
}

interface DispatchInput {
  targetUrl: string;
  profile: RedteamJobProfile;
  generator?: RedteamGenerator;
  agentId?: string;
}

export interface ListJobsParams {
  agentId?: string;
  limit?: number;
}

const errorEnvelopeSchema = z.object({ error: z.string() });

function messageFromBody(body: unknown, status: number): string {
  const parsed = errorEnvelopeSchema.safeParse(body);
  if (parsed.success) return parsed.data.error;
  return `red-team request failed (HTTP ${status})`;
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

/** One same-origin call to `/api/redteam{path}`: fetch → surface errors → validate. */
async function request<S extends z.ZodTypeAny>(
  path: string,
  schema: S,
  init?: RequestInit,
): Promise<z.infer<S>> {
  const response = await fetch(`/api/redteam${path}`, init);
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return schema.parse(json);
}

/** Translate the UI's camelCase dispatch shape to the Rust wire contract (snake_case). */
function dispatchBody(input: DispatchInput): {
  target_url: string;
  profile: RedteamJobProfile;
  generator?: RedteamGenerator;
  agent_id?: string;
} {
  const body: {
    target_url: string;
    profile: RedteamJobProfile;
    generator?: RedteamGenerator;
    agent_id?: string;
  } = { target_url: input.targetUrl, profile: input.profile };
  if (input.generator !== undefined) body.generator = input.generator;
  if (input.agentId !== undefined && input.agentId !== '') body.agent_id = input.agentId;
  return body;
}

function jobsQuery(params?: ListJobsParams): string {
  const query = new URLSearchParams();
  if (params?.agentId !== undefined && params.agentId !== '') query.set('agent_id', params.agentId);
  if (params?.limit !== undefined) query.set('limit', String(params.limit));
  const serialized = query.toString();
  return serialized === '' ? '' : `?${serialized}`;
}

/** Translate the UI's camelCase report shape to the Rust wire contract (snake_case). */
function createReportBody(input: CreateReportInput): {
  job_id: string;
  compare_job_id?: string;
  ttl_days?: number;
} {
  const body: { job_id: string; compare_job_id?: string; ttl_days?: number } = {
    job_id: input.jobId,
  };
  if (input.compareJobId !== undefined && input.compareJobId !== '') {
    body.compare_job_id = input.compareJobId;
  }
  if (input.ttlDays !== undefined) body.ttl_days = input.ttlDays;
  return body;
}

/** Durable red-team job client (same-origin `/api/redteam/*` proxy to Rust). */
export const redteam = {
  /** Dispatch a job. Returns the persisted `Queued` summary with its id. */
  dispatch(input: DispatchInput): Promise<RedteamJobSummary> {
    return request('/dispatch', redteamJobSummarySchema, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(dispatchBody(input)),
    });
  },

  /** Fetch a job and its per-attack results. */
  getJob(id: string): Promise<RedteamJobDetail> {
    return request(`/jobs/${encodeURIComponent(id)}`, redteamJobDetailSchema);
  },

  /** List recent jobs in the workspace, newest first. */
  async listJobs(params?: ListJobsParams): Promise<RedteamJobSummary[]> {
    return (await request(`/jobs${jobsQuery(params)}`, redteamJobListResponseSchema)).jobs;
  },

  /** Cooperatively cancel a job; returns the updated (or unchanged terminal) summary. */
  cancel(id: string): Promise<RedteamJobSummary> {
    return request(`/jobs/${encodeURIComponent(id)}/cancel`, redteamJobSummarySchema, {
      method: 'POST',
    });
  },

  /** Mint a shareable report link for a completed job (optionally a same-agent comparison). */
  createReport(input: CreateReportInput): Promise<RedteamReportShare> {
    return request('/reports', redteamReportShareSchema, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(createReportBody(input)),
    });
  },

  /** Revoke a shareable report link. The public PDF then 404s. */
  async revokeReport(token: string): Promise<void> {
    const response = await fetch(`/api/redteam/reports/${encodeURIComponent(token)}/revoke`, {
      method: 'POST',
    });
    if (!response.ok) throw new Error(messageFromBody(await readJson(response), response.status));
  },
};
