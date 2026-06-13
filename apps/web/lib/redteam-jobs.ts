/**
 * Client contract for the durable red-team job API.
 *
 * The browser calls the same-origin authed routes under `/api/redteam/*`, which
 * proxy to the Rust orchestrator (`crates/tl-server/src/redteam`). Rust owns the
 * durable job + per-attack results; the dashboard only dispatches, polls, lists,
 * and cancels. Schemas mirror the Rust wire types in `crates/tl-core/src/redteam.rs`
 * (snake_case on the wire — the wire contract is the source of truth) and validate
 * every response at the boundary.
 */
import { z } from 'zod';

export const REDTEAM_JOB_PROFILES = ['fast', 'full', 'max'] as const;
export const redteamJobProfileSchema = z.enum(REDTEAM_JOB_PROFILES);
export type RedteamJobProfile = z.infer<typeof redteamJobProfileSchema>;

export const jobStatusSchema = z.enum(['queued', 'running', 'complete', 'error', 'cancelled']);
export type JobStatus = z.infer<typeof jobStatusSchema>;

export const redteamGeneratorSchema = z.enum(['deterministic', 'hackagent']);
export type RedteamGenerator = z.infer<typeof redteamGeneratorSchema>;

/** Terminal states stop polling. */
export function isTerminalStatus(status: JobStatus): boolean {
  return status === 'complete' || status === 'error' || status === 'cancelled';
}

export const redteamJobSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  environment_id: z.string(),
  status: jobStatusSchema,
  target: z.string(),
  // Dispatch validates profile ∈ {fast,full,max} server-side, so a persisted
  // job always carries one of those — constrain it here to catch backend drift.
  profile: redteamJobProfileSchema,
  generator: redteamGeneratorSchema,
  agent_id: z.string().nullish(),
  attacks: z.number(),
  landed: z.number(),
  blocked: z.number(),
  error: z.string().nullish(),
  created_at: z.string(),
  updated_at: z.string(),
});
export type RedteamJobSummary = z.infer<typeof redteamJobSummarySchema>;

export const redteamJobResultSchema = z.object({
  seq: z.number(),
  attack: z.string(),
  goal: z.string(),
  outcome: z.string(),
  landed: z.boolean(),
  prompt: z.string().nullish(),
  reply: z.string(),
  trace_id: z.string().nullish(),
});
export type RedteamJobResult = z.infer<typeof redteamJobResultSchema>;

export const redteamJobDetailSchema = z.object({
  job: redteamJobSummarySchema,
  results: z.array(redteamJobResultSchema),
});
export type RedteamJobDetail = z.infer<typeof redteamJobDetailSchema>;

export const redteamJobListResponseSchema = z.object({
  jobs: z.array(redteamJobSummarySchema),
});

export interface DispatchInput {
  targetUrl: string;
  profile: RedteamJobProfile;
  generator?: RedteamGenerator;
  agentId?: string;
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

/** Dispatch a job. Returns the persisted `Queued` summary with its id. */
export async function dispatchJob(input: DispatchInput): Promise<RedteamJobSummary> {
  // Translate the UI's camelCase shape to the Rust wire contract (snake_case).
  const body: {
    target_url: string;
    profile: RedteamJobProfile;
    generator?: RedteamGenerator;
    agent_id?: string;
  } = {
    target_url: input.targetUrl,
    profile: input.profile,
  };
  if (input.generator !== undefined) body.generator = input.generator;
  if (input.agentId !== undefined && input.agentId !== '') body.agent_id = input.agentId;

  const response = await fetch('/api/redteam/dispatch', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobSummarySchema.parse(json);
}

/** Fetch a job and its per-attack results. */
export async function getJob(id: string): Promise<RedteamJobDetail> {
  const response = await fetch(`/api/redteam/jobs/${encodeURIComponent(id)}`);
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobDetailSchema.parse(json);
}

/** List recent jobs in the workspace, newest first. */
export async function listJobs(params?: {
  agentId?: string;
  limit?: number;
}): Promise<RedteamJobSummary[]> {
  const query = new URLSearchParams();
  if (params?.agentId !== undefined && params.agentId !== '') query.set('agent_id', params.agentId);
  if (params?.limit !== undefined) query.set('limit', String(params.limit));
  const suffix = query.toString() === '' ? '' : `?${query.toString()}`;
  const response = await fetch(`/api/redteam/jobs${suffix}`);
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobListResponseSchema.parse(json).jobs;
}

/** Cooperatively cancel a job; returns the updated (or unchanged terminal) summary. */
export async function cancelJob(id: string): Promise<RedteamJobSummary> {
  const response = await fetch(`/api/redteam/jobs/${encodeURIComponent(id)}/cancel`, {
    method: 'POST',
  });
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobSummarySchema.parse(json);
}
